use anyhow::{anyhow, Context, Result};
use serde_json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, info, warn};

use super::protocol::{QmpCommand, QmpGreeting, QmpResponse, SendKeyArgs};

/// QMP 客户端
pub struct QmpClient {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
}

impl QmpClient {
    /// 连接到 QMP Socket 并完成握手
    pub async fn connect(socket_path: &str) -> Result<Self> {
        info!("连接到 QMP Socket: {}", socket_path);

        // 建立 Unix Socket 连接
        let stream = UnixStream::connect(socket_path)
            .await
            .context("无法连接到 QMP Socket")?;

        let mut reader = BufReader::new(
            stream
                .try_clone()
                .await
                .context("无法克隆 UnixStream")?,
        );

        // 读取 QMP 问候信息
        let mut greeting_line = String::new();
        reader
            .read_line(&mut greeting_line)
            .await
            .context("读取 QMP 问候信息失败")?;

        let greeting: QmpGreeting =
            serde_json::from_str(&greeting_line).context("解析 QMP 问候信息失败")?;

        info!(
            "已连接到 QEMU {}.{}.{}",
            greeting.qmp.version.qemu.major,
            greeting.qmp.version.qemu.minor,
            greeting.qmp.version.qemu.micro
        );

        let mut client = Self { stream, reader };

        // 发送 qmp_capabilities 进入命令模式
        client.negotiate_capabilities().await?;

        Ok(client)
    }

    /// 协商 QMP 能力
    async fn negotiate_capabilities(&mut self) -> Result<()> {
        let cmd = QmpCommand {
            execute: "qmp_capabilities",
            arguments: None,
            id: Some("init"),
        };

        self.execute_command(&cmd).await?;
        info!("QMP 能力协商完成");
        Ok(())
    }

    /// 执行 QMP 命令
    async fn execute_command(&mut self, cmd: &QmpCommand<'_>) -> Result<QmpResponse> {
        // 序列化并发送命令
        let cmd_json = serde_json::to_string(cmd)?;
        debug!("发送 QMP 命令: {}", cmd_json);

        self.stream.write_all(cmd_json.as_bytes()).await?;
        self.stream.write_all(b"\n").await?;
        self.stream.flush().await?;

        // 读取响应
        let mut response_line = String::new();
        self.reader.read_line(&mut response_line).await?;

        let response: QmpResponse = serde_json::from_str(&response_line)
            .context("解析 QMP 响应失败")?;

        // 检查错误
        if let Some(error) = &response.error {
            return Err(anyhow!(
                "QMP 命令执行失败: {} - {}",
                error.error_class,
                error.desc
            ));
        }

        debug!("收到 QMP 响应: {:?}", response);
        Ok(response)
    }

    /// 发送按键序列
    pub async fn send_keys(&mut self, keys: Vec<&str>, hold_time: Option<u32>) -> Result<()> {
        use super::protocol::QmpKey;

        let qmp_keys: Vec<QmpKey> = keys
            .into_iter()
            .map(|k| QmpKey::new_qcode(k))
            .collect();

        let args = SendKeyArgs {
            keys: qmp_keys,
            hold_time,
        };

        let cmd = QmpCommand {
            execute: "send-key",
            arguments: Some(serde_json::to_value(args)?),
            id: Some("send-key"),
        };

        self.execute_command(&cmd).await?;
        Ok(())
    }

    /// 发送单个按键
    pub async fn send_key(&mut self, key: &str) -> Result<()> {
        self.send_keys(vec![key], None).await
    }

    /// 查询 QMP 版本
    pub async fn query_version(&mut self) -> Result<QmpResponse> {
        let cmd = QmpCommand {
            execute: "query-version",
            arguments: None,
            id: Some("query-version"),
        };

        self.execute_command(&cmd).await
    }

    /// 查询虚拟机状态
    pub async fn query_status(&mut self) -> Result<QmpResponse> {
        let cmd = QmpCommand {
            execute: "query-status",
            arguments: None,
            id: Some("query-status"),
        };

        self.execute_command(&cmd).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要实际的 QMP Socket 才能运行
    async fn test_qmp_connection() {
        let socket_path = "/var/lib/libvirt/qemu/domain-1-test/monitor.sock";
        let result = QmpClient::connect(socket_path).await;
        assert!(result.is_ok());
    }
}

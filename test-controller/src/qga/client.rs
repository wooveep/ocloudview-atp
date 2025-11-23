use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::time::Duration;
use tracing::{debug, info, warn};
use virt::domain::Domain;

use super::protocol::*;
use super::QgaError;

/// QEMU Guest Agent 客户端
pub struct QgaClient<'a> {
    domain: &'a Domain,
    timeout: i32, // 超时时间（秒）
}

impl<'a> QgaClient<'a> {
    /// 创建新的 QGA 客户端
    pub fn new(domain: &'a Domain) -> Self {
        Self {
            domain,
            timeout: 30, // 默认 30 秒超时
        }
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, timeout: i32) -> Self {
        self.timeout = timeout;
        self
    }

    /// 执行 QGA 命令的通用方法
    fn execute_command<T, R>(&self, command: &str, args: Option<T>) -> Result<R>
    where
        T: serde::Serialize,
        R: DeserializeOwned,
    {
        // 构造 QMP 命令
        let cmd = QgaCommand {
            execute: command,
            arguments: args,
        };

        let cmd_json = serde_json::to_string(&cmd)
            .context("序列化 QGA 命令失败")?;

        debug!("发送 QGA 命令: {}", cmd_json);

        // 通过 Libvirt 发送命令
        let response_json = self
            .domain
            .qemu_agent_command(&cmd_json, self.timeout, 0)
            .map_err(|e| QgaError::LibvirtError(e.to_string()))?;

        debug!("收到 QGA 响应: {}", response_json);

        // 解析响应
        let response: QgaResponse<R> = serde_json::from_str(&response_json)
            .map_err(|e| QgaError::ParseError(e.to_string()))?;

        // 检查错误
        if let Some(error) = response.error {
            return Err(QgaError::CommandFailed(format!(
                "{}: {}",
                error.error_class, error.desc
            ))
            .into());
        }

        // 返回结果
        response
            .ret
            .ok_or_else(|| QgaError::NoResponse.into())
    }

    /// 测试 QGA 连通性
    pub fn ping(&self) -> Result<()> {
        info!("测试 QGA 连通性");

        #[derive(Serialize)]
        struct Empty {}

        #[derive(Deserialize)]
        struct PingResponse {}

        self.execute_command::<Empty, PingResponse>("guest-ping", None)?;

        info!("QGA 连通性测试成功");
        Ok(())
    }

    /// 获取 Guest Agent 信息
    pub fn get_info(&self) -> Result<GuestInfo> {
        info!("获取 Guest Agent 信息");

        #[derive(Serialize)]
        struct Empty {}

        self.execute_command::<Empty, GuestInfo>("guest-info", None)
    }

    /// 获取操作系统信息
    pub fn get_osinfo(&self) -> Result<GuestOsInfo> {
        info!("获取操作系统信息");

        #[derive(Serialize)]
        struct Empty {}

        self.execute_command::<Empty, GuestOsInfo>("guest-get-osinfo", None)
    }

    /// 执行命令（异步启动）
    pub fn exec(&self, cmd: GuestExecCommand) -> Result<i64> {
        info!("执行 Guest 命令: {}", cmd.path);

        let result: GuestExecResult = self.execute_command("guest-exec", Some(cmd))?;

        info!("命令已启动，PID: {}", result.pid);
        Ok(result.pid)
    }

    /// 查询命令执行状态
    pub fn exec_status(&self, pid: i64) -> Result<GuestExecStatus> {
        debug!("查询进程状态: PID {}", pid);

        let request = GuestExecStatusRequest { pid };
        self.execute_command("guest-exec-status", Some(request))
    }

    /// 执行命令并等待完成
    pub fn exec_and_wait(&self, cmd: GuestExecCommand, poll_interval: Duration) -> Result<GuestExecStatus> {
        let pid = self.exec(cmd)?;

        info!("等待进程完成: PID {}", pid);

        // 轮询直到进程退出
        loop {
            let status = self.exec_status(pid)?;

            if status.exited {
                info!(
                    "进程已退出: PID {}, 退出码: {:?}",
                    pid, status.exit_code
                );
                return Ok(status);
            }

            std::thread::sleep(poll_interval);
        }
    }

    /// 执行 Shell 命令（便捷方法）
    pub fn exec_shell(&self, shell_cmd: &str) -> Result<GuestExecStatus> {
        info!("执行 Shell 命令: {}", shell_cmd);

        // 根据操作系统选择 Shell
        let os_info = self.get_osinfo().ok();

        let (shell, arg_flag) = if let Some(info) = os_info {
            if info.id.as_deref() == Some("windows") || info.name.as_ref().map_or(false, |n| n.contains("Windows")) {
                ("C:\\Windows\\System32\\cmd.exe", "/c")
            } else {
                ("/bin/sh", "-c")
            }
        } else {
            // 默认使用 Linux Shell
            ("/bin/sh", "-c")
        };

        let cmd = GuestExecCommand::simple(
            shell,
            vec![arg_flag.to_string(), shell_cmd.to_string()],
        );

        self.exec_and_wait(cmd, Duration::from_millis(500))
    }

    /// 打开文件
    pub fn file_open(&self, path: &str, mode: Option<&str>) -> Result<i64> {
        info!("打开文件: {} (模式: {:?})", path, mode);

        let args = GuestFileOpen {
            path: path.to_string(),
            mode: mode.map(|s| s.to_string()),
        };

        let result: GuestFileHandle = self.execute_command("guest-file-open", Some(args))?;

        info!("文件已打开，句柄: {}", result.handle);
        Ok(result.handle)
    }

    /// 读取文件
    pub fn file_read(&self, handle: i64, count: Option<i64>) -> Result<GuestFileReadResult> {
        debug!("读取文件: 句柄 {}", handle);

        let args = GuestFileRead { handle, count };

        self.execute_command("guest-file-read", Some(args))
    }

    /// 写入文件
    pub fn file_write(&self, handle: i64, data: &[u8]) -> Result<i64> {
        use base64::{Engine as _, engine::general_purpose};

        debug!("写入文件: 句柄 {}, {} 字节", handle, data.len());

        let buf_b64 = general_purpose::STANDARD.encode(data);

        let args = GuestFileWrite { handle, buf_b64 };

        let result: GuestFileWriteResult =
            self.execute_command("guest-file-write", Some(args))?;

        Ok(result.count)
    }

    /// 关闭文件
    pub fn file_close(&self, handle: i64) -> Result<()> {
        debug!("关闭文件: 句柄 {}", handle);

        let args = GuestFileClose { handle };

        #[derive(Deserialize)]
        struct EmptyResponse {}

        self.execute_command::<_, EmptyResponse>("guest-file-close", Some(args))?;

        Ok(())
    }

    /// 读取整个文件内容（便捷方法）
    pub fn read_file(&self, path: &str) -> Result<String> {
        info!("读取文件内容: {}", path);

        let handle = self.file_open(path, Some("r"))?;

        let mut content = Vec::new();
        loop {
            let result = self.file_read(handle, Some(4096))?;

            if let Ok(decoded) = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &result.buf_b64,
            ) {
                content.extend_from_slice(&decoded);
            }

            if result.eof {
                break;
            }
        }

        self.file_close(handle)?;

        String::from_utf8(content).context("文件内容不是有效的 UTF-8")
    }

    /// 写入整个文件内容（便捷方法）
    pub fn write_file(&self, path: &str, content: &str) -> Result<()> {
        info!("写入文件内容: {} ({} 字节)", path, content.len());

        let handle = self.file_open(path, Some("w"))?;
        self.file_write(handle, content.as_bytes())?;
        self.file_close(handle)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // 需要实际的 Guest Agent
    fn test_qga_ping() {
        // 此测试需要实际的 Libvirt 环境和运行的 VM
    }
}

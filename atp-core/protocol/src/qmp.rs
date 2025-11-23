//! QMP (QEMU Machine Protocol) 协议实现
//!
//! QMP 是 QEMU 的机器协议，用于管理和监控虚拟机。
//! 通过 Unix Socket 连接到 QEMU 的 monitor 接口。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tracing::{debug, info};
use virt::domain::Domain;

use crate::{Protocol, ProtocolBuilder, ProtocolError, ProtocolType, Result};

// ============================================================================
// QMP 协议数据结构
// ============================================================================

/// QMP 命令结构定义
#[derive(Debug, Serialize)]
pub struct QmpCommand<'a> {
    pub execute: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<&'a str>,
}

/// QMP 响应结构定义
#[derive(Debug, Deserialize)]
pub struct QmpResponse {
    #[serde(rename = "return")]
    pub ret: Option<serde_json::Value>,
    pub error: Option<QmpError>,
    pub event: Option<String>,
}

/// QMP 错误信息
#[derive(Debug, Deserialize)]
pub struct QmpError {
    #[serde(rename = "class")]
    pub error_class: String,
    pub desc: String,
}

/// QMP 问候信息 (Greeting)
#[derive(Debug, Deserialize)]
pub struct QmpGreeting {
    #[serde(rename = "QMP")]
    pub qmp: QmpInfo,
}

#[derive(Debug, Deserialize)]
pub struct QmpInfo {
    pub version: QmpVersion,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct QmpVersion {
    pub qemu: QemuVersion,
    pub package: String,
}

#[derive(Debug, Deserialize)]
pub struct QemuVersion {
    pub major: u32,
    pub minor: u32,
    pub micro: u32,
}

/// QMP 按键定义
#[derive(Debug, Serialize)]
pub struct QmpKey {
    #[serde(rename = "type")]
    pub key_type: String, // "qcode"
    pub data: String,     // QKeyCode 字符串，如 "a", "shift", "ret"
}

impl QmpKey {
    pub fn new_qcode(qcode: &str) -> Self {
        Self {
            key_type: "qcode".to_string(),
            data: qcode.to_string(),
        }
    }
}

/// send-key 命令参数
#[derive(Debug, Serialize)]
pub struct SendKeyArgs {
    pub keys: Vec<QmpKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "hold-time")]
    pub hold_time: Option<u32>, // 单位：毫秒
}

// ============================================================================
// QMP 协议实现
// ============================================================================

/// QMP 协议实现
pub struct QmpProtocol {
    /// 写入端（用Arc<Mutex>包装以支持可变访问）
    writer: Option<Arc<Mutex<WriteHalf<UnixStream>>>>,
    /// 读取端（用Arc<Mutex>包装以支持可变访问）
    reader: Option<Arc<Mutex<BufReader<ReadHalf<UnixStream>>>>>,
    /// QMP Socket 路径
    socket_path: Option<String>,
    /// 连接状态
    connected: bool,
}

impl QmpProtocol {
    pub fn new() -> Self {
        Self {
            writer: None,
            reader: None,
            socket_path: None,
            connected: false,
        }
    }

    /// 执行 QMP 命令
    pub async fn execute_command(&mut self, cmd: &QmpCommand<'_>) -> Result<QmpResponse> {
        if !self.connected {
            return Err(ProtocolError::ConnectionFailed(
                "QMP 未连接".to_string(),
            ));
        }

        let writer = self
            .writer
            .as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("Writer 不可用".to_string()))?;

        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("Reader 不可用".to_string()))?;

        // 序列化并发送命令
        let cmd_json = serde_json::to_string(cmd)
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;

        debug!("发送 QMP 命令: {}", cmd_json);

        let mut writer_guard = writer.lock().await;
        writer_guard
            .write_all(cmd_json.as_bytes())
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        writer_guard
            .write_all(b"\n")
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        writer_guard
            .flush()
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        drop(writer_guard);

        // 读取响应
        let mut reader_guard = reader.lock().await;
        let mut response_line = String::new();
        reader_guard
            .read_line(&mut response_line)
            .await
            .map_err(|e| ProtocolError::ReceiveFailed(e.to_string()))?;
        drop(reader_guard);

        let response: QmpResponse = serde_json::from_str(&response_line)
            .map_err(|e| ProtocolError::ParseError(e.to_string()))?;

        // 检查错误
        if let Some(error) = &response.error {
            return Err(ProtocolError::CommandFailed(format!(
                "{}: {}",
                error.error_class, error.desc
            )));
        }

        debug!("收到 QMP 响应: {:?}", response);
        Ok(response)
    }

    /// 发送按键序列
    pub async fn send_keys(&mut self, keys: Vec<&str>, hold_time: Option<u32>) -> Result<()> {
        let qmp_keys: Vec<QmpKey> = keys.into_iter().map(|k| QmpKey::new_qcode(k)).collect();

        let args = SendKeyArgs {
            keys: qmp_keys,
            hold_time,
        };

        let cmd = QmpCommand {
            execute: "send-key",
            arguments: Some(serde_json::to_value(args).map_err(|e| {
                ProtocolError::SendFailed(format!("序列化参数失败: {}", e))
            })?),
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
}

impl Default for QmpProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Protocol for QmpProtocol {
    async fn connect(&mut self, domain: &Domain) -> Result<()> {
        // 从 Domain 获取 QMP Socket 路径
        // 通常位于 /var/lib/libvirt/qemu/domain-{id}-{name}/monitor.sock
        let domain_name = domain
            .get_name()
            .map_err(|e| ProtocolError::ConnectionFailed(e.to_string()))?;

        // 尝试构建 socket 路径
        // 注意：这是一个简化的实现，实际可能需要从 libvirt XML 中读取
        let socket_path = format!("/var/lib/libvirt/qemu/domain-*-{}/monitor.sock", domain_name);

        info!("连接到 QMP Socket: {}", socket_path);

        // 由于路径包含通配符，我们需要展开它
        // 这里简化处理，实际应该使用 glob 或从 XML 读取
        // 为了演示，我们假设路径是已知的或通过其他方式获取

        // TODO: 实现真实的 socket 路径解析

        // 暂时存储 socket 路径用于后续重连
        self.socket_path = Some(socket_path.clone());

        // 建立 Unix Socket 连接
        let stream = UnixStream::connect(&socket_path)
            .await
            .map_err(|e| {
                ProtocolError::ConnectionFailed(format!("无法连接到 QMP Socket: {}", e))
            })?;

        // 分离读写端
        let (read_half, write_half) = tokio::io::split(stream);
        let mut reader = BufReader::new(read_half);

        // 读取 QMP 问候信息
        let mut greeting_line = String::new();
        reader
            .read_line(&mut greeting_line)
            .await
            .map_err(|e| ProtocolError::ConnectionFailed(format!("读取问候信息失败: {}", e)))?;

        let greeting: QmpGreeting = serde_json::from_str(&greeting_line)
            .map_err(|e| ProtocolError::ConnectionFailed(format!("解析问候信息失败: {}", e)))?;

        info!(
            "已连接到 QEMU {}.{}.{}",
            greeting.qmp.version.qemu.major,
            greeting.qmp.version.qemu.minor,
            greeting.qmp.version.qemu.micro
        );

        self.writer = Some(Arc::new(Mutex::new(write_half)));
        self.reader = Some(Arc::new(Mutex::new(reader)));
        self.connected = true;

        // 发送 qmp_capabilities 进入命令模式
        self.negotiate_capabilities().await?;

        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(ProtocolError::ConnectionFailed(
                "QMP 未连接".to_string(),
            ));
        }

        let writer = self
            .writer
            .as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("Writer 不可用".to_string()))?;

        let mut writer_guard = writer.lock().await;
        writer_guard
            .write_all(data)
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        writer_guard
            .write_all(b"\n")
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        writer_guard
            .flush()
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;

        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        if !self.connected {
            return Err(ProtocolError::ConnectionFailed(
                "QMP 未连接".to_string(),
            ));
        }

        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("Reader 不可用".to_string()))?;

        let mut reader_guard = reader.lock().await;
        let mut line = String::new();
        reader_guard
            .read_line(&mut line)
            .await
            .map_err(|e| ProtocolError::ReceiveFailed(e.to_string()))?;

        Ok(line.into_bytes())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.writer = None;
        self.reader = None;
        self.connected = false;
        self.socket_path = None;

        info!("QMP 连接已断开");
        Ok(())
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::QMP
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }
}

// ============================================================================
// QMP 协议构建器
// ============================================================================

/// QMP 协议构建器
pub struct QmpProtocolBuilder;

impl QmpProtocolBuilder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QmpProtocolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolBuilder for QmpProtocolBuilder {
    fn build(&self) -> Box<dyn Protocol> {
        Box::new(QmpProtocol::new())
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::QMP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qmp_key_creation() {
        let key = QmpKey::new_qcode("a");
        assert_eq!(key.key_type, "qcode");
        assert_eq!(key.data, "a");
    }

    #[test]
    fn test_send_key_args_serialization() {
        let args = SendKeyArgs {
            keys: vec![QmpKey::new_qcode("a"), QmpKey::new_qcode("b")],
            hold_time: Some(100),
        };

        let json = serde_json::to_string(&args).unwrap();
        assert!(json.contains("\"hold-time\":100"));
    }
}

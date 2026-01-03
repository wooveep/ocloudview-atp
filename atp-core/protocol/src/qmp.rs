//! QMP (QEMU Machine Protocol) 协议实现
//!
//! QMP 是 QEMU 的机器协议，用于管理和监控虚拟机。
//! 通过 libvirt 的 `qemu_monitor_command` API 进行通信，支持远程连接。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

/// QMP 按键定义
#[derive(Debug, Serialize)]
pub struct QmpKey {
    #[serde(rename = "type")]
    pub key_type: String, // "qcode"
    pub data: String, // QKeyCode 字符串，如 "a", "shift", "ret"
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
///
/// 通过 libvirt 的 `qemu_monitor_command` API 发送 QMP 命令，
/// 支持本地和远程 libvirt 连接。
pub struct QmpProtocol {
    /// Domain 引用（用 Arc<Mutex> 包装）
    domain: Option<Arc<Mutex<Domain>>>,
    /// 连接状态
    connected: bool,
}

impl QmpProtocol {
    pub fn new() -> Self {
        Self {
            domain: None,
            connected: false,
        }
    }

    /// 执行 QMP 命令
    pub async fn execute_command(&self, cmd: &QmpCommand<'_>) -> Result<QmpResponse> {
        if !self.connected {
            return Err(ProtocolError::ConnectionFailed("QMP 未连接".to_string()));
        }

        let domain_guard = self
            .domain
            .as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("Domain 不可用".to_string()))?;

        // 序列化命令
        let cmd_json =
            serde_json::to_string(cmd).map_err(|e| ProtocolError::SendFailed(e.to_string()))?;

        debug!("发送 QMP 命令 (via libvirt): {}", cmd_json);

        // 通过 libvirt 发送命令（在阻塞任务中执行）
        let domain_guard = domain_guard.lock().await;
        let domain_clone = domain_guard.clone();
        drop(domain_guard);

        let response_json = tokio::task::spawn_blocking(move || {
            // flags = 0 表示使用默认行为
            domain_clone.qemu_monitor_command(&cmd_json, 0)
        })
        .await
        .map_err(|e| ProtocolError::CommandFailed(format!("任务执行失败: {}", e)))?
        .map_err(|e| ProtocolError::CommandFailed(format!("QMP 命令失败: {}", e)))?;

        debug!("收到 QMP 响应: {}", response_json);

        // 解析响应
        let response: QmpResponse = serde_json::from_str(&response_json)
            .map_err(|e| ProtocolError::ParseError(format!("解析响应失败: {}", e)))?;

        // 检查错误
        if let Some(error) = &response.error {
            return Err(ProtocolError::CommandFailed(format!(
                "{}: {}",
                error.error_class, error.desc
            )));
        }

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
            arguments: Some(
                serde_json::to_value(args)
                    .map_err(|e| ProtocolError::SendFailed(format!("序列化参数失败: {}", e)))?,
            ),
            id: None, // libvirt passthrough 不支持 id 字段
        };

        self.execute_command(&cmd).await?;
        Ok(())
    }

    /// 发送单个按键
    pub async fn send_key(&mut self, key: &str) -> Result<()> {
        self.send_keys(vec![key], None).await
    }

    /// 查询 QMP 版本
    pub async fn query_version(&self) -> Result<QmpResponse> {
        let cmd = QmpCommand {
            execute: "query-version",
            arguments: None,
            id: None, // libvirt passthrough 不支持 id 字段
        };

        self.execute_command(&cmd).await
    }

    /// 查询虚拟机状态
    pub async fn query_status(&self) -> Result<QmpResponse> {
        let cmd = QmpCommand {
            execute: "query-status",
            arguments: None,
            id: None, // libvirt passthrough 不支持 id 字段
        };

        self.execute_command(&cmd).await
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
        info!("通过 libvirt passthrough 连接 QMP");

        // 克隆 domain（libvirt Domain 支持 clone）
        let domain_clone = domain.clone();

        // 验证 domain 支持 QMP - 发送 query-status 测试
        // 注意：libvirt qemu_monitor_command 不支持 id 字段
        let test_cmd = QmpCommand {
            execute: "query-status",
            arguments: None,
            id: None, // libvirt passthrough 不支持 id 字段
        };

        let cmd_json = serde_json::to_string(&test_cmd)
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;

        // 同步调用测试
        let test_domain = domain.clone();
        let result =
            tokio::task::spawn_blocking(move || test_domain.qemu_monitor_command(&cmd_json, 0))
                .await
                .map_err(|e| ProtocolError::ConnectionFailed(format!("QMP 测试任务失败: {}", e)))?;

        match result {
            Ok(response) => {
                debug!("QMP 测试响应: {}", response);
                info!("QMP (via libvirt) 连接成功");
            }
            Err(e) => {
                return Err(ProtocolError::ConnectionFailed(format!(
                    "QMP 不可用: {} (确保虚拟机正在运行且支持 QMP)",
                    e
                )));
            }
        }

        self.domain = Some(Arc::new(Mutex::new(domain_clone)));
        self.connected = true;

        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(ProtocolError::ConnectionFailed("QMP 未连接".to_string()));
        }

        let domain_guard = self
            .domain
            .as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("Domain 不可用".to_string()))?;

        let cmd_json = String::from_utf8(data.to_vec())
            .map_err(|e| ProtocolError::SendFailed(format!("数据不是有效的 UTF-8: {}", e)))?;

        let domain_guard = domain_guard.lock().await;
        let domain_clone = domain_guard.clone();
        drop(domain_guard);

        tokio::task::spawn_blocking(move || domain_clone.qemu_monitor_command(&cmd_json, 0))
            .await
            .map_err(|e| ProtocolError::SendFailed(format!("任务执行失败: {}", e)))?
            .map_err(|e| ProtocolError::SendFailed(format!("发送失败: {}", e)))?;

        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        // QMP 通过 libvirt 是请求-响应模式，不支持单独的 receive
        // 这里返回错误，实际的接收在 execute_command 中处理
        Err(ProtocolError::CommandFailed(
            "QMP (via libvirt) 不支持独立的 receive 操作，请使用 execute_command".to_string(),
        ))
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.domain = None;
        self.connected = false;
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

    #[test]
    fn test_qmp_protocol_creation() {
        let protocol = QmpProtocol::new();
        assert!(!protocol.connected);
        assert!(protocol.domain.is_none());
    }
}

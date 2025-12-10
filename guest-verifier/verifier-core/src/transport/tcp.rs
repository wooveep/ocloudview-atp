//! TCP 传输实现

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, info};

use crate::{Event, Result, VerifierError, VerifyResult};
use super::VerifierTransport;

/// TCP 传输实现
pub struct TcpTransport {
    stream: Option<TcpStream>,
    endpoint: Option<String>,
}

impl TcpTransport {
    /// 创建新的 TCP 传输
    pub fn new() -> Self {
        Self {
            stream: None,
            endpoint: None,
        }
    }

    /// 检查连接是否存在
    fn ensure_connected(&self) -> Result<()> {
        if self.stream.is_none() {
            return Err(VerifierError::ConnectionFailed(
                "未连接到服务器".to_string(),
            ));
        }
        Ok(())
    }

    /// 发送 JSON 消息（长度前缀格式）
    async fn send_json(&mut self, json: &str) -> Result<()> {
        if let Some(stream) = &mut self.stream {
            // 发送长度（4字节，大端序）
            let len = json.len() as u32;
            stream.write_u32(len).await.map_err(|e| {
                error!("发送消息长度失败: {}", e);
                VerifierError::IoError(e)
            })?;

            // 发送 JSON 数据
            stream.write_all(json.as_bytes()).await.map_err(|e| {
                error!("发送消息内容失败: {}", e);
                VerifierError::IoError(e)
            })?;

            stream.flush().await.map_err(|e| {
                error!("刷新输出缓冲失败: {}", e);
                VerifierError::IoError(e)
            })?;

            Ok(())
        } else {
            Err(VerifierError::ConnectionFailed("未连接".to_string()))
        }
    }

    /// 接收 JSON 消息（长度前缀格式）
    async fn receive_json(&mut self) -> Result<String> {
        if let Some(stream) = &mut self.stream {
            // 读取长度（4字节，大端序）
            let len = stream.read_u32().await.map_err(|e| {
                error!("读取消息长度失败: {}", e);
                VerifierError::IoError(e)
            })?;

            // 限制消息大小（最大 10MB）
            const MAX_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;
            if len > MAX_MESSAGE_SIZE {
                return Err(VerifierError::ConnectionFailed(format!(
                    "消息过大: {} bytes (最大: {} bytes)",
                    len, MAX_MESSAGE_SIZE
                )));
            }

            // 读取 JSON 数据
            let mut buffer = vec![0u8; len as usize];
            stream.read_exact(&mut buffer).await.map_err(|e| {
                error!("读取消息内容失败: {}", e);
                VerifierError::IoError(e)
            })?;

            let json = String::from_utf8(buffer).map_err(|e| {
                error!("解码 UTF-8 失败: {}", e);
                VerifierError::ConnectionFailed(format!("UTF-8 解码失败: {}", e))
            })?;

            Ok(json)
        } else {
            Err(VerifierError::ConnectionFailed("未连接".to_string()))
        }
    }
}

impl Default for TcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VerifierTransport for TcpTransport {
    async fn connect(&mut self, endpoint: &str, vm_id: Option<&str>) -> Result<()> {
        info!("连接到 TCP 服务器: {}", endpoint);

        match TcpStream::connect(endpoint).await {
            Ok(mut stream) => {
                info!("成功连接到 TCP 服务器");

                // 发送 VM ID（如果提供，使用长度前缀格式）
                if let Some(vm_id) = vm_id {
                    debug!("发送 VM ID: {}", vm_id);
                    let vm_id_bytes = vm_id.as_bytes();
                    stream.write_u32(vm_id_bytes.len() as u32).await.map_err(|e| {
                        error!("发送 VM ID 长度失败: {}", e);
                        VerifierError::IoError(e)
                    })?;
                    stream.write_all(vm_id_bytes).await.map_err(|e| {
                        error!("发送 VM ID 失败: {}", e);
                        VerifierError::IoError(e)
                    })?;
                    stream.flush().await.map_err(|e| {
                        error!("刷新失败: {}", e);
                        VerifierError::IoError(e)
                    })?;
                }

                self.stream = Some(stream);
                self.endpoint = Some(endpoint.to_string());
                Ok(())
            }
            Err(e) => {
                error!("TCP 连接失败: {}", e);
                Err(VerifierError::IoError(e))
            }
        }
    }

    async fn send_result(&mut self, result: &VerifyResult) -> Result<()> {
        self.ensure_connected()?;

        let json = serde_json::to_string(result).map_err(|e| {
            VerifierError::ConnectionFailed(format!("序列化验证结果失败: {}", e))
        })?;

        debug!("发送验证结果: {}", json);
        self.send_json(&json).await
    }

    async fn receive_event(&mut self) -> Result<Event> {
        self.ensure_connected()?;

        let json = self.receive_json().await?;
        debug!("接收到事件: {}", json);

        let event: Event = serde_json::from_str(&json).map_err(|e| {
            error!("解析事件失败: {}", e);
            VerifierError::ConnectionFailed(format!("解析事件失败: {}", e))
        })?;

        Ok(event)
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            info!("关闭 TCP 连接");
            stream.shutdown().await.map_err(|e| {
                error!("关闭 TCP 连接失败: {}", e);
                VerifierError::IoError(e)
            })?;
        }
        self.endpoint = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_transport_creation() {
        let transport = TcpTransport::new();
        assert!(transport.stream.is_none());
        assert!(transport.endpoint.is_none());
    }

    #[test]
    fn test_tcp_transport_default() {
        let transport = TcpTransport::default();
        assert!(transport.stream.is_none());
    }
}

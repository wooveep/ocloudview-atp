//! WebSocket 传输实现

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, protocol::CloseFrame},
    MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info};

use crate::{Event, RawInputEvent, VerifiedInputEvent, Result, VerifierError, VerifyResult};
use super::VerifierTransport;

/// WebSocket 传输实现
pub struct WebSocketTransport {
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    endpoint: Option<String>,
}

impl WebSocketTransport {
    /// 创建新的 WebSocket 传输
    pub fn new() -> Self {
        Self {
            ws_stream: None,
            endpoint: None,
        }
    }

    /// 检查连接是否存在
    fn ensure_connected(&self) -> Result<()> {
        if self.ws_stream.is_none() {
            return Err(VerifierError::ConnectionFailed(
                "未连接到服务器".to_string(),
            ));
        }
        Ok(())
    }

    /// 发送 JSON 消息的通用方法
    async fn send_json_msg(&mut self, json: String, msg_type: &str) -> Result<()> {
        debug!("发送{}: {}", msg_type, json);

        if let Some(ws_stream) = &mut self.ws_stream {
            ws_stream
                .send(Message::Text(json))
                .await
                .map_err(|e| {
                    error!("发送{}失败: {}", msg_type, e);
                    VerifierError::ConnectionFailed(format!("发送失败: {}", e))
                })?;
        }

        Ok(())
    }
}

impl Default for WebSocketTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VerifierTransport for WebSocketTransport {
    async fn connect(&mut self, endpoint: &str, vm_id: Option<&str>) -> Result<()> {
        info!("连接到 WebSocket 服务器: {}", endpoint);

        // 如果 endpoint 不包含协议，添加 ws:// 前缀
        let url = if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
            endpoint.to_string()
        } else {
            format!("ws://{}", endpoint)
        };

        match connect_async(&url).await {
            Ok((mut ws_stream, _)) => {
                info!("成功连接到 WebSocket 服务器");

                // 发送 VM ID（如果提供）
                if let Some(vm_id) = vm_id {
                    debug!("发送 VM ID: {}", vm_id);
                    ws_stream
                        .send(Message::Text(vm_id.to_string()))
                        .await
                        .map_err(|e| {
                            error!("发送 VM ID 失败: {}", e);
                            VerifierError::ConnectionFailed(format!("发送 VM ID 失败: {}", e))
                        })?;
                }

                self.ws_stream = Some(ws_stream);
                self.endpoint = Some(endpoint.to_string());
                Ok(())
            }
            Err(e) => {
                error!("WebSocket 连接失败: {}", e);
                Err(VerifierError::ConnectionFailed(format!(
                    "WebSocket 连接失败: {}",
                    e
                )))
            }
        }
    }

    async fn send_result(&mut self, result: &VerifyResult) -> Result<()> {
        self.ensure_connected()?;
        let json = serde_json::to_string(result).map_err(|e| {
            VerifierError::ConnectionFailed(format!("序列化验证结果失败: {}", e))
        })?;
        self.send_json_msg(json, "验证结果").await
    }

    async fn send_input_event(&mut self, event: &VerifiedInputEvent) -> Result<()> {
        self.ensure_connected()?;
        let json = serde_json::to_string(event).map_err(|e| {
            VerifierError::ConnectionFailed(format!("序列化输入事件失败: {}", e))
        })?;
        self.send_json_msg(json, "输入事件").await
    }

    async fn send_raw_input_event(&mut self, event: &RawInputEvent) -> Result<()> {
        self.ensure_connected()?;
        let json = serde_json::to_string(event).map_err(|e| {
            VerifierError::ConnectionFailed(format!("序列化原始输入事件失败: {}", e))
        })?;
        self.send_json_msg(json, "原始输入事件").await
    }

    async fn receive_event(&mut self) -> Result<Event> {
        self.ensure_connected()?;

        if let Some(ws_stream) = &mut self.ws_stream {
            loop {
                match ws_stream.next().await {
                    Some(Ok(msg)) => match msg {
                        Message::Text(text) => {
                            debug!("接收到事件: {}", text);
                            let event: Event = serde_json::from_str(&text).map_err(|e| {
                                error!("解析事件失败: {}", e);
                                VerifierError::ConnectionFailed(format!("解析事件失败: {}", e))
                            })?;
                            return Ok(event);
                        }
                        Message::Binary(data) => {
                            debug!("接收到二进制事件: {} bytes", data.len());
                            let event: Event = serde_json::from_slice(&data).map_err(|e| {
                                error!("解析二进制事件失败: {}", e);
                                VerifierError::ConnectionFailed(format!("解析事件失败: {}", e))
                            })?;
                            return Ok(event);
                        }
                        Message::Ping(_) | Message::Pong(_) => {
                            // 忽略心跳消息，继续接收下一条
                            continue;
                        }
                        Message::Close(frame) => {
                            let reason = frame
                                .as_ref()
                                .map(|f| f.reason.to_string())
                                .unwrap_or_else(|| "未知原因".to_string());
                            error!("WebSocket 连接已关闭: {}", reason);
                            self.ws_stream = None;
                            return Err(VerifierError::ConnectionFailed(format!(
                                "连接已关闭: {}",
                                reason
                            )));
                        }
                        Message::Frame(_) => {
                            // 底层帧，通常不会收到
                            continue;
                        }
                    },
                    Some(Err(e)) => {
                        error!("接收消息失败: {}", e);
                        self.ws_stream = None;
                        return Err(VerifierError::ConnectionFailed(format!(
                            "接收失败: {}",
                            e
                        )));
                    }
                    None => {
                        error!("WebSocket 流已关闭");
                        self.ws_stream = None;
                        return Err(VerifierError::ConnectionFailed(
                            "连接已断开".to_string(),
                        ));
                    }
                }
            }
        }

        Err(VerifierError::ConnectionFailed("未连接".to_string()))
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut ws_stream) = self.ws_stream.take() {
            info!("关闭 WebSocket 连接");
            ws_stream
                .close(Some(CloseFrame {
                    code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                    reason: "正常关闭".into(),
                }))
                .await
                .map_err(|e| {
                    error!("关闭 WebSocket 连接失败: {}", e);
                    VerifierError::ConnectionFailed(format!("关闭失败: {}", e))
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
    fn test_websocket_transport_creation() {
        let transport = WebSocketTransport::new();
        assert!(transport.ws_stream.is_none());
        assert!(transport.endpoint.is_none());
    }

    #[test]
    fn test_websocket_transport_default() {
        let transport = WebSocketTransport::default();
        assert!(transport.ws_stream.is_none());
    }
}

//! VirtioSerial 协议实现
//!
//! 提供可扩展的协议处理机制

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;
use virt::domain::Domain;
use tracing::{debug, info};

use crate::{Protocol, ProtocolType, ProtocolBuilder, ProtocolError};
use super::channel::{VirtioChannel, ChannelInfo};

/// 协议处理器 trait
///
/// 允许用户自定义协议格式和处理逻辑
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// 编码请求数据
    async fn encode_request(&self, data: &[u8]) -> Result<Vec<u8>, ProtocolError>;

    /// 解码响应数据
    async fn decode_response(&self, data: &[u8]) -> Result<Vec<u8>, ProtocolError>;

    /// 获取处理器名称
    fn name(&self) -> &str;
}

/// 原始协议处理器（不做任何处理）
pub struct RawProtocolHandler;

#[async_trait]
impl ProtocolHandler for RawProtocolHandler {
    async fn encode_request(&self, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        Ok(data.to_vec())
    }

    async fn decode_response(&self, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        Ok(data.to_vec())
    }

    fn name(&self) -> &str {
        "raw"
    }
}

/// JSON 协议处理器
///
/// 将数据包装为 JSON 格式
#[derive(Debug, Clone)]
pub struct JsonProtocolHandler {
    /// 请求包装字段
    pub request_field: String,
    /// 响应包装字段
    pub response_field: String,
}

impl JsonProtocolHandler {
    pub fn new(request_field: &str, response_field: &str) -> Self {
        Self {
            request_field: request_field.to_string(),
            response_field: response_field.to_string(),
        }
    }
}

impl Default for JsonProtocolHandler {
    fn default() -> Self {
        Self {
            request_field: "data".to_string(),
            response_field: "result".to_string(),
        }
    }
}

#[async_trait]
impl ProtocolHandler for JsonProtocolHandler {
    async fn encode_request(&self, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let text = String::from_utf8_lossy(data);

        let mut map = serde_json::Map::new();
        map.insert(self.request_field.clone(), JsonValue::String(text.to_string()));
        let json = JsonValue::Object(map);

        let mut encoded = serde_json::to_vec(&json)
            .map_err(|e| ProtocolError::ParseError(e.to_string()))?;

        // 添加换行符作为消息分隔符
        encoded.push(b'\n');

        Ok(encoded)
    }

    async fn decode_response(&self, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let json: JsonValue = serde_json::from_slice(data)
            .map_err(|e| ProtocolError::ParseError(e.to_string()))?;

        let result = json.get(&self.response_field)
            .ok_or_else(|| ProtocolError::ParseError(
                format!("响应中缺少字段: {}", self.response_field)
            ))?;

        let text = result.as_str()
            .ok_or_else(|| ProtocolError::ParseError("响应字段不是字符串".to_string()))?;

        Ok(text.as_bytes().to_vec())
    }

    fn name(&self) -> &str {
        "json"
    }
}

/// VirtioSerial 协议
pub struct VirtioSerialProtocol {
    /// 通道
    channel: VirtioChannel,

    /// 协议处理器
    handler: Arc<dyn ProtocolHandler>,

    /// 是否已连接
    connected: bool,
}

impl VirtioSerialProtocol {
    /// 创建新的协议实例
    pub fn new(channel: VirtioChannel, handler: Arc<dyn ProtocolHandler>) -> Self {
        Self {
            channel,
            handler,
            connected: false,
        }
    }

    /// 发送数据
    pub async fn send_data(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        debug!("发送数据: {} 字节", data.len());

        // 使用处理器编码请求
        let encoded = self.handler.encode_request(data).await?;

        // 发送到通道
        self.channel.send_raw(&encoded).await?;

        Ok(())
    }

    /// 接收数据
    pub async fn receive_data(&mut self) -> Result<Vec<u8>, ProtocolError> {
        debug!("接收数据");

        // 从通道接收
        let mut buffer = vec![0u8; 4096];
        let n = self.channel.receive_raw(&mut buffer).await?;
        buffer.truncate(n);

        // 使用处理器解码响应
        let decoded = self.handler.decode_response(&buffer).await?;

        Ok(decoded)
    }

    /// 发送字符串
    pub async fn send_string(&mut self, text: &str) -> Result<(), ProtocolError> {
        self.send_data(text.as_bytes()).await
    }

    /// 接收字符串
    pub async fn receive_string(&mut self) -> Result<String, ProtocolError> {
        let data = self.receive_data().await?;
        String::from_utf8(data)
            .map_err(|e| ProtocolError::ParseError(e.to_string()))
    }

    /// 请求-响应模式
    pub async fn request_response(&mut self, request: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        self.send_data(request).await?;
        self.receive_data().await
    }

    /// 获取通道信息
    pub fn channel_info(&self) -> &ChannelInfo {
        self.channel.info()
    }
}

#[async_trait]
impl Protocol for VirtioSerialProtocol {
    async fn connect(&mut self, _domain: &Domain) -> Result<(), ProtocolError> {
        info!("连接 VirtioSerial 协议");

        self.channel.connect().await?;
        self.connected = true;

        info!("VirtioSerial 协议已连接");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ProtocolError> {
        info!("断开 VirtioSerial 协议");

        self.channel.disconnect().await;
        self.connected = false;

        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        self.send_data(data).await
    }

    async fn receive(&mut self) -> Result<Vec<u8>, ProtocolError> {
        self.receive_data().await
    }


    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::VirtioSerial("virtio-serial".to_string())
    }

    async fn is_connected(&self) -> bool {
        self.connected && self.channel.is_connected()
    }
}

/// VirtioSerial 协议构建器
pub struct VirtioSerialBuilder {
    /// 通道名称
    channel_name: String,

    /// Socket 路径（可选，如果不提供则从 domain XML 解析）
    socket_path: Option<std::path::PathBuf>,

    /// 协议处理器
    handler: Option<Arc<dyn ProtocolHandler>>,
}

impl VirtioSerialBuilder {
    /// 创建新的构建器
    pub fn new(channel_name: &str) -> Self {
        Self {
            channel_name: channel_name.to_string(),
            socket_path: None,
            handler: None,
        }
    }

    /// 设置 socket 路径
    pub fn with_socket_path(mut self, path: std::path::PathBuf) -> Self {
        self.socket_path = Some(path);
        self
    }

    /// 设置协议处理器
    pub fn with_handler(mut self, handler: Arc<dyn ProtocolHandler>) -> Self {
        self.handler = Some(handler);
        self
    }

    /// 使用原始协议处理器
    pub fn with_raw_handler(self) -> Self {
        self.with_handler(Arc::new(RawProtocolHandler))
    }

    /// 使用 JSON 协议处理器
    pub fn with_json_handler(self) -> Self {
        self.with_handler(Arc::new(JsonProtocolHandler::default()))
    }

    /// 使用自定义 JSON 协议处理器
    pub fn with_custom_json_handler(self, request_field: &str, response_field: &str) -> Self {
        self.with_handler(Arc::new(JsonProtocolHandler::new(request_field, response_field)))
    }
}

impl ProtocolBuilder for VirtioSerialBuilder {
    fn build(&self) -> Box<dyn Protocol> {
        // 如果没有指定处理器，使用原始处理器
        let handler = self.handler.clone()
            .unwrap_or_else(|| Arc::new(RawProtocolHandler));

        // 创建通道
        let channel = if let Some(path) = &self.socket_path {
            VirtioChannel::new(&self.channel_name, path.clone())
        } else {
            // 如果没有提供路径，创建一个未连接的通道
            // 实际连接时会从 domain XML 解析路径
            VirtioChannel::new(&self.channel_name, std::path::PathBuf::new())
        };

        Box::new(VirtioSerialProtocol::new(channel, handler))
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::VirtioSerial(self.channel_name.clone())
    }
}

impl Default for VirtioSerialBuilder {
    fn default() -> Self {
        Self::new("com.vmagent.sock")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_raw_handler() {
        let handler = RawProtocolHandler;
        let data = b"test data";

        let encoded = handler.encode_request(data).await.unwrap();
        assert_eq!(encoded, data);

        let decoded = handler.decode_response(&encoded).await.unwrap();
        assert_eq!(decoded, data);
    }

    #[tokio::test]
    async fn test_json_handler() {
        let handler = JsonProtocolHandler::default();
        let data = b"test message";

        let encoded = handler.encode_request(data).await.unwrap();
        let json_str = String::from_utf8_lossy(&encoded);
        assert!(json_str.contains("test message"));
        assert!(json_str.contains("data"));

        // 模拟响应
        let response = br#"{"result": "success"}"#;
        let decoded = handler.decode_response(response).await.unwrap();
        assert_eq!(decoded, b"success");
    }
}

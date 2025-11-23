use anyhow::{Context, Result};
use serde_json;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, info, warn};

use crate::capture::KeyEvent;

/// WebSocket 客户端
pub struct WebSocketClient {
    server_url: String,
}

impl WebSocketClient {
    pub fn new(server_url: String) -> Self {
        Self { server_url }
    }

    /// 连接到 WebSocket 服务器
    pub async fn connect(&self) -> Result<()> {
        info!("连接到 WebSocket 服务器: {}", self.server_url);

        let (ws_stream, _) = connect_async(&self.server_url)
            .await
            .context("连接 WebSocket 失败")?;

        info!("WebSocket 已连接");

        let (mut write, mut read) = ws_stream.split();

        // 发送注册消息
        let register_msg = serde_json::json!({
            "type": "register",
            "agentId": "native-agent",
            "platform": std::env::consts::OS,
            "timestamp": chrono::Utc::now().timestamp_millis()
        });

        write
            .send(Message::Text(register_msg.to_string()))
            .await
            .context("发送注册消息失败")?;

        // 接收消息循环
        while let Some(message) = read.next().await {
            match message {
                Ok(msg) => {
                    debug!("收到消息: {:?}", msg);
                    if let Message::Text(text) = msg {
                        self.handle_message(&text)?;
                    }
                }
                Err(e) => {
                    warn!("接收消息错误: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// 处理收到的消息
    fn handle_message(&self, message: &str) -> Result<()> {
        let value: serde_json::Value = serde_json::from_str(message)?;
        debug!("解析消息: {:?}", value);

        // TODO: 处理不同类型的消息

        Ok(())
    }

    /// 发送键盘事件（占位符）
    pub async fn send_key_event(&mut self, event: KeyEvent) -> Result<()> {
        // TODO: 实现发送逻辑
        debug!("发送键盘事件: {:?}", event);
        Ok(())
    }
}

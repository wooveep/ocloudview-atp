//! SPICE 协议实现
//!
//! SPICE (Simple Protocol for Independent Computing Environments) 是一个开源的远程桌面协议。
//! 本模块实现 SPICE 协议的各种通道，用于模拟用户操作和测试 VDI 环境的负载。
//!
//! ## 架构
//!
//! SPICE 协议使用多通道架构，每个通道负责不同类型的数据传输：
//!
//! - **Main 通道**: 主会话连接、握手、认证
//! - **Display 通道**: 远程显示更新、图形命令、视频流
//! - **Inputs 通道**: 键盘和鼠标事件
//! - **Cursor 通道**: 鼠标光标形状和定位
//! - **Playback 通道**: 音频输出
//! - **Record 通道**: 麦克风输入
//! - **Usbredir 通道**: USB 设备重定向
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use atp_protocol::spice::{SpiceClient, SpiceConfig};
//!
//! // 创建 SPICE 客户端
//! let config = SpiceConfig::new("192.168.1.100", 5900)
//!     .with_password("secret");
//!
//! let mut client = SpiceClient::new(config);
//! client.connect().await?;
//!
//! // 发送键盘输入
//! client.inputs().send_key_down(0x1E).await?;  // 'A' key
//! client.inputs().send_key_up(0x1E).await?;
//!
//! // 发送鼠标移动
//! client.inputs().send_mouse_position(100, 200, 0).await?;
//! client.inputs().send_mouse_press(MouseButton::Left).await?;
//! ```

pub mod types;
pub mod constants;
pub mod messages;
pub mod channel;
pub mod discovery;
pub mod client;
pub mod inputs;
pub mod display;
pub mod usbredir;

// 重新导出主要类型
pub use types::*;
pub use constants::*;
pub use channel::{SpiceChannel, ChannelType};
pub use discovery::{SpiceDiscovery, SpiceVmInfo};
pub use client::{SpiceClient, SpiceConfig};
pub use inputs::{InputsChannel, MouseButton, MouseMode, KeyModifiers};
pub use display::{DisplayChannel, DisplayConfig};
pub use usbredir::{UsbRedirChannel, UsbDevice, UsbFilter};

use crate::{Protocol, ProtocolBuilder, ProtocolError, ProtocolType, Result};
use async_trait::async_trait;
use virt::domain::Domain;
use std::sync::Arc;
use tokio::sync::RwLock;

/// SPICE 协议实现
///
/// 实现 Protocol trait，提供统一的协议接口
pub struct SpiceProtocol {
    /// SPICE 客户端
    client: Option<Arc<RwLock<SpiceClient>>>,
    /// 配置
    config: Option<SpiceConfig>,
    /// 连接状态
    connected: bool,
}

impl SpiceProtocol {
    pub fn new() -> Self {
        Self {
            client: None,
            config: None,
            connected: false,
        }
    }

    /// 使用配置创建
    pub fn with_config(config: SpiceConfig) -> Self {
        Self {
            client: None,
            config: Some(config),
            connected: false,
        }
    }

    /// 获取 SPICE 客户端引用
    pub fn client(&self) -> Option<Arc<RwLock<SpiceClient>>> {
        self.client.clone()
    }

    /// 发送键盘按键
    pub async fn send_key(&self, scancode: u32, pressed: bool) -> Result<()> {
        let client = self.client.as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("SPICE 未连接".to_string()))?;

        let client_guard = client.read().await;
        if pressed {
            client_guard.inputs().send_key_down(scancode).await
        } else {
            client_guard.inputs().send_key_up(scancode).await
        }
    }

    /// 发送鼠标移动
    pub async fn send_mouse_move(&self, x: u32, y: u32, display_id: u8) -> Result<()> {
        let client = self.client.as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("SPICE 未连接".to_string()))?;

        let client_guard = client.read().await;
        client_guard.inputs().send_mouse_position(x, y, display_id).await
    }

    /// 发送鼠标点击
    pub async fn send_mouse_click(&self, button: MouseButton, pressed: bool) -> Result<()> {
        let client = self.client.as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("SPICE 未连接".to_string()))?;

        let client_guard = client.read().await;
        if pressed {
            client_guard.inputs().send_mouse_press(button).await
        } else {
            client_guard.inputs().send_mouse_release(button).await
        }
    }
}

impl Default for SpiceProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Protocol for SpiceProtocol {
    async fn connect(&mut self, domain: &Domain) -> Result<()> {
        // 通过 libvirt 发现 SPICE 配置
        let discovery = SpiceDiscovery::new();
        let vm_info = discovery.discover_from_domain(domain).await?;

        // 创建配置
        let config = SpiceConfig::new(&vm_info.host, vm_info.port)
            .with_tls_port(vm_info.tls_port)
            .with_password_opt(vm_info.password);

        // 创建并连接客户端
        let mut client = SpiceClient::new(config.clone());
        client.connect().await?;

        self.client = Some(Arc::new(RwLock::new(client)));
        self.config = Some(config);
        self.connected = true;

        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        // SPICE 协议通过特定通道发送数据，这里作为通用接口
        // 实际使用时应该调用具体的通道方法
        let _client = self.client.as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("SPICE 未连接".to_string()))?;

        // 将数据发送到主通道（用于调试/测试）
        tracing::debug!("SPICE send {} bytes via generic interface", data.len());

        // TODO: 实现通过主通道发送原始数据
        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        // SPICE 协议通过特定通道接收数据
        let _client = self.client.as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("SPICE 未连接".to_string()))?;

        // TODO: 实现从主通道接收数据
        Ok(Vec::new())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(client) = self.client.take() {
            let mut client_guard = client.write().await;
            client_guard.disconnect().await?;
        }

        self.connected = false;
        self.config = None;

        tracing::info!("SPICE 连接已断开");
        Ok(())
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::Spice
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }
}

/// SPICE 协议构建器
pub struct SpiceProtocolBuilder {
    config: Option<SpiceConfig>,
}

impl SpiceProtocolBuilder {
    pub fn new() -> Self {
        Self { config: None }
    }

    pub fn with_config(mut self, config: SpiceConfig) -> Self {
        self.config = Some(config);
        self
    }
}

impl Default for SpiceProtocolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolBuilder for SpiceProtocolBuilder {
    fn build(&self) -> Box<dyn Protocol> {
        match &self.config {
            Some(config) => Box::new(SpiceProtocol::with_config(config.clone())),
            None => Box::new(SpiceProtocol::new()),
        }
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::Spice
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spice_protocol_creation() {
        let protocol = SpiceProtocol::new();
        assert_eq!(protocol.protocol_type(), ProtocolType::Spice);
        assert!(!protocol.connected);
    }

    #[test]
    fn test_spice_config_builder() {
        let config = SpiceConfig::new("192.168.1.100", 5900)
            .with_password("test123")
            .with_tls_port(Some(5901));

        assert_eq!(config.host, "192.168.1.100");
        assert_eq!(config.port, 5900);
        assert_eq!(config.tls_port, Some(5901));
        assert_eq!(config.password, Some("test123".to_string()));
    }
}

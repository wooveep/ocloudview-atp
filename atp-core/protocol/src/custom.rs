//! 自定义 virtio-serial 协议支持

use async_trait::async_trait;
use virt::domain::Domain;

use crate::{Protocol, ProtocolBuilder, ProtocolType, Result};

/// 自定义 virtio-serial 协议
pub struct CustomProtocol {
    /// 通道名称
    channel_name: String,

    /// 连接状态
    connected: bool,
}

impl CustomProtocol {
    /// 创建自定义协议
    ///
    /// # 参数
    /// - `channel_name`: virtio-serial 通道名称，如 "org.example.custom.0"
    pub fn new(channel_name: String) -> Self {
        Self {
            channel_name,
            connected: false,
        }
    }
}

#[async_trait]
impl Protocol for CustomProtocol {
    async fn connect(&mut self, _domain: &Domain) -> Result<()> {
        // TODO: 实现自定义协议连接逻辑
        // 1. 通过 Libvirt 获取 virtio-serial 设备路径
        // 2. 打开通道
        self.connected = true;
        Ok(())
    }

    async fn send(&mut self, _data: &[u8]) -> Result<()> {
        // TODO: 实现发送逻辑
        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        // TODO: 实现接收逻辑
        Ok(Vec::new())
    }

    async fn disconnect(&mut self) -> Result<()> {
        // TODO: 实现断开逻辑
        self.connected = false;
        Ok(())
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::VirtioSerial(self.channel_name.clone())
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }
}

/// 自定义协议构建器
pub struct CustomProtocolBuilder {
    channel_name: String,
}

impl CustomProtocolBuilder {
    pub fn new(channel_name: String) -> Self {
        Self { channel_name }
    }
}

impl ProtocolBuilder for CustomProtocolBuilder {
    fn build(&self) -> Box<dyn Protocol> {
        Box::new(CustomProtocol::new(self.channel_name.clone()))
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::VirtioSerial(self.channel_name.clone())
    }
}

//! QGA 协议实现
//!
//! TODO: 将现有的 test-controller/src/qga 代码迁移到这里

use async_trait::async_trait;
use virt::domain::Domain;

use crate::{Protocol, ProtocolBuilder, ProtocolType, Result};

/// QGA 协议实现
pub struct QgaProtocol {
    // TODO: 添加字段
    connected: bool,
}

impl QgaProtocol {
    pub fn new() -> Self {
        Self { connected: false }
    }
}

#[async_trait]
impl Protocol for QgaProtocol {
    async fn connect(&mut self, _domain: &Domain) -> Result<()> {
        // TODO: 实现连接逻辑
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
        ProtocolType::QGA
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }
}

/// QGA 协议构建器
pub struct QgaProtocolBuilder;

impl ProtocolBuilder for QgaProtocolBuilder {
    fn build(&self) -> Box<dyn Protocol> {
        Box::new(QgaProtocol::new())
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::QGA
    }
}

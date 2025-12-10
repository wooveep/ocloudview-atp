//! 协议抽象接口

use async_trait::async_trait;
use virt::domain::Domain;

use crate::Result;

/// 协议类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProtocolType {
    /// QMP 协议
    QMP,

    /// QGA 协议
    QGA,

    /// 自定义 virtio-serial 协议
    VirtioSerial(String),

    /// SPICE 协议（预留）
    Spice,
}

/// 协议trait
///
/// 所有协议实现必须实现此 trait
#[async_trait]
pub trait Protocol: Send + Sync {
    /// 连接到虚拟机
    async fn connect(&mut self, domain: &Domain) -> Result<()>;

    /// 发送数据
    async fn send(&mut self, data: &[u8]) -> Result<()>;

    /// 接收数据
    async fn receive(&mut self) -> Result<Vec<u8>>;

    /// 断开连接
    async fn disconnect(&mut self) -> Result<()>;

    /// 获取协议类型
    fn protocol_type(&self) -> ProtocolType;

    /// 检查连接是否活跃
    async fn is_connected(&self) -> bool;

    /// 获取协议名称
    fn name(&self) -> String {
        match self.protocol_type() {
            ProtocolType::QMP => "qmp".to_string(),
            ProtocolType::QGA => "qga".to_string(),
            ProtocolType::VirtioSerial(name) => name,
            ProtocolType::Spice => "spice".to_string(),
        }
    }
}

/// 协议构建器 trait
///
/// 用于创建协议实例
pub trait ProtocolBuilder: Send + Sync {
    /// 构建协议实例
    fn build(&self) -> Box<dyn Protocol>;

    /// 获取协议类型
    fn protocol_type(&self) -> ProtocolType;
}

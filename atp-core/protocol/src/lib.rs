//! ATP 协议层
//!
//! 提供统一的协议抽象接口，支持 QMP、QGA、VirtioSerial 和 SPICE。
//!
//! ## 支持的协议
//!
//! - **QMP**: QEMU Machine Protocol，用于管理和监控虚拟机
//! - **QGA**: QEMU Guest Agent，用于在虚拟机内部执行命令
//! - **VirtioSerial**: 自定义 virtio-serial 协议支持
//! - **SPICE**: 远程桌面协议，支持多通道（显示、输入、USB 重定向等）

pub mod traits;
pub mod registry;
pub mod qmp;
pub mod qga;
pub mod virtio;
pub mod custom;
pub mod spice;

pub use traits::{Protocol, ProtocolType, ProtocolBuilder};
pub use registry::ProtocolRegistry;

// 导出 VirtioSerial 相关类型
pub use virtio::{
    VirtioSerialProtocol,
    VirtioSerialBuilder,
    VirtioChannel,
    ChannelInfo,
    ProtocolHandler,
    RawProtocolHandler,
    JsonProtocolHandler,
};

// 导出 SPICE 相关类型
pub use spice::{
    SpiceProtocol,
    SpiceProtocolBuilder,
    SpiceClient,
    SpiceConfig,
    SpiceDiscovery,
    SpiceVmInfo,
    InputsChannel,
    DisplayChannel,
    UsbRedirChannel,
    MouseButton,
    MouseMode,
    KeyModifiers,
    DisplayConfig,
    UsbDevice,
    UsbFilter,
};

use thiserror::Error;

/// 协议层错误
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("协议 {0} 不存在")]
    ProtocolNotFound(String),

    #[error("协议 {0} 已注册")]
    ProtocolAlreadyRegistered(String),

    #[error("协议连接失败: {0}")]
    ConnectionFailed(String),

    #[error("发送数据失败: {0}")]
    SendFailed(String),

    #[error("接收数据失败: {0}")]
    ReceiveFailed(String),

    #[error("协议解析失败: {0}")]
    ParseError(String),

    #[error("命令执行失败: {0}")]
    CommandFailed(String),

    #[error("超时")]
    Timeout,

    #[error("传输层错误: {0}")]
    TransportError(#[from] atp_transport::TransportError),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ProtocolError>;

//! VirtioSerial 自定义协议支持
//!
//! 支持通过 virtio-serial 通道与虚拟机内的自定义 agent 通信。
//! 协议内容可扩展，支持原始数据发送和自定义协议处理器。

mod channel;
mod protocol;

pub use channel::{VirtioChannel, ChannelInfo};
pub use protocol::{
    VirtioSerialProtocol,
    VirtioSerialBuilder,
    ProtocolHandler,
    RawProtocolHandler,
    JsonProtocolHandler,
};

use crate::ProtocolError;

/// VirtioSerial 协议结果
pub type Result<T> = std::result::Result<T, ProtocolError>;

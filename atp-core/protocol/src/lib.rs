//! ATP 协议层
//!
//! 提供统一的协议抽象接口，支持 QMP、QGA、自定义协议和 SPICE（预留）。

pub mod traits;
pub mod registry;
pub mod qmp;
pub mod qga;
pub mod custom;

pub use traits::{Protocol, ProtocolType};
pub use registry::ProtocolRegistry;

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

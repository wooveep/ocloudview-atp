//! Guest 验证器服务端
//!
//! 提供 WebSocket/TCP 服务器，接收 Guest Agent 的验证结果，
//! 并与发送的事件进行一对一匹配。

pub mod server;
pub mod service;
pub mod types;
pub mod client;

pub use server::VerificationServer;
pub use service::VerificationService;
pub use types::{Event, VerifyResult, ClientConnection};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerificationError {
    #[error("客户端未连接: {0}")]
    ClientNotConnected(String),

    #[error("验证超时")]
    Timeout,

    #[error("事件未找到: {0}")]
    EventNotFound(String),

    #[error("服务器错误: {0}")]
    ServerError(String),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, VerificationError>;

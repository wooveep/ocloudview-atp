//! Guest 验证器核心库

pub mod verifier;
pub mod transport;
pub mod event;

pub use verifier::{Verifier, VerifierType};
pub use transport::VerifierTransport;
pub use event::{Event, RawInputEvent, VerifiedInputEvent, VerifyResult};

// 为了向后兼容，保留 InputEvent 别名
#[allow(deprecated)]
pub use event::InputEvent;

// 重新导出传输实现
pub use transport::{WebSocketTransport, TcpTransport};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerifierError {
    #[error("验证失败: {0}")]
    VerificationFailed(String),

    #[error("连接失败: {0}")]
    ConnectionFailed(String),

    #[error("超时")]
    Timeout,

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, VerifierError>;

//! SSH 错误定义

use thiserror::Error;

/// SSH 操作结果类型
pub type Result<T> = std::result::Result<T, SshError>;

/// SSH 错误类型
#[derive(Error, Debug)]
pub enum SshError {
    /// 连接错误
    #[error("SSH 连接失败: {0}")]
    ConnectionError(String),

    /// 认证错误
    #[error("SSH 认证失败: {0}")]
    AuthenticationError(String),

    /// 密钥加载错误
    #[error("SSH 密钥加载失败: {0}")]
    KeyLoadError(String),

    /// 会话错误
    #[error("SSH 会话错误: {0}")]
    SessionError(String),

    /// 通道错误
    #[error("SSH 通道错误: {0}")]
    ChannelError(String),

    /// 命令执行错误
    #[error("命令执行失败: {0}")]
    ExecutionError(String),

    /// 超时错误
    #[error("SSH 操作超时: {0}")]
    TimeoutError(String),

    /// IO 错误
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    /// 配置错误
    #[error("配置错误: {0}")]
    ConfigError(String),
}

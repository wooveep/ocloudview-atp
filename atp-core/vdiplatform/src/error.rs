//! VDI 平台错误定义

use thiserror::Error;

/// VDI 平台错误类型
#[derive(Error, Debug)]
pub enum VdiError {
    #[error("HTTP 错误: {0}")]
    HttpError(String),

    #[error("认证错误: {0}")]
    AuthError(String),

    #[error("API 错误 [{0}]: {1}")]
    ApiError(u16, String),

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("超时错误: {0}")]
    Timeout(String),

    #[error("资源不存在: {0}")]
    NotFound(String),

    #[error("操作失败: {0}")]
    OperationFailed(String),

    #[error("未知错误: {0}")]
    Unknown(String),
}

/// VDI 平台结果类型
pub type Result<T> = std::result::Result<T, VdiError>;

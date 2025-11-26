//! 场景编排器
//!
//! 用于编排和执行 VDI 平台测试场景，整合 VDI 平台操作和虚拟化层操作。

pub mod scenario;
pub mod executor;
pub mod adapter;
pub mod report;

pub use scenario::{TestScenario, TestStep, VdiAction, VirtualizationAction, VerifyCondition};
pub use executor::ScenarioExecutor;
pub use adapter::VdiVirtualizationAdapter;
pub use report::{TestReport, StepResult, StepStatus};

use thiserror::Error;

/// 编排器错误类型
#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("VDI 平台错误: {0}")]
    VdiError(String),

    #[error("虚拟化层错误: {0}")]
    VirtualizationError(String),

    #[error("场景解析错误: {0}")]
    ScenarioParseError(String),

    #[error("验证失败: {0}")]
    VerificationFailed(String),

    #[error("超时: {0}")]
    Timeout(String),

    #[error("资源不存在: {0}")]
    ResourceNotFound(String),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("未知错误: {0}")]
    Unknown(String),
}

/// 编排器结果类型
pub type Result<T> = std::result::Result<T, OrchestratorError>;

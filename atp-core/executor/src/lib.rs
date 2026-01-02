//! ATP 执行器
//!
//! 测试场景执行引擎

pub mod scenario;
pub mod runner;
pub mod test_config;

pub use scenario::{
    Scenario, ScenarioStep, Action, VerificationConfig,
    TargetSelector, TargetSelectorConfig, TargetMode,
    ParallelConfig, FailureStrategy,
};
pub use runner::{ScenarioRunner, ExecutionReport, StepReport, StepStatus, MultiTargetReport};
pub use test_config::{TestConfig, VdiConfig};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("场景加载失败: {0}")]
    ScenarioLoadFailed(String),

    #[error("步骤执行失败: {0}")]
    StepExecutionFailed(String),

    #[error("超时")]
    Timeout,

    #[error("协议错误: {0}")]
    ProtocolError(String),

    #[error("传输错误: {0}")]
    TransportError(String),

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    SerdeError(String),

    #[error("数据库错误: {0}")]
    DatabaseError(String),
}

pub type Result<T> = std::result::Result<T, ExecutorError>;

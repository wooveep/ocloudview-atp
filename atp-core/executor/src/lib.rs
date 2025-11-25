//! ATP 执行器
//!
//! 测试场景执行引擎

pub mod scenario;
pub mod runner;

pub use scenario::{Scenario, ScenarioStep, Action};
pub use runner::{ScenarioRunner, ExecutionReport, StepReport, StepStatus};

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
    ProtocolError(#[from] atp_protocol::ProtocolError),

    #[error("传输错误: {0}")]
    TransportError(#[from] atp_transport::TransportError),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    SerdeError(String),
}

pub type Result<T> = std::result::Result<T, ExecutorError>;

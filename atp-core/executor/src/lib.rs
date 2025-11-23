//! ATP 执行器
//!
//! 测试场景执行引擎

pub mod scenario;
pub mod runner;

pub use scenario::{Scenario, ScenarioStep};
pub use runner::ScenarioRunner;

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
}

pub type Result<T> = std::result::Result<T, ExecutorError>;

//! ATP 执行器
//!
//! 测试场景执行引擎

pub mod runner;
pub mod scenario;
pub mod spice_connector;
pub mod ssh_manager;
pub mod storage_ops;
pub mod test_config;
pub mod vdi_ops;

pub use runner::{ExecutionReport, MultiTargetReport, ScenarioRunner, StepReport, StepStatus};
pub use scenario::{
    Action, FailureStrategy, InputChannelConfig, InputChannelType, ParallelConfig, Scenario,
    ScenarioStep, TargetMode, TargetSelector, TargetSelectorConfig, VerificationConfig,
};
pub use ssh_manager::{SshConnectionManager, SshParams};
pub use storage_ops::{
    // 脑裂修复
    AffectedVm,
    AutoReplicaSelector,
    // 磁盘位置查询
    DiskLocationInfo,
    DiskLocationResult,
    HealEntryResult,
    HealReport,
    HealStrategy,
    InteractiveReplicaSelector,
    ReplicaSelector,
    ReplicaStat,
    StorageOpsService,
};
pub use test_config::{TestConfig, VdiConfig};
pub use vdi_ops::{
    // Public utility functions
    ensure_host_registered,
    get_domain_libvirt_state,
    list_host_domains_state,
    matches_pattern,
    // New batch operation results
    BatchAssignResult,
    BatchAutoAdResult,
    BatchOpError,
    BatchOperations,
    BatchRenameResult,
    BatchStartError,
    BatchStartResult,
    // New verification types
    CompareResult,
    LibvirtDomainState,
    QgaVerifier,
    QgaVerifyResult,
    VdiBatchOps,
    VdiVerifyOps,
    VerifyResult,
    VmInfo,
    VmMatcher,
};

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

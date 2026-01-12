//! Gluster 错误定义

use thiserror::Error;

/// Gluster 操作结果类型
pub type Result<T> = std::result::Result<T, GlusterError>;

/// Gluster 错误类型
#[derive(Error, Debug)]
pub enum GlusterError {
    /// SSH 错误
    #[error("SSH 错误: {0}")]
    SshError(#[from] atp_ssh_executor::SshError),

    /// 命令执行错误
    #[error("命令执行错误: {0}")]
    CommandError(String),

    /// 解析错误
    #[error("解析错误: {0}")]
    ParseError(String),

    /// 文件不存在
    #[error("文件不存在: {0}")]
    FileNotFound(String),

    /// 不是 Gluster 文件系统
    #[error("不是 Gluster 文件系统: {0}")]
    NotGlusterFs(String),

    /// getfattr 未安装
    #[error("getfattr 命令未找到，请安装 attr 包")]
    GetfattrNotFound,

    /// Gluster 未运行
    #[error("Gluster 服务未运行")]
    GlusterNotRunning,

    /// 卷不存在
    #[error("Gluster 卷不存在: {0}")]
    VolumeNotFound(String),
}

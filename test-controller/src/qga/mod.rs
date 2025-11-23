pub mod protocol;
pub mod client;
pub mod examples;

pub use protocol::{
    GuestExecCommand, GuestExecStatus, GuestExecStatusRequest,
    GuestFileOpen, GuestFileRead, GuestFileWrite, GuestFileClose,
    GuestInfo, GuestOsInfo,
};
pub use client::QgaClient;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum QgaError {
    #[error("QGA 命令执行失败: {0}")]
    CommandFailed(String),

    #[error("QGA 未响应")]
    NoResponse,

    #[error("QGA 响应解析失败: {0}")]
    ParseError(String),

    #[error("进程执行超时")]
    Timeout,

    #[error("进程执行失败，退出码: {0}")]
    ProcessFailed(i32),

    #[error("Libvirt 错误: {0}")]
    LibvirtError(String),
}

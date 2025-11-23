pub mod manager;

pub use manager::LibvirtManager;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LibvirtError {
    #[error("连接到 Libvirt 失败: {0}")]
    ConnectionFailed(String),

    #[error("查找虚拟机失败: {0}")]
    DomainNotFound(String),

    #[error("获取虚拟机 XML 失败: {0}")]
    XmlParseFailed(String),

    #[error("QMP Socket 路径未找到")]
    QmpSocketNotFound,

    #[error("Libvirt 操作错误: {0}")]
    OperationError(String),
}

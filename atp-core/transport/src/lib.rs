//! ATP 传输层
//!
//! 负责与 Libvirt 的长连接管理，支持多主机节点和并发执行。

pub mod config;
pub mod connection;
pub mod pool;
pub mod manager;

pub use config::{TransportConfig, PoolConfig};
pub use connection::{HostConnection, ConnectionState};
pub use pool::ConnectionPool;
pub use manager::TransportManager;

use thiserror::Error;

/// 传输层错误
#[derive(Error, Debug)]
pub enum TransportError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),

    #[error("主机 {0} 不存在")]
    HostNotFound(String),

    #[error("虚拟机 {0} 不存在")]
    DomainNotFound(String),

    #[error("连接池已满")]
    PoolExhausted,

    #[error("连接超时")]
    Timeout,

    #[error("连接已断开")]
    Disconnected,

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Libvirt 错误: {0}")]
    LibvirtError(String),

    #[error("配置错误: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, TransportError>;

/// 主机信息
#[derive(Debug, Clone)]
pub struct HostInfo {
    /// 主机 ID
    pub id: String,

    /// 主机名或 IP
    pub host: String,

    /// Libvirt URI
    pub uri: String,

    /// 标签（用于分组）
    pub tags: Vec<String>,

    /// 元数据
    pub metadata: std::collections::HashMap<String, String>,
}

impl HostInfo {
    pub fn new(id: &str, host: &str) -> Self {
        Self {
            id: id.to_string(),
            host: host.to_string(),
            uri: format!("qemu+ssh://{}:22/system", host),
            tags: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_uri(mut self, uri: &str) -> Self {
        self.uri = uri.to_string();
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

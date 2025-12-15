//! ATP 传输层
//!
//! 负责与 Libvirt 的长连接管理，支持多主机节点和并发执行。

pub mod config;
pub mod connection;
pub mod pool;
pub mod manager;

pub use config::{TransportConfig, PoolConfig, ReconnectConfig, SelectionStrategy};
pub use connection::{HostConnection, ConnectionState, ConnectionMetrics};
pub use pool::{ConnectionPool, ConnectionPoolStats};
pub use manager::TransportManager;

use serde::{Deserialize, Serialize};
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

// ============================================
// Libvirt 虚拟机状态枚举
// ============================================

/// Libvirt 虚拟机状态
///
/// 对应 libvirt 的 virDomainState 枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LibvirtDomainState {
    /// 未定义状态
    NoState,
    /// 运行中
    Running,
    /// 阻塞（等待 I/O）
    Blocked,
    /// 暂停
    Paused,
    /// 正在关机
    Shutdown,
    /// 已关闭
    Shutoff,
    /// 崩溃
    Crashed,
    /// 休眠到磁盘
    PMSuspended,
    /// 未知状态
    Unknown,
}

impl LibvirtDomainState {
    /// 从 virt::sys::virDomainState 创建
    pub fn from_virt_state(state: virt::sys::virDomainState) -> Self {
        match state {
            virt::sys::VIR_DOMAIN_NOSTATE => Self::NoState,
            virt::sys::VIR_DOMAIN_RUNNING => Self::Running,
            virt::sys::VIR_DOMAIN_BLOCKED => Self::Blocked,
            virt::sys::VIR_DOMAIN_PAUSED => Self::Paused,
            virt::sys::VIR_DOMAIN_SHUTDOWN => Self::Shutdown,
            virt::sys::VIR_DOMAIN_SHUTOFF => Self::Shutoff,
            virt::sys::VIR_DOMAIN_CRASHED => Self::Crashed,
            virt::sys::VIR_DOMAIN_PMSUSPENDED => Self::PMSuspended,
            _ => Self::Unknown,
        }
    }

    /// 获取中文显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::NoState => "未定义",
            Self::Running => "运行中",
            Self::Blocked => "阻塞",
            Self::Paused => "暂停",
            Self::Shutdown => "关机中",
            Self::Shutoff => "已关闭",
            Self::Crashed => "崩溃",
            Self::PMSuspended => "休眠",
            Self::Unknown => "未知",
        }
    }

    /// 获取带 emoji 的显示名称
    pub fn display_with_emoji(&self) -> &'static str {
        match self {
            Self::NoState => "未定义 ⚪",
            Self::Running => "运行中 ✅",
            Self::Blocked => "阻塞 ⏳",
            Self::Paused => "暂停 🟡",
            Self::Shutdown => "关机中 ⏹️",
            Self::Shutoff => "已关闭 ⚫",
            Self::Crashed => "崩溃 💥",
            Self::PMSuspended => "休眠 🌙",
            Self::Unknown => "未知 ⚠️",
        }
    }

    /// 检查是否运行中
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running | Self::Blocked)
    }

    /// 检查是否已关闭
    pub fn is_shutoff(&self) -> bool {
        matches!(self, Self::Shutoff)
    }

    /// 检查是否活跃（非关闭状态）
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Shutoff | Self::Crashed | Self::NoState | Self::Unknown)
    }
}

impl std::fmt::Display for LibvirtDomainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Libvirt 虚拟机信息
#[derive(Debug, Clone)]
pub struct LibvirtDomainInfo {
    /// 虚拟机名称
    pub name: String,
    /// 虚拟机 UUID
    pub uuid: Option<String>,
    /// 状态
    pub state: LibvirtDomainState,
    /// vCPU 数量
    pub vcpu: u32,
    /// 内存大小 (MB)
    pub memory_mb: u64,
    /// 最大内存 (MB)
    pub max_memory_mb: u64,
}

/// 虚拟机列表过滤选项
#[derive(Debug, Clone, Default)]
pub struct ListDomainsFilter {
    /// 包含活跃的虚拟机
    pub active: bool,
    /// 包含非活跃的虚拟机
    pub inactive: bool,
    /// 按状态筛选（如果设置了则只返回匹配的状态）
    pub states: Option<Vec<LibvirtDomainState>>,
}

impl ListDomainsFilter {
    /// 创建查询所有虚拟机的过滤器
    pub fn all() -> Self {
        Self {
            active: true,
            inactive: true,
            states: None,
        }
    }

    /// 创建只查询活跃虚拟机的过滤器
    pub fn active_only() -> Self {
        Self {
            active: true,
            inactive: false,
            states: None,
        }
    }

    /// 创建只查询非活跃虚拟机的过滤器
    pub fn inactive_only() -> Self {
        Self {
            active: false,
            inactive: true,
            states: None,
        }
    }

    /// 添加状态筛选
    pub fn with_states(mut self, states: Vec<LibvirtDomainState>) -> Self {
        self.states = Some(states);
        self
    }

    /// 获取 libvirt 的 flags 值
    pub fn to_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.active {
            flags |= 1; // VIR_CONNECT_LIST_DOMAINS_ACTIVE
        }
        if self.inactive {
            flags |= 2; // VIR_CONNECT_LIST_DOMAINS_INACTIVE
        }
        if flags == 0 {
            flags = 3; // 默认查询所有
        }
        flags
    }
}

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

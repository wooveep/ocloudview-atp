//! 传输层配置

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 传输层配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// 连接池配置
    pub pool: PoolConfig,

    /// 连接超时（秒）
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,

    /// 心跳间隔（秒）
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: u64,

    /// 是否启用自动重连
    #[serde(default = "default_auto_reconnect")]
    pub auto_reconnect: bool,
}

/// 连接池配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// 每个主机的最大连接数
    #[serde(default = "default_max_connections_per_host")]
    pub max_connections_per_host: usize,

    /// 最小连接数
    #[serde(default = "default_min_connections_per_host")]
    pub min_connections_per_host: usize,

    /// 连接空闲超时（秒）
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            pool: PoolConfig::default(),
            connect_timeout: default_connect_timeout(),
            heartbeat_interval: default_heartbeat_interval(),
            auto_reconnect: default_auto_reconnect(),
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_host: default_max_connections_per_host(),
            min_connections_per_host: default_min_connections_per_host(),
            idle_timeout: default_idle_timeout(),
        }
    }
}

impl TransportConfig {
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout)
    }

    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.heartbeat_interval)
    }
}

impl PoolConfig {
    pub fn idle_timeout(&self) -> Duration {
        Duration::from_secs(self.idle_timeout)
    }
}

// 默认值函数
fn default_connect_timeout() -> u64 {
    30
}

fn default_heartbeat_interval() -> u64 {
    60
}

fn default_auto_reconnect() -> bool {
    true
}

fn default_max_connections_per_host() -> usize {
    10
}

fn default_min_connections_per_host() -> usize {
    1
}

fn default_idle_timeout() -> u64 {
    300
}

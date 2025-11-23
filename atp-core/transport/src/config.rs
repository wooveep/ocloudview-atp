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

    /// 重连配置
    pub reconnect: ReconnectConfig,
}

/// 重连配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectConfig {
    /// 最大重连次数（0 表示无限重连）
    #[serde(default = "default_max_reconnect_attempts")]
    pub max_attempts: u32,

    /// 初始重连延迟（秒）
    #[serde(default = "default_initial_reconnect_delay")]
    pub initial_delay: u64,

    /// 最大重连延迟（秒）
    #[serde(default = "default_max_reconnect_delay")]
    pub max_delay: u64,

    /// 重连延迟倍增因子
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
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

    /// 连接选择策略
    #[serde(default)]
    pub selection_strategy: SelectionStrategy,
}

/// 连接选择策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectionStrategy {
    /// 轮询
    RoundRobin,
    /// 最少连接
    LeastConnections,
    /// 随机
    Random,
}

impl Default for SelectionStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            pool: PoolConfig::default(),
            connect_timeout: default_connect_timeout(),
            heartbeat_interval: default_heartbeat_interval(),
            auto_reconnect: default_auto_reconnect(),
            reconnect: ReconnectConfig::default(),
        }
    }
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_reconnect_attempts(),
            initial_delay: default_initial_reconnect_delay(),
            max_delay: default_max_reconnect_delay(),
            backoff_multiplier: default_backoff_multiplier(),
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_host: default_max_connections_per_host(),
            min_connections_per_host: default_min_connections_per_host(),
            idle_timeout: default_idle_timeout(),
            selection_strategy: SelectionStrategy::default(),
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

impl ReconnectConfig {
    pub fn initial_delay(&self) -> Duration {
        Duration::from_secs(self.initial_delay)
    }

    pub fn max_delay(&self) -> Duration {
        Duration::from_secs(self.max_delay)
    }

    /// 计算指数退避延迟
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_secs = (self.initial_delay as f64 * self.backoff_multiplier.powi(attempt as i32))
            .min(self.max_delay as f64);
        Duration::from_secs(delay_secs as u64)
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

fn default_max_reconnect_attempts() -> u32 {
    5
}

fn default_initial_reconnect_delay() -> u64 {
    1
}

fn default_max_reconnect_delay() -> u64 {
    60
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

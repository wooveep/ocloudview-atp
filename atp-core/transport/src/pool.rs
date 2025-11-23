//! 连接池管理

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{HostConnection, HostInfo, PoolConfig, Result, SelectionStrategy, TransportConfig, TransportError};

/// 主机连接集合
struct HostConnections {
    connections: Vec<Arc<HostConnection>>,
    round_robin_index: usize,
}

/// 连接池
pub struct ConnectionPool {
    /// 主机连接映射
    hosts: Arc<RwLock<HashMap<String, HostConnections>>>,

    /// 连接池配置
    config: PoolConfig,

    /// 传输配置（用于创建连接）
    transport_config: Arc<TransportConfig>,
}

impl ConnectionPool {
    /// 创建新的连接池（使用默认传输配置）
    pub fn new(config: PoolConfig) -> Self {
        Self::with_transport_config(config, Arc::new(TransportConfig::default()))
    }

    /// 创建新的连接池（指定传输配置）
    pub fn with_transport_config(config: PoolConfig, transport_config: Arc<TransportConfig>) -> Self {
        Self {
            hosts: Arc::new(RwLock::new(HashMap::new())),
            config,
            transport_config,
        }
    }

    /// 添加主机
    pub async fn add_host(&self, host_info: HostInfo) -> Result<()> {
        let host_id = host_info.id.clone();
        info!("添加主机到连接池: {}", host_id);

        let mut hosts = self.hosts.write().await;

        // 检查主机是否已存在
        if hosts.contains_key(&host_id) {
            return Err(TransportError::ConfigError(format!(
                "主机 {} 已存在",
                host_id
            )));
        }

        // 创建初始连接
        let mut connections = Vec::new();
        for _ in 0..self.config.min_connections_per_host {
            let conn = HostConnection::with_config(
                host_info.clone(),
                Arc::clone(&self.transport_config)
            );
            connections.push(Arc::new(conn));
        }

        let host_conns = HostConnections {
            connections,
            round_robin_index: 0,
        };

        hosts.insert(host_id.clone(), host_conns);

        // 在后台异步建立连接
        let hosts_clone = Arc::clone(&self.hosts);
        let host_id_clone = host_id.clone();
        tokio::spawn(async move {
            if let Some(host_conns) = hosts_clone.read().await.get(&host_id_clone) {
                for conn in &host_conns.connections {
                    if let Err(e) = conn.connect().await {
                        warn!("主机 {} 的连接初始化失败: {}", host_id_clone, e);
                    }
                }
            }
        });

        Ok(())
    }

    /// 移除主机
    pub async fn remove_host(&self, host_id: &str) -> Result<()> {
        info!("从连接池移除主机: {}", host_id);

        let mut hosts = self.hosts.write().await;

        if let Some(host_conns) = hosts.remove(host_id) {
            // 断开所有连接
            for conn in host_conns.connections {
                let _ = conn.disconnect().await;
            }
            Ok(())
        } else {
            Err(TransportError::HostNotFound(host_id.to_string()))
        }
    }

    /// 获取连接（根据配置的策略）
    pub async fn get_connection(&self, host_id: &str) -> Result<Arc<HostConnection>> {
        match self.config.selection_strategy {
            SelectionStrategy::RoundRobin => self.get_connection_round_robin(host_id).await,
            SelectionStrategy::LeastConnections => self.get_connection_least_connections(host_id).await,
            SelectionStrategy::Random => self.get_connection_random(host_id).await,
        }
    }

    /// 轮询策略获取连接
    async fn get_connection_round_robin(&self, host_id: &str) -> Result<Arc<HostConnection>> {
        let mut hosts = self.hosts.write().await;

        let host_conns = hosts
            .get_mut(host_id)
            .ok_or_else(|| TransportError::HostNotFound(host_id.to_string()))?;

        if host_conns.connections.is_empty() {
            return Err(TransportError::PoolExhausted);
        }

        // 轮询选择下一个连接
        let index = host_conns.round_robin_index % host_conns.connections.len();
        host_conns.round_robin_index = (host_conns.round_robin_index + 1) % host_conns.connections.len();

        let conn = Arc::clone(&host_conns.connections[index]);

        debug!(
            "轮询策略选择主机 {} 的连接 {} (总共 {} 个连接)",
            host_id,
            index,
            host_conns.connections.len()
        );

        Ok(conn)
    }

    /// 最少连接策略获取连接
    async fn get_connection_least_connections(&self, host_id: &str) -> Result<Arc<HostConnection>> {
        let hosts = self.hosts.read().await;

        let host_conns = hosts
            .get(host_id)
            .ok_or_else(|| TransportError::HostNotFound(host_id.to_string()))?;

        if host_conns.connections.is_empty() {
            return Err(TransportError::PoolExhausted);
        }

        // 简化版：选择第一个已连接的连接
        // 在实际实现中，应该跟踪每个连接的活跃使用计数
        for (i, conn) in host_conns.connections.iter().enumerate() {
            if conn.is_alive().await {
                debug!(
                    "最少连接策略选择主机 {} 的连接 {}",
                    host_id, i
                );
                return Ok(Arc::clone(conn));
            }
        }

        // 如果没有已连接的，返回第一个
        debug!(
            "最少连接策略选择主机 {} 的连接 0 (无活跃连接)",
            host_id
        );
        Ok(Arc::clone(&host_conns.connections[0]))
    }

    /// 随机策略获取连接
    async fn get_connection_random(&self, host_id: &str) -> Result<Arc<HostConnection>> {
        let hosts = self.hosts.read().await;

        let host_conns = hosts
            .get(host_id)
            .ok_or_else(|| TransportError::HostNotFound(host_id.to_string()))?;

        if host_conns.connections.is_empty() {
            return Err(TransportError::PoolExhausted);
        }

        // 使用简单的随机选择
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let mut hasher = RandomState::new().build_hasher();
        std::time::SystemTime::now().hash(&mut hasher);
        let random_value = hasher.finish();
        let index = (random_value as usize) % host_conns.connections.len();

        debug!(
            "随机策略选择主机 {} 的连接 {}",
            host_id, index
        );

        Ok(Arc::clone(&host_conns.connections[index]))
    }

    /// 获取所有主机 ID
    pub async fn list_hosts(&self) -> Vec<String> {
        let hosts = self.hosts.read().await;
        hosts.keys().cloned().collect()
    }

    /// 获取主机的连接数
    pub async fn connection_count(&self, host_id: &str) -> Result<usize> {
        let hosts = self.hosts.read().await;
        hosts
            .get(host_id)
            .map(|host_conns| host_conns.connections.len())
            .ok_or_else(|| TransportError::HostNotFound(host_id.to_string()))
    }

    /// 获取主机的活跃连接数
    pub async fn active_connection_count(&self, host_id: &str) -> Result<usize> {
        let hosts = self.hosts.read().await;
        let host_conns = hosts
            .get(host_id)
            .ok_or_else(|| TransportError::HostNotFound(host_id.to_string()))?;

        let mut count = 0;
        for conn in &host_conns.connections {
            if conn.is_alive().await {
                count += 1;
            }
        }

        Ok(count)
    }

    /// 获取连接池统计信息
    pub async fn stats(&self) -> HashMap<String, ConnectionPoolStats> {
        let hosts = self.hosts.read().await;
        let mut stats = HashMap::new();

        for (host_id, host_conns) in hosts.iter() {
            let total = host_conns.connections.len();
            let mut active = 0;

            for conn in &host_conns.connections {
                if conn.is_alive().await {
                    active += 1;
                }
            }

            stats.insert(
                host_id.clone(),
                ConnectionPoolStats {
                    total_connections: total,
                    active_connections: active,
                    selection_strategy: self.config.selection_strategy,
                },
            );
        }

        stats
    }
}

/// 连接池统计信息
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub selection_strategy: SelectionStrategy,
}

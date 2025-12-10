//! 连接池管理

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
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

    /// 管理任务取消通道
    management_shutdown: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl ConnectionPool {
    /// 创建新的连接池（使用默认传输配置）
    pub fn new(config: PoolConfig) -> Self {
        Self::with_transport_config(config, Arc::new(TransportConfig::default()))
    }

    /// 创建新的连接池（指定传输配置）
    pub fn with_transport_config(config: PoolConfig, transport_config: Arc<TransportConfig>) -> Self {
        let pool = Self {
            hosts: Arc::new(RwLock::new(HashMap::new())),
            config,
            transport_config,
            management_shutdown: Arc::new(Mutex::new(None)),
        };

        // 启动管理任务
        pool.start_management_task();

        pool
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

        // 找到活跃使用数最少的连接
        let mut best_conn: Option<(Arc<HostConnection>, u32)> = None;

        for conn in &host_conns.connections {
            // 只考虑已连接的连接
            if !conn.is_alive().await {
                continue;
            }

            let active_uses = conn.metrics().active_uses().await;

            match best_conn {
                None => {
                    best_conn = Some((Arc::clone(conn), active_uses));
                }
                Some((_, current_min)) => {
                    if active_uses < current_min {
                        best_conn = Some((Arc::clone(conn), active_uses));
                    }
                }
            }
        }

        // 如果找到了活跃连接，返回它
        if let Some((conn, active_uses)) = best_conn {
            debug!(
                "最少连接策略选择主机 {} 的连接，活跃使用数: {}",
                host_id, active_uses
            );
            return Ok(conn);
        }

        // 如果没有活跃连接，返回第一个
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
            let mut total_requests = 0u64;
            let mut total_errors = 0u64;
            let mut total_active_uses = 0u32;

            for conn in &host_conns.connections {
                if conn.is_alive().await {
                    active += 1;
                }
                let metrics = conn.metrics();
                total_requests += metrics.total_requests().await;
                total_errors += metrics.error_count().await;
                total_active_uses += metrics.active_uses().await;
            }

            stats.insert(
                host_id.clone(),
                ConnectionPoolStats {
                    total_connections: total,
                    active_connections: active,
                    selection_strategy: self.config.selection_strategy,
                    total_requests,
                    total_errors,
                    total_active_uses,
                },
            );
        }

        stats
    }

    /// 启动管理任务（自动扩缩容和空闲连接清理）
    fn start_management_task(&self) {
        let hosts = Arc::clone(&self.hosts);
        let config = self.config.clone();
        let transport_config = Arc::clone(&self.transport_config);

        // 创建取消通道
        let (tx, mut rx) = tokio::sync::oneshot::channel();

        // 保存取消通道
        let shutdown = Arc::clone(&self.management_shutdown);
        tokio::spawn(async move {
            *shutdown.lock().await = Some(tx);
        });

        // 启动管理任务
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        debug!("执行连接池管理任务");

                        let mut hosts_guard = hosts.write().await;

                        for (host_id, host_conns) in hosts_guard.iter_mut() {
                            // 1. 清理空闲连接
                            Self::cleanup_idle_connections(
                                host_id,
                                host_conns,
                                config.idle_timeout(),
                                config.min_connections_per_host,
                            ).await;

                            // 2. 自动扩容（如果所有连接都在使用中）
                            Self::scale_up_if_needed(
                                host_id,
                                host_conns,
                                config.max_connections_per_host,
                                Arc::clone(&transport_config),
                            ).await;
                        }
                    }
                    _ = &mut rx => {
                        info!("连接池管理任务已停止");
                        break;
                    }
                }
            }
        });
    }

    /// 清理空闲连接
    async fn cleanup_idle_connections(
        host_id: &str,
        host_conns: &mut HostConnections,
        idle_timeout: tokio::time::Duration,
        min_connections: usize,
    ) {
        let now = chrono::Utc::now();
        let mut to_remove = Vec::new();

        // 找出需要移除的空闲连接
        for (i, conn) in host_conns.connections.iter().enumerate() {
            // 保留最小数量的连接
            if host_conns.connections.len() - to_remove.len() <= min_connections {
                break;
            }

            // 检查连接是否空闲超时
            let last_active = conn.last_active().await;
            let idle_duration = now.signed_duration_since(last_active);

            if idle_duration.num_seconds() as u64 > idle_timeout.as_secs() {
                // 连接空闲超时，标记移除
                to_remove.push(i);
                debug!(
                    "主机 {} 的连接 {} 空闲超时，将被移除 (空闲时间: {}秒)",
                    host_id,
                    i,
                    idle_duration.num_seconds()
                );
            }
        }

        // 移除空闲连接
        for &index in to_remove.iter().rev() {
            let conn = host_conns.connections.remove(index);
            let _ = conn.disconnect().await;
        }

        if !to_remove.is_empty() {
            info!(
                "主机 {} 清理了 {} 个空闲连接，剩余 {} 个连接",
                host_id,
                to_remove.len(),
                host_conns.connections.len()
            );
        }
    }

    /// 自动扩容（如果需要）
    async fn scale_up_if_needed(
        host_id: &str,
        host_conns: &mut HostConnections,
        max_connections: usize,
        transport_config: Arc<TransportConfig>,
    ) {
        // 如果已达到最大连接数，不扩容
        if host_conns.connections.len() >= max_connections {
            return;
        }

        // 检查是否所有连接都在高负载下
        let mut high_load_count = 0;
        for conn in &host_conns.connections {
            let metrics = conn.metrics();
            let active_uses = metrics.active_uses().await;

            // 如果活跃使用数 > 5，认为是高负载（这个阈值可以配置）
            if active_uses > 5 {
                high_load_count += 1;
            }
        }

        // 如果超过 80% 的连接处于高负载，则扩容
        let high_load_ratio = high_load_count as f64 / host_conns.connections.len() as f64;
        if high_load_ratio > 0.8 {
            // 扩容：添加一个新连接
            let host_info = host_conns.connections[0].host_info().clone();
            let new_conn = HostConnection::with_config(host_info, transport_config);

            // 异步建立连接
            let new_conn_arc = Arc::new(new_conn);
            let new_conn_clone = Arc::clone(&new_conn_arc);
            tokio::spawn(async move {
                if let Err(e) = new_conn_clone.connect().await {
                    warn!("新连接建立失败: {}", e);
                }
            });

            host_conns.connections.push(new_conn_arc);

            info!(
                "主机 {} 自动扩容：添加新连接 (当前连接数: {}，高负载率: {:.2})",
                host_id,
                host_conns.connections.len(),
                high_load_ratio
            );
        }
    }

    /// 停止管理任务
    #[allow(dead_code)]
    async fn stop_management_task(&self) {
        let mut shutdown_guard = self.management_shutdown.lock().await;
        if let Some(tx) = shutdown_guard.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        // 尝试停止管理任务
        // 注意：由于 Drop 是同步的，我们不能直接 await
        // 这里只是尽力而为，实际清理会在任务自然结束时完成
        let shutdown = Arc::clone(&self.management_shutdown);
        tokio::spawn(async move {
            let mut guard = shutdown.lock().await;
            if let Some(tx) = guard.take() {
                let _ = tx.send(());
            }
        });
    }
}

/// 连接池统计信息
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub selection_strategy: SelectionStrategy,
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_active_uses: u32,
}

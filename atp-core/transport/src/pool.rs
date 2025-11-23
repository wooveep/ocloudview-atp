//! 连接池管理

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::{HostConnection, HostInfo, PoolConfig, Result, TransportError};

/// 连接池
pub struct ConnectionPool {
    /// 主机连接映射
    hosts: Arc<RwLock<HashMap<String, Vec<HostConnection>>>>,

    /// 连接池配置
    config: PoolConfig,
}

impl ConnectionPool {
    /// 创建新的连接池
    pub fn new(config: PoolConfig) -> Self {
        Self {
            hosts: Arc::new(RwLock::new(HashMap::new())),
            config,
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
        for i in 0..self.config.min_connections_per_host {
            let conn = HostConnection::new(host_info.clone());
            // TODO: 在后台异步建立连接
            connections.push(conn);
        }

        hosts.insert(host_id, connections);

        Ok(())
    }

    /// 移除主机
    pub async fn remove_host(&self, host_id: &str) -> Result<()> {
        info!("从连接池移除主机: {}", host_id);

        let mut hosts = self.hosts.write().await;

        if let Some(connections) = hosts.remove(host_id) {
            // 断开所有连接
            for conn in connections {
                let _ = conn.disconnect().await;
            }
            Ok(())
        } else {
            Err(TransportError::HostNotFound(host_id.to_string()))
        }
    }

    /// 获取连接
    pub async fn get_connection(&self, host_id: &str) -> Result<&HostConnection> {
        let hosts = self.hosts.read().await;

        let connections = hosts
            .get(host_id)
            .ok_or_else(|| TransportError::HostNotFound(host_id.to_string()))?;

        // 简单的轮询策略：返回第一个可用连接
        // TODO: 实现更智能的连接选择策略
        connections
            .first()
            .ok_or(TransportError::PoolExhausted)
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
            .map(|conns| conns.len())
            .ok_or_else(|| TransportError::HostNotFound(host_id.to_string()))
    }
}

// TODO: 实现连接池的自动扩缩容和健康检查

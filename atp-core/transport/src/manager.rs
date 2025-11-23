//! 传输管理器

use std::sync::Arc;
use tracing::{info, warn};

use crate::{ConnectionPool, HostInfo, Result, TransportConfig};

/// 传输管理器
///
/// 负责管理所有主机的连接池和并发执行
pub struct TransportManager {
    /// 连接池
    pool: Arc<ConnectionPool>,

    /// 配置
    config: TransportConfig,
}

impl TransportManager {
    /// 创建新的传输管理器
    pub fn new(config: TransportConfig) -> Self {
        let pool = Arc::new(ConnectionPool::new(config.pool.clone()));

        Self { pool, config }
    }

    /// 创建默认配置的传输管理器
    pub fn default() -> Self {
        Self::new(TransportConfig::default())
    }

    /// 添加主机
    pub async fn add_host(&self, host_info: HostInfo) -> Result<()> {
        self.pool.add_host(host_info).await
    }

    /// 移除主机
    pub async fn remove_host(&self, host_id: &str) -> Result<()> {
        self.pool.remove_host(host_id).await
    }

    /// 列出所有主机
    pub async fn list_hosts(&self) -> Vec<String> {
        self.pool.list_hosts().await
    }

    /// 获取连接池
    pub fn pool(&self) -> &Arc<ConnectionPool> {
        &self.pool
    }

    /// 获取配置
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }

    // TODO: 实现并发执行任务的方法
    // pub async fn execute_on_host<F, T>(&self, host_id: &str, task: F) -> Result<T>
    // where
    //     F: FnOnce(&HostConnection) -> Future<Output = Result<T>>,
    // {
    //     ...
    // }

    // TODO: 实现在多个主机上并发执行的方法
    // pub async fn execute_on_hosts<F, T>(&self, host_ids: &[&str], task: F) -> Vec<Result<T>>
    // {
    //     ...
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transport_manager_creation() {
        let manager = TransportManager::default();
        assert_eq!(manager.list_hosts().await.len(), 0);
    }

    #[tokio::test]
    async fn test_add_host() {
        let manager = TransportManager::default();
        let host_info = HostInfo::new("test-host", "192.168.1.100");

        // 注意：这会尝试真实连接，在没有实际主机时会失败
        // 在实际测试中需要 mock
        let result = manager.add_host(host_info).await;

        // 暂时跳过实际连接测试
        // assert!(result.is_ok());
    }
}

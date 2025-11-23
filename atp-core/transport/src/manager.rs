//! 传输管理器

use std::sync::Arc;
use std::future::Future;
use tokio::task::JoinHandle;

use crate::{ConnectionPool, ConnectionPoolStats, HostConnection, HostInfo, Result, TransportConfig, TransportError};

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

    /// 在指定主机上执行任务
    ///
    /// # 示例
    /// ```ignore
    /// let result = manager.execute_on_host("host1", |conn| async move {
    ///     // 使用连接执行操作
    ///     Ok(())
    /// }).await?;
    /// ```
    pub async fn execute_on_host<F, Fut, T>(&self, host_id: &str, task: F) -> Result<T>
    where
        F: FnOnce(Arc<HostConnection>) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let conn = self.pool.get_connection(host_id).await?;
        task(conn).await
    }

    /// 在多个主机上并发执行任务
    ///
    /// # 示例
    /// ```ignore
    /// let results = manager.execute_on_hosts(
    ///     &["host1", "host2"],
    ///     |conn| async move {
    ///         // 使用连接执行操作
    ///         Ok(())
    ///     }
    /// ).await;
    /// ```
    pub async fn execute_on_hosts<F, Fut, T>(
        &self,
        host_ids: &[&str],
        task: F,
    ) -> Vec<Result<T>>
    where
        F: Fn(Arc<HostConnection>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T>> + Send,
        T: Send + 'static,
    {
        let task = Arc::new(task);
        let mut handles = Vec::new();

        for &host_id in host_ids {
            let pool = Arc::clone(&self.pool);
            let task = Arc::clone(&task);
            let host_id = host_id.to_string();

            let handle: JoinHandle<Result<T>> = tokio::spawn(async move {
                let conn = pool.get_connection(&host_id).await?;
                task(conn).await
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(TransportError::ConnectionFailed(e.to_string()))),
            }
        }

        results
    }

    /// 在所有主机上并发执行任务
    ///
    /// # 示例
    /// ```ignore
    /// let results = manager.execute_on_all_hosts(|conn| async move {
    ///     // 使用连接执行操作
    ///     Ok(())
    /// }).await;
    /// ```
    pub async fn execute_on_all_hosts<F, Fut, T>(&self, task: F) -> Vec<(String, Result<T>)>
    where
        F: Fn(Arc<HostConnection>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T>> + Send,
        T: Send + 'static,
    {
        let host_ids = self.list_hosts().await;
        let task = Arc::new(task);
        let mut handles = Vec::new();

        for host_id in host_ids {
            let pool = Arc::clone(&self.pool);
            let task = Arc::clone(&task);
            let host_id_clone = host_id.clone();

            let handle: JoinHandle<(String, Result<T>)> = tokio::spawn(async move {
                let result = match pool.get_connection(&host_id_clone).await {
                    Ok(conn) => task(conn).await,
                    Err(e) => Err(e),
                };
                (host_id_clone, result)
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    // 如果 join 失败，记录错误
                    results.push((
                        "unknown".to_string(),
                        Err(TransportError::ConnectionFailed(e.to_string())),
                    ));
                }
            }
        }

        results
    }

    /// 获取所有主机的连接池统计信息
    pub async fn stats(&self) -> std::collections::HashMap<String, ConnectionPoolStats> {
        self.pool.stats().await
    }

    /// 获取指定主机的连接数
    pub async fn connection_count(&self, host_id: &str) -> Result<usize> {
        self.pool.connection_count(host_id).await
    }

    /// 获取指定主机的活跃连接数
    pub async fn active_connection_count(&self, host_id: &str) -> Result<usize> {
        self.pool.active_connection_count(host_id).await
    }
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

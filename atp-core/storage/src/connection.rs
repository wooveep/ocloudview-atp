use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use tracing::{debug, info};

use crate::error::{Result, StorageError};

/// 存储管理器 - 负责数据库连接和迁移
pub struct StorageManager {
    pool: SqlitePool,
}

impl StorageManager {
    /// 创建新的存储管理器
    ///
    /// # 参数
    /// - `db_path`: 数据库文件路径
    ///
    /// # 示例
    /// ```no_run
    /// # use atp_storage::StorageManager;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = StorageManager::new("~/.config/atp/data.db").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(db_path: &str) -> Result<Self> {
        // 展开用户目录
        let expanded_path = shellexpand::tilde(db_path);
        let path = Path::new(expanded_path.as_ref());

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                StorageError::ConnectionError(format!("Failed to create database directory: {}", e))
            })?;
        }

        info!("Connecting to database at: {}", path.display());

        // 创建连接池
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite://{}", path.display()))
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        let manager = Self { pool };

        // 运行迁移
        manager.run_migrations().await?;

        Ok(manager)
    }

    /// 创建内存数据库(用于测试)
    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        let manager = Self { pool };
        manager.run_migrations().await?;

        Ok(manager)
    }

    /// 运行数据库迁移
    async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");

        // 读取迁移脚本
        let migration_sql = include_str!("../migrations/001_initial.sql");

        // 执行迁移
        sqlx::query(migration_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::MigrationError(e.to_string()))?;

        debug!("Database migrations completed successfully");

        Ok(())
    }

    /// 获取数据库连接池
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// 关闭数据库连接
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_database() {
        let storage = StorageManager::new_in_memory().await.unwrap();
        storage.health_check().await.unwrap();
    }

    #[tokio::test]
    async fn test_migrations() {
        let storage = StorageManager::new_in_memory().await.unwrap();

        // 验证表是否创建
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test_reports'",
        )
        .fetch_one(storage.pool())
        .await
        .unwrap();

        assert_eq!(result.0, 1, "test_reports table should exist");
    }
}

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
        // 使用 ?mode=rwc 确保数据库文件不存在时自动创建
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite://{}?mode=rwc", path.display()))
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        let manager = Self { pool };

        // 运行迁移
        manager.run_all_migrations().await?;

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
        manager.run_all_migrations().await?;

        Ok(manager)
    }

    /// 运行所有数据库迁移
    pub async fn run_all_migrations(&self) -> Result<usize> {
        info!("Running database migrations");

        // 使用 sqlx 的 migrate! 宏嵌入并运行迁移
        // 路径是相对于 Cargo.toml 的
        let migrator = sqlx::migrate!("./migrations");
        
        // 获取待应用的迁移数量（用于报告）
        // 注意：这里无法直接获取数量，migrator.run() 返回 Result<()>
        // 我们先运行迁移
        migrator
            .run(&self.pool)
            .await
            .map_err(|e: sqlx::migrate::MigrateError| StorageError::MigrationError(e.to_string()))?;

        debug!("Database migrations completed successfully");

        // 返回一个虚拟的成功数，因为 run() 不返回数量
        // 在实际场景中，我们可以查询 _sqlx_migrations 表来获取更多信息
        // 但为了简单起见，这里返回 1 表示成功
        Ok(1)
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

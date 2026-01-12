//! 资源池仓储
//!
//! 提供资源池数据的 CRUD 操作，对标 VDI pool 表

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::debug;

use crate::error::{Result, StorageError};
use crate::models::PoolRecord;

/// 资源池仓储
pub struct PoolRepository {
    pool: SqlitePool,
}

impl PoolRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 插入或更新资源池记录
    pub async fn upsert(&self, record: &PoolRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO pools (id, name, status, vmc_id, cpu_over, memory_over, arch, create_time, update_time, cached_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                status = excluded.status,
                vmc_id = excluded.vmc_id,
                cpu_over = excluded.cpu_over,
                memory_over = excluded.memory_over,
                arch = excluded.arch,
                create_time = excluded.create_time,
                update_time = excluded.update_time,
                cached_at = excluded.cached_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.status)
        .bind(&record.vmc_id)
        .bind(&record.cpu_over)
        .bind(&record.memory_over)
        .bind(&record.arch)
        .bind(&record.create_time)
        .bind(&record.update_time)
        .bind(&record.cached_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        debug!("Upserted pool: {}", record.id);
        Ok(())
    }

    /// 批量插入或更新资源池
    pub async fn upsert_batch(&self, records: &[PoolRecord]) -> Result<usize> {
        let mut count = 0;
        for record in records {
            self.upsert(record).await?;
            count += 1;
        }
        debug!("Upserted {} pools", count);
        Ok(count)
    }

    /// 根据 ID 获取资源池
    pub async fn get_by_id(&self, id: &str) -> Result<Option<PoolRecord>> {
        let record = sqlx::query_as::<_, PoolRecord>("SELECT * FROM pools WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 获取所有资源池
    pub async fn list_all(&self) -> Result<Vec<PoolRecord>> {
        let records = sqlx::query_as::<_, PoolRecord>("SELECT * FROM pools ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 删除资源池
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM pools WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }

    /// 清除过期缓存
    pub async fn clear_stale(&self, max_age_secs: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs);
        let result = sqlx::query("DELETE FROM pools WHERE cached_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        debug!("Cleared {} stale pool records", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// 清除所有记录
    pub async fn clear_all(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM pools")
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected())
    }
}

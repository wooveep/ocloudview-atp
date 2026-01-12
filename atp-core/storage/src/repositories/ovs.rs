//! OVS 网络仓储
//!
//! 提供 OVS 网络数据的 CRUD 操作，对标 VDI ovs 表

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::debug;

use crate::error::{Result, StorageError};
use crate::models::OvsRecord;

/// OVS 网络仓储
pub struct OvsRepository {
    pool: SqlitePool,
}

impl OvsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 插入或更新 OVS 记录
    pub async fn upsert(&self, record: &OvsRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO ovs (id, name, vmc_id, remark, create_time, update_time, cached_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                vmc_id = excluded.vmc_id,
                remark = excluded.remark,
                create_time = excluded.create_time,
                update_time = excluded.update_time,
                cached_at = excluded.cached_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.vmc_id)
        .bind(&record.remark)
        .bind(&record.create_time)
        .bind(&record.update_time)
        .bind(&record.cached_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        debug!("Upserted ovs: {}", record.id);
        Ok(())
    }

    /// 批量插入或更新 OVS
    pub async fn upsert_batch(&self, records: &[OvsRecord]) -> Result<usize> {
        let mut count = 0;
        for record in records {
            self.upsert(record).await?;
            count += 1;
        }
        debug!("Upserted {} ovs records", count);
        Ok(count)
    }

    /// 根据 ID 获取 OVS
    pub async fn get_by_id(&self, id: &str) -> Result<Option<OvsRecord>> {
        let record = sqlx::query_as::<_, OvsRecord>("SELECT * FROM ovs WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 获取所有 OVS
    pub async fn list_all(&self) -> Result<Vec<OvsRecord>> {
        let records = sqlx::query_as::<_, OvsRecord>("SELECT * FROM ovs ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 删除 OVS
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM ovs WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }

    /// 清除过期缓存
    pub async fn clear_stale(&self, max_age_secs: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs);
        let result = sqlx::query("DELETE FROM ovs WHERE cached_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        debug!("Cleared {} stale ovs records", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// 清除所有记录
    pub async fn clear_all(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM ovs")
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected())
    }
}

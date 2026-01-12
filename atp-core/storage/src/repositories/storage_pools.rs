//! 存储池仓储
//!
//! 提供存储池数据的 CRUD 操作，对标 VDI storage_pool 表

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::debug;

use crate::error::{Result, StorageError};
use crate::models::StoragePoolRecord;

/// 存储池仓储
pub struct StoragePoolRepository {
    pool: SqlitePool,
}

impl StoragePoolRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 插入或更新存储池记录
    pub async fn upsert(&self, record: &StoragePoolRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO storage_pools (
                id, name, nfs_ip, nfs_path, status, pool_type, path, vmc_id, pool_id, host_id,
                is_share, is_iso, remark, source_host_name, source_name, create_time, update_time, cached_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                nfs_ip = excluded.nfs_ip,
                nfs_path = excluded.nfs_path,
                status = excluded.status,
                pool_type = excluded.pool_type,
                path = excluded.path,
                vmc_id = excluded.vmc_id,
                pool_id = excluded.pool_id,
                host_id = excluded.host_id,
                is_share = excluded.is_share,
                is_iso = excluded.is_iso,
                remark = excluded.remark,
                source_host_name = excluded.source_host_name,
                source_name = excluded.source_name,
                create_time = excluded.create_time,
                update_time = excluded.update_time,
                cached_at = excluded.cached_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.nfs_ip)
        .bind(&record.nfs_path)
        .bind(&record.status)
        .bind(&record.pool_type)
        .bind(&record.path)
        .bind(&record.vmc_id)
        .bind(&record.pool_id)
        .bind(&record.host_id)
        .bind(&record.is_share)
        .bind(&record.is_iso)
        .bind(&record.remark)
        .bind(&record.source_host_name)
        .bind(&record.source_name)
        .bind(&record.create_time)
        .bind(&record.update_time)
        .bind(&record.cached_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        debug!("Upserted storage_pool: {}", record.id);
        Ok(())
    }

    /// 批量插入或更新存储池
    pub async fn upsert_batch(&self, records: &[StoragePoolRecord]) -> Result<usize> {
        let mut count = 0;
        for record in records {
            self.upsert(record).await?;
            count += 1;
        }
        debug!("Upserted {} storage_pool records", count);
        Ok(count)
    }

    /// 根据 ID 获取存储池
    pub async fn get_by_id(&self, id: &str) -> Result<Option<StoragePoolRecord>> {
        let record =
            sqlx::query_as::<_, StoragePoolRecord>("SELECT * FROM storage_pools WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 根据名称获取存储池
    pub async fn get_by_name(&self, name: &str) -> Result<Option<StoragePoolRecord>> {
        let record =
            sqlx::query_as::<_, StoragePoolRecord>("SELECT * FROM storage_pools WHERE name = ?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await
                .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 根据存储池类型查询
    pub async fn find_by_type(&self, pool_type: &str) -> Result<Vec<StoragePoolRecord>> {
        let records = sqlx::query_as::<_, StoragePoolRecord>(
            "SELECT * FROM storage_pools WHERE pool_type = ? ORDER BY name",
        )
        .bind(pool_type)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 根据主机 ID 查询存储池
    pub async fn find_by_host_id(&self, host_id: &str) -> Result<Vec<StoragePoolRecord>> {
        let records = sqlx::query_as::<_, StoragePoolRecord>(
            "SELECT * FROM storage_pools WHERE host_id = ? ORDER BY name",
        )
        .bind(host_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 获取所有存储池
    pub async fn list_all(&self) -> Result<Vec<StoragePoolRecord>> {
        let records =
            sqlx::query_as::<_, StoragePoolRecord>("SELECT * FROM storage_pools ORDER BY name")
                .fetch_all(&self.pool)
                .await
                .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 获取共享存储池
    pub async fn list_shared(&self) -> Result<Vec<StoragePoolRecord>> {
        let records = sqlx::query_as::<_, StoragePoolRecord>(
            "SELECT * FROM storage_pools WHERE is_share = 1 ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 删除存储池
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM storage_pools WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }

    /// 清除过期缓存
    pub async fn clear_stale(&self, max_age_secs: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs);
        let result = sqlx::query("DELETE FROM storage_pools WHERE cached_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        debug!(
            "Cleared {} stale storage_pool records",
            result.rows_affected()
        );
        Ok(result.rows_affected())
    }

    /// 清除所有记录
    pub async fn clear_all(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM storage_pools")
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected())
    }
}

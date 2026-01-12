//! 存储卷仓储
//!
//! 提供存储卷数据的 CRUD 操作，对标 VDI storage_volume 表

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::debug;

use crate::error::{Result, StorageError};
use crate::models::StorageVolumeRecord;

/// 存储卷仓储
pub struct StorageVolumeRepository {
    pool: SqlitePool,
}

impl StorageVolumeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 插入或更新存储卷记录
    pub async fn upsert(&self, record: &StorageVolumeRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO storage_volumes (
                id, name, filename, storage_pool_id, domain_id, is_start_disk, size, domains,
                is_recycle, read_iops_sec, write_iops_sec, read_bytes_sec, write_bytes_sec,
                bus_type, create_time, update_time, cached_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                filename = excluded.filename,
                storage_pool_id = excluded.storage_pool_id,
                domain_id = excluded.domain_id,
                is_start_disk = excluded.is_start_disk,
                size = excluded.size,
                domains = excluded.domains,
                is_recycle = excluded.is_recycle,
                read_iops_sec = excluded.read_iops_sec,
                write_iops_sec = excluded.write_iops_sec,
                read_bytes_sec = excluded.read_bytes_sec,
                write_bytes_sec = excluded.write_bytes_sec,
                bus_type = excluded.bus_type,
                create_time = excluded.create_time,
                update_time = excluded.update_time,
                cached_at = excluded.cached_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.filename)
        .bind(&record.storage_pool_id)
        .bind(&record.domain_id)
        .bind(&record.is_start_disk)
        .bind(&record.size)
        .bind(&record.domains)
        .bind(&record.is_recycle)
        .bind(&record.read_iops_sec)
        .bind(&record.write_iops_sec)
        .bind(&record.read_bytes_sec)
        .bind(&record.write_bytes_sec)
        .bind(&record.bus_type)
        .bind(&record.create_time)
        .bind(&record.update_time)
        .bind(&record.cached_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        debug!("Upserted storage_volume: {}", record.id);
        Ok(())
    }

    /// 批量插入或更新存储卷
    pub async fn upsert_batch(&self, records: &[StorageVolumeRecord]) -> Result<usize> {
        let mut count = 0;
        for record in records {
            self.upsert(record).await?;
            count += 1;
        }
        debug!("Upserted {} storage_volume records", count);
        Ok(count)
    }

    /// 根据 ID 获取存储卷
    pub async fn get_by_id(&self, id: &str) -> Result<Option<StorageVolumeRecord>> {
        let record =
            sqlx::query_as::<_, StorageVolumeRecord>("SELECT * FROM storage_volumes WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 根据文件名查询存储卷
    pub async fn get_by_filename(&self, filename: &str) -> Result<Option<StorageVolumeRecord>> {
        let record = sqlx::query_as::<_, StorageVolumeRecord>(
            "SELECT * FROM storage_volumes WHERE filename = ?",
        )
        .bind(filename)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 根据存储池 ID 查询存储卷
    pub async fn find_by_storage_pool_id(
        &self,
        storage_pool_id: &str,
    ) -> Result<Vec<StorageVolumeRecord>> {
        let records = sqlx::query_as::<_, StorageVolumeRecord>(
            "SELECT * FROM storage_volumes WHERE storage_pool_id = ? ORDER BY name",
        )
        .bind(storage_pool_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 根据虚拟机 ID 查询存储卷
    pub async fn find_by_domain_id(&self, domain_id: &str) -> Result<Vec<StorageVolumeRecord>> {
        let records = sqlx::query_as::<_, StorageVolumeRecord>(
            "SELECT * FROM storage_volumes WHERE domain_id = ? ORDER BY name",
        )
        .bind(domain_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 获取所有存储卷
    pub async fn list_all(&self) -> Result<Vec<StorageVolumeRecord>> {
        let records =
            sqlx::query_as::<_, StorageVolumeRecord>("SELECT * FROM storage_volumes ORDER BY name")
                .fetch_all(&self.pool)
                .await
                .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 获取启动盘
    pub async fn list_boot_disks(&self) -> Result<Vec<StorageVolumeRecord>> {
        let records = sqlx::query_as::<_, StorageVolumeRecord>(
            "SELECT * FROM storage_volumes WHERE is_start_disk = 1 ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 删除存储卷
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM storage_volumes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }

    /// 清除过期缓存
    pub async fn clear_stale(&self, max_age_secs: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs);
        let result = sqlx::query("DELETE FROM storage_volumes WHERE cached_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        debug!(
            "Cleared {} stale storage_volume records",
            result.rows_affected()
        );
        Ok(result.rows_affected())
    }

    /// 清除所有记录
    pub async fn clear_all(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM storage_volumes")
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected())
    }
}

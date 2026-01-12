//! 级联端口组仓储
//!
//! 提供级联端口组数据的 CRUD 操作，对标 VDI cascad_port_group 表

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::debug;

use crate::error::{Result, StorageError};
use crate::models::CascadPortGroupRecord;

/// 级联端口组仓储
pub struct CascadPortGroupRepository {
    pool: SqlitePool,
}

impl CascadPortGroupRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 插入或更新级联端口组记录
    pub async fn upsert(&self, record: &CascadPortGroupRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO cascad_port_groups (id, host_id, physical_nic, ovs_id, create_time, update_time, cached_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                host_id = excluded.host_id,
                physical_nic = excluded.physical_nic,
                ovs_id = excluded.ovs_id,
                create_time = excluded.create_time,
                update_time = excluded.update_time,
                cached_at = excluded.cached_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.host_id)
        .bind(&record.physical_nic)
        .bind(&record.ovs_id)
        .bind(&record.create_time)
        .bind(&record.update_time)
        .bind(&record.cached_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        debug!("Upserted cascad_port_group: {}", record.id);
        Ok(())
    }

    /// 批量插入或更新级联端口组
    pub async fn upsert_batch(&self, records: &[CascadPortGroupRecord]) -> Result<usize> {
        let mut count = 0;
        for record in records {
            self.upsert(record).await?;
            count += 1;
        }
        debug!("Upserted {} cascad_port_group records", count);
        Ok(count)
    }

    /// 根据 ID 获取级联端口组
    pub async fn get_by_id(&self, id: &str) -> Result<Option<CascadPortGroupRecord>> {
        let record = sqlx::query_as::<_, CascadPortGroupRecord>(
            "SELECT * FROM cascad_port_groups WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 根据主机 ID 获取级联端口组
    pub async fn find_by_host_id(&self, host_id: &str) -> Result<Vec<CascadPortGroupRecord>> {
        let records = sqlx::query_as::<_, CascadPortGroupRecord>(
            "SELECT * FROM cascad_port_groups WHERE host_id = ?",
        )
        .bind(host_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 根据 OVS ID 获取级联端口组
    pub async fn find_by_ovs_id(&self, ovs_id: &str) -> Result<Vec<CascadPortGroupRecord>> {
        let records = sqlx::query_as::<_, CascadPortGroupRecord>(
            "SELECT * FROM cascad_port_groups WHERE ovs_id = ?",
        )
        .bind(ovs_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 获取所有级联端口组
    pub async fn list_all(&self) -> Result<Vec<CascadPortGroupRecord>> {
        let records =
            sqlx::query_as::<_, CascadPortGroupRecord>("SELECT * FROM cascad_port_groups")
                .fetch_all(&self.pool)
                .await
                .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 删除级联端口组
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM cascad_port_groups WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }

    /// 清除过期缓存
    pub async fn clear_stale(&self, max_age_secs: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs);
        let result = sqlx::query("DELETE FROM cascad_port_groups WHERE cached_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        debug!(
            "Cleared {} stale cascad_port_group records",
            result.rows_affected()
        );
        Ok(result.rows_affected())
    }

    /// 清除所有记录
    pub async fn clear_all(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM cascad_port_groups")
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected())
    }
}

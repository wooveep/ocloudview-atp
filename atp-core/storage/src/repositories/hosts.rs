use chrono::Utc;
use sqlx::SqlitePool;
use tracing::debug;

use crate::error::{Result, StorageError};
use crate::models::HostRecord;

/// 主机仓储
pub struct HostRepository {
    pool: SqlitePool,
}

impl HostRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 插入或更新主机记录（支持 VDI 扩展字段）
    pub async fn upsert(&self, record: &HostRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO hosts (
                id, host, uri, tags, metadata, ssh_username, ssh_password, ssh_port, ssh_key_path,
                created_at, updated_at, ip_v6, status, pool_id, vmc_id, manufacturer, model,
                cpu, cpu_size, memory, physical_memory, domain_limit, extranet_ip, extranet_ip_v6,
                arch, domain_cap_xml, qemu_version, libvirt_version, cached_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                host = excluded.host,
                uri = excluded.uri,
                tags = excluded.tags,
                metadata = excluded.metadata,
                ssh_username = COALESCE(hosts.ssh_username, excluded.ssh_username),
                ssh_password = COALESCE(hosts.ssh_password, excluded.ssh_password),
                ssh_port = COALESCE(hosts.ssh_port, excluded.ssh_port),
                ssh_key_path = COALESCE(hosts.ssh_key_path, excluded.ssh_key_path),
                updated_at = excluded.updated_at,
                ip_v6 = excluded.ip_v6,
                status = excluded.status,
                pool_id = excluded.pool_id,
                vmc_id = excluded.vmc_id,
                manufacturer = excluded.manufacturer,
                model = excluded.model,
                cpu = excluded.cpu,
                cpu_size = excluded.cpu_size,
                memory = excluded.memory,
                physical_memory = excluded.physical_memory,
                domain_limit = excluded.domain_limit,
                extranet_ip = excluded.extranet_ip,
                extranet_ip_v6 = excluded.extranet_ip_v6,
                arch = excluded.arch,
                domain_cap_xml = excluded.domain_cap_xml,
                qemu_version = excluded.qemu_version,
                libvirt_version = excluded.libvirt_version,
                cached_at = excluded.cached_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.host)
        .bind(&record.uri)
        .bind(&record.tags)
        .bind(&record.metadata)
        .bind(&record.ssh_username)
        .bind(&record.ssh_password)
        .bind(&record.ssh_port)
        .bind(&record.ssh_key_path)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .bind(&record.ip_v6)
        .bind(&record.status)
        .bind(&record.pool_id)
        .bind(&record.vmc_id)
        .bind(&record.manufacturer)
        .bind(&record.model)
        .bind(&record.cpu)
        .bind(&record.cpu_size)
        .bind(&record.memory)
        .bind(&record.physical_memory)
        .bind(&record.domain_limit)
        .bind(&record.extranet_ip)
        .bind(&record.extranet_ip_v6)
        .bind(&record.arch)
        .bind(&record.domain_cap_xml)
        .bind(&record.qemu_version)
        .bind(&record.libvirt_version)
        .bind(&record.cached_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        debug!("Upserted host: {}", record.id);
        Ok(())
    }

    /// 更新主机 SSH 配置
    pub async fn update_ssh(
        &self,
        id: &str,
        ssh_username: Option<&str>,
        ssh_password: Option<&str>,
        ssh_port: Option<i32>,
        ssh_key_path: Option<&str>,
    ) -> Result<bool> {
        let now = Utc::now();
        let result = sqlx::query(
            r#"
            UPDATE hosts SET
                ssh_username = COALESCE(?, ssh_username),
                ssh_password = COALESCE(?, ssh_password),
                ssh_port = COALESCE(?, ssh_port),
                ssh_key_path = COALESCE(?, ssh_key_path),
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(ssh_username)
        .bind(ssh_password)
        .bind(ssh_port)
        .bind(ssh_key_path)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        debug!("Updated SSH config for host: {}", id);
        Ok(result.rows_affected() > 0)
    }

    /// 根据 ID 获取主机
    pub async fn get_by_id(&self, id: &str) -> Result<Option<HostRecord>> {
        let record = sqlx::query_as::<_, HostRecord>("SELECT * FROM hosts WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 根据 IP 获取主机
    pub async fn get_by_ip(&self, ip: &str) -> Result<Option<HostRecord>> {
        let record = sqlx::query_as::<_, HostRecord>("SELECT * FROM hosts WHERE host = ?")
            .bind(ip)
            .fetch_optional(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 获取所有主机
    pub async fn list_all(&self) -> Result<Vec<HostRecord>> {
        let records = sqlx::query_as::<_, HostRecord>("SELECT * FROM hosts ORDER BY id")
            .fetch_all(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 根据资源池 ID 获取主机
    pub async fn find_by_pool_id(&self, pool_id: &str) -> Result<Vec<HostRecord>> {
        let records = sqlx::query_as::<_, HostRecord>(
            "SELECT * FROM hosts WHERE pool_id = ? ORDER BY id",
        )
        .bind(pool_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 根据状态获取主机
    pub async fn find_by_status(&self, status: i32) -> Result<Vec<HostRecord>> {
        let records = sqlx::query_as::<_, HostRecord>(
            "SELECT * FROM hosts WHERE status = ? ORDER BY id",
        )
        .bind(status)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 删除主机
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM hosts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }

    /// 清除过期缓存（保护手动添加的记录，即 cached_at 为 NULL 的记录）
    pub async fn clear_stale(&self, max_age_secs: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs);
        let result = sqlx::query("DELETE FROM hosts WHERE cached_at IS NOT NULL AND cached_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        debug!("Cleared {} stale host records", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// 批量插入或更新主机
    pub async fn upsert_batch(&self, records: &[HostRecord]) -> Result<usize> {
        let mut count = 0;
        for record in records {
            self.upsert(record).await?;
            count += 1;
        }
        debug!("Upserted {} host records", count);
        Ok(count)
    }
}

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::debug;

use crate::error::{Result, StorageError};
use crate::models::{DomainHostMappingRecord, HostRecord};

/// 主机仓储
pub struct HostRepository {
    pool: SqlitePool,
}

impl HostRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 插入或更新主机记录
    pub async fn upsert(&self, record: &HostRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO hosts (id, host, uri, tags, metadata, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                host = excluded.host,
                uri = excluded.uri,
                tags = excluded.tags,
                metadata = excluded.metadata,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.host)
        .bind(&record.uri)
        .bind(&record.tags)
        .bind(&record.metadata)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        debug!("Upserted host: {}", record.id);
        Ok(())
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

    /// 获取所有主机
    pub async fn list_all(&self) -> Result<Vec<HostRecord>> {
        let records = sqlx::query_as::<_, HostRecord>("SELECT * FROM hosts ORDER BY id")
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
}

/// 虚拟机-主机映射仓储
pub struct DomainHostMappingRepository {
    pool: SqlitePool,
}

impl DomainHostMappingRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 插入或更新虚拟机-主机映射
    pub async fn upsert(&self, record: &DomainHostMappingRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO domain_host_mappings (domain_name, host_id, host_ip, host_name, os_type, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(domain_name) DO UPDATE SET
                host_id = excluded.host_id,
                host_ip = excluded.host_ip,
                host_name = excluded.host_name,
                os_type = excluded.os_type,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&record.domain_name)
        .bind(&record.host_id)
        .bind(&record.host_ip)
        .bind(&record.host_name)
        .bind(&record.os_type)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        debug!(
            "Upserted domain-host mapping: {} -> {} (os: {:?})",
            record.domain_name, record.host_id, record.os_type
        );
        Ok(())
    }

    /// 批量插入或更新虚拟机-主机映射
    pub async fn upsert_batch(&self, records: &[DomainHostMappingRecord]) -> Result<()> {
        for record in records {
            self.upsert(record).await?;
        }
        Ok(())
    }

    /// 根据虚拟机名称获取映射
    pub async fn get_by_domain(
        &self,
        domain_name: &str,
    ) -> Result<Option<DomainHostMappingRecord>> {
        let record = sqlx::query_as::<_, DomainHostMappingRecord>(
            "SELECT * FROM domain_host_mappings WHERE domain_name = ?",
        )
        .bind(domain_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 获取主机上的所有虚拟机
    pub async fn get_by_host(&self, host_id: &str) -> Result<Vec<DomainHostMappingRecord>> {
        let records = sqlx::query_as::<_, DomainHostMappingRecord>(
            "SELECT * FROM domain_host_mappings WHERE host_id = ?",
        )
        .bind(host_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 获取所有映射
    pub async fn list_all(&self) -> Result<Vec<DomainHostMappingRecord>> {
        let records = sqlx::query_as::<_, DomainHostMappingRecord>(
            "SELECT * FROM domain_host_mappings ORDER BY domain_name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 删除虚拟机映射
    pub async fn delete(&self, domain_name: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM domain_host_mappings WHERE domain_name = ?")
            .bind(domain_name)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }

    /// 清除所有映射（用于全量更新前）
    pub async fn clear_all(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM domain_host_mappings")
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageManager;

    #[tokio::test]
    async fn test_host_repository() {
        let storage = StorageManager::new_in_memory().await.unwrap();
        let repo = HostRepository::new(storage.pool().clone());

        let host = HostRecord {
            id: "test-host".to_string(),
            host: "192.168.1.100".to_string(),
            uri: "qemu+tcp://192.168.1.100/system".to_string(),
            tags: Some(r#"["prod"]"#.to_string()),
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Upsert
        repo.upsert(&host).await.unwrap();

        // Get by ID
        let found = repo.get_by_id("test-host").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().host, "192.168.1.100");

        // List all
        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        let deleted = repo.delete("test-host").await.unwrap();
        assert!(deleted);

        let found = repo.get_by_id("test-host").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_domain_host_mapping_repository() {
        let storage = StorageManager::new_in_memory().await.unwrap();
        let host_repo = HostRepository::new(storage.pool().clone());
        let mapping_repo = DomainHostMappingRepository::new(storage.pool().clone());

        // First create a host
        let host = HostRecord {
            id: "host1".to_string(),
            host: "192.168.1.100".to_string(),
            uri: "qemu+tcp://192.168.1.100/system".to_string(),
            tags: None,
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        host_repo.upsert(&host).await.unwrap();

        // Create mapping
        let mapping = DomainHostMappingRecord {
            domain_name: "win10_lyt001".to_string(),
            host_id: "host1".to_string(),
            host_ip: "192.168.1.100".to_string(),
            host_name: Some("compute-node-1".to_string()),
            os_type: Some("win10-64".to_string()),
            updated_at: Utc::now(),
        };

        mapping_repo.upsert(&mapping).await.unwrap();

        // Get by domain
        let found = mapping_repo.get_by_domain("win10_lyt001").await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.host_id, "host1");
        assert_eq!(found.host_ip, "192.168.1.100");

        // Get by host
        let mappings = mapping_repo.get_by_host("host1").await.unwrap();
        assert_eq!(mappings.len(), 1);
    }
}

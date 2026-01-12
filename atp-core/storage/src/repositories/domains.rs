//! 虚拟机仓储
//!
//! 提供虚拟机数据的 CRUD 操作，对标 VDI domain 表

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::debug;

use crate::error::{Result, StorageError};
use crate::models::DomainRecord;

/// 虚拟机仓储
pub struct DomainRepository {
    pool: SqlitePool,
}

impl DomainRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 插入或更新虚拟机记录
    pub async fn upsert(&self, record: &DomainRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO domains (
                id, name, is_model, status, is_connected, vmc_id, pool_id, host_id,
                last_successful_host_id, cpu, memory, iso_path, is_clone_domain, clone_type,
                mother_id, snapshot_count, freeze, last_freeze_time, command, os_name,
                os_edition, system_type, mainboard, bootloader, working_group, desktop_pool_id,
                user_id, remark, connect_time, disconnect_time, os_type, soundcard_type,
                domain_xml, affinity_ip, sockets, cores, threads, original_ip, original_mac,
                is_recycle, disable_alpha, graphics_card_num, independ_disk_cnt, mouse_mode,
                domain_fake, host_bios_enable, host_model_enable, nested_virtual, admin_id,
                admin_name, allow_monitor, agent_version, gpu_type, auto_join_domain,
                vgpu_type, keyboard_bus, mouse_bus, keep_alive, create_time, update_time, cached_at
            )
            VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            )
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                is_model = excluded.is_model,
                status = excluded.status,
                is_connected = excluded.is_connected,
                vmc_id = excluded.vmc_id,
                pool_id = excluded.pool_id,
                host_id = excluded.host_id,
                last_successful_host_id = excluded.last_successful_host_id,
                cpu = excluded.cpu,
                memory = excluded.memory,
                iso_path = excluded.iso_path,
                is_clone_domain = excluded.is_clone_domain,
                clone_type = excluded.clone_type,
                mother_id = excluded.mother_id,
                snapshot_count = excluded.snapshot_count,
                freeze = excluded.freeze,
                last_freeze_time = excluded.last_freeze_time,
                command = excluded.command,
                os_name = excluded.os_name,
                os_edition = excluded.os_edition,
                system_type = excluded.system_type,
                mainboard = excluded.mainboard,
                bootloader = excluded.bootloader,
                working_group = excluded.working_group,
                desktop_pool_id = excluded.desktop_pool_id,
                user_id = excluded.user_id,
                remark = excluded.remark,
                connect_time = excluded.connect_time,
                disconnect_time = excluded.disconnect_time,
                os_type = excluded.os_type,
                soundcard_type = excluded.soundcard_type,
                domain_xml = excluded.domain_xml,
                affinity_ip = excluded.affinity_ip,
                sockets = excluded.sockets,
                cores = excluded.cores,
                threads = excluded.threads,
                original_ip = excluded.original_ip,
                original_mac = excluded.original_mac,
                is_recycle = excluded.is_recycle,
                disable_alpha = excluded.disable_alpha,
                graphics_card_num = excluded.graphics_card_num,
                independ_disk_cnt = excluded.independ_disk_cnt,
                mouse_mode = excluded.mouse_mode,
                domain_fake = excluded.domain_fake,
                host_bios_enable = excluded.host_bios_enable,
                host_model_enable = excluded.host_model_enable,
                nested_virtual = excluded.nested_virtual,
                admin_id = excluded.admin_id,
                admin_name = excluded.admin_name,
                allow_monitor = excluded.allow_monitor,
                agent_version = excluded.agent_version,
                gpu_type = excluded.gpu_type,
                auto_join_domain = excluded.auto_join_domain,
                vgpu_type = excluded.vgpu_type,
                keyboard_bus = excluded.keyboard_bus,
                mouse_bus = excluded.mouse_bus,
                keep_alive = excluded.keep_alive,
                create_time = excluded.create_time,
                update_time = excluded.update_time,
                cached_at = excluded.cached_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.is_model)
        .bind(&record.status)
        .bind(&record.is_connected)
        .bind(&record.vmc_id)
        .bind(&record.pool_id)
        .bind(&record.host_id)
        .bind(&record.last_successful_host_id)
        .bind(&record.cpu)
        .bind(&record.memory)
        .bind(&record.iso_path)
        .bind(&record.is_clone_domain)
        .bind(&record.clone_type)
        .bind(&record.mother_id)
        .bind(&record.snapshot_count)
        .bind(&record.freeze)
        .bind(&record.last_freeze_time)
        .bind(&record.command)
        .bind(&record.os_name)
        .bind(&record.os_edition)
        .bind(&record.system_type)
        .bind(&record.mainboard)
        .bind(&record.bootloader)
        .bind(&record.working_group)
        .bind(&record.desktop_pool_id)
        .bind(&record.user_id)
        .bind(&record.remark)
        .bind(&record.connect_time)
        .bind(&record.disconnect_time)
        .bind(&record.os_type)
        .bind(&record.soundcard_type)
        .bind(&record.domain_xml)
        .bind(&record.affinity_ip)
        .bind(&record.sockets)
        .bind(&record.cores)
        .bind(&record.threads)
        .bind(&record.original_ip)
        .bind(&record.original_mac)
        .bind(&record.is_recycle)
        .bind(&record.disable_alpha)
        .bind(&record.graphics_card_num)
        .bind(&record.independ_disk_cnt)
        .bind(&record.mouse_mode)
        .bind(&record.domain_fake)
        .bind(&record.host_bios_enable)
        .bind(&record.host_model_enable)
        .bind(&record.nested_virtual)
        .bind(&record.admin_id)
        .bind(&record.admin_name)
        .bind(&record.allow_monitor)
        .bind(&record.agent_version)
        .bind(&record.gpu_type)
        .bind(&record.auto_join_domain)
        .bind(&record.vgpu_type)
        .bind(&record.keyboard_bus)
        .bind(&record.mouse_bus)
        .bind(&record.keep_alive)
        .bind(&record.create_time)
        .bind(&record.update_time)
        .bind(&record.cached_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        debug!("Upserted domain: {}", record.id);
        Ok(())
    }

    /// 批量插入或更新虚拟机
    pub async fn upsert_batch(&self, records: &[DomainRecord]) -> Result<usize> {
        let mut count = 0;
        for record in records {
            self.upsert(record).await?;
            count += 1;
        }
        debug!("Upserted {} domains", count);
        Ok(count)
    }

    /// 根据 ID 获取虚拟机
    pub async fn get_by_id(&self, id: &str) -> Result<Option<DomainRecord>> {
        let record = sqlx::query_as::<_, DomainRecord>("SELECT * FROM domains WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 根据名称获取虚拟机
    pub async fn get_by_name(&self, name: &str) -> Result<Option<DomainRecord>> {
        let record = sqlx::query_as::<_, DomainRecord>("SELECT * FROM domains WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(record)
    }

    /// 根据名称模糊查询虚拟机
    pub async fn find_by_name_pattern(&self, pattern: &str) -> Result<Vec<DomainRecord>> {
        let like_pattern = format!("%{}%", pattern);
        let records = sqlx::query_as::<_, DomainRecord>(
            "SELECT * FROM domains WHERE name LIKE ? ORDER BY name",
        )
        .bind(&like_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 根据主机 ID 查询虚拟机
    pub async fn find_by_host_id(&self, host_id: &str) -> Result<Vec<DomainRecord>> {
        let records = sqlx::query_as::<_, DomainRecord>(
            "SELECT * FROM domains WHERE host_id = ? ORDER BY name",
        )
        .bind(host_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 根据状态查询虚拟机
    pub async fn find_by_status(&self, status: i32) -> Result<Vec<DomainRecord>> {
        let records = sqlx::query_as::<_, DomainRecord>(
            "SELECT * FROM domains WHERE status = ? ORDER BY name",
        )
        .bind(status)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 根据用户 ID 查询虚拟机
    pub async fn find_by_user_id(&self, user_id: &str) -> Result<Vec<DomainRecord>> {
        let records = sqlx::query_as::<_, DomainRecord>(
            "SELECT * FROM domains WHERE user_id = ? ORDER BY name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 获取所有虚拟机
    pub async fn list_all(&self) -> Result<Vec<DomainRecord>> {
        let records =
            sqlx::query_as::<_, DomainRecord>("SELECT * FROM domains ORDER BY name")
                .fetch_all(&self.pool)
                .await
                .map_err(StorageError::DatabaseError)?;

        Ok(records)
    }

    /// 获取虚拟机数量
    pub async fn count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM domains")
            .fetch_one(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(row.0)
    }

    /// 删除虚拟机
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM domains WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }

    /// 清除过期缓存
    pub async fn clear_stale(&self, max_age_secs: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs);
        let result = sqlx::query("DELETE FROM domains WHERE cached_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        debug!("Cleared {} stale domain records", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// 清除所有记录
    pub async fn clear_all(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM domains")
            .execute(&self.pool)
            .await
            .map_err(StorageError::DatabaseError)?;

        Ok(result.rows_affected())
    }

    /// 获取最新缓存时间
    pub async fn get_latest_cache_time(&self) -> Result<Option<chrono::DateTime<Utc>>> {
        let row: Option<(Option<chrono::DateTime<Utc>>,)> =
            sqlx::query_as("SELECT MAX(cached_at) FROM domains")
                .fetch_optional(&self.pool)
                .await
                .map_err(StorageError::DatabaseError)?;

        Ok(row.and_then(|r| r.0))
    }
}

//! VDI 数据缓存管理器
//!
//! 提供从 VDI API 响应同步数据到本地 SQLite 数据库的功能，
//! 实现 "API 调用后更新本地表，后续函数调用本地表" 的数据流。

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::SqlitePool;
use tracing::{debug, info};

use crate::error::Result;
use crate::models::{
    CascadPortGroupRecord, DomainRecord, HostRecord, OvsRecord, PoolRecord, StoragePoolRecord,
    StorageVolumeRecord,
};
use crate::repositories::{
    CascadPortGroupRepository, DomainRepository, HostRepository, OvsRepository, PoolRepository,
    StoragePoolRepository, StorageVolumeRepository,
};
use crate::StorageManager;

/// 默认缓存过期时间（秒）= 5 分钟
pub const DEFAULT_CACHE_TTL: i64 = 300;

/// VDI 数据缓存管理器
pub struct VdiCacheManager {
    pool: SqlitePool,
    /// 缓存过期时间（秒），默认 300 秒 = 5 分钟
    cache_ttl: i64,
}

impl VdiCacheManager {
    /// 创建新的缓存管理器
    pub fn new(storage: StorageManager) -> Self {
        Self {
            pool: storage.pool().clone(),
            cache_ttl: DEFAULT_CACHE_TTL,
        }
    }

    /// 创建带自定义 TTL 的缓存管理器
    pub fn with_ttl(storage: StorageManager, cache_ttl: i64) -> Self {
        Self {
            pool: storage.pool().clone(),
            cache_ttl,
        }
    }

    /// 从数据库连接池创建缓存管理器
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self {
            pool,
            cache_ttl: DEFAULT_CACHE_TTL,
        }
    }

    /// 获取数据库连接池引用
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// 获取缓存 TTL
    pub fn cache_ttl(&self) -> i64 {
        self.cache_ttl
    }

    // ============================================
    // 主机同步
    // ============================================

    /// 从 API 响应同步主机数据到本地
    pub async fn sync_hosts(&self, hosts: &[Value]) -> Result<usize> {
        info!("Syncing {} hosts to local cache", hosts.len());
        let repo = HostRepository::new(self.pool.clone());
        let now = Utc::now();
        let mut count = 0;

        for host in hosts {
            if let Some(id) = host["id"].as_str() {
                // 尝试获取现有记录以保留 SSH 配置
                let existing: Option<HostRecord> = repo.get_by_id(id).await?;

                let record = HostRecord {
                    id: id.to_string(),
                    host: host["ip"].as_str().unwrap_or("").to_string(),
                    uri: format!("qemu+tcp://{}/system", host["ip"].as_str().unwrap_or("")),
                    tags: None,
                    metadata: Some(host.to_string()),
                    // 保留现有 SSH 配置
                    ssh_username: existing.as_ref().and_then(|e| e.ssh_username.clone()),
                    ssh_password: existing.as_ref().and_then(|e| e.ssh_password.clone()),
                    ssh_port: existing.as_ref().and_then(|e| e.ssh_port),
                    ssh_key_path: existing.as_ref().and_then(|e| e.ssh_key_path.clone()),
                    created_at: existing.as_ref().map(|e| e.created_at).unwrap_or(now),
                    updated_at: now,
                    // VDI 扩展字段
                    ip_v6: host["ipV6"].as_str().map(|s| s.to_string()),
                    status: host["status"].as_i64().map(|v| v as i32),
                    pool_id: host["poolId"].as_str().map(|s| s.to_string()),
                    vmc_id: host["vmcId"].as_str().map(|s| s.to_string()),
                    manufacturer: host["manufacturer"].as_str().map(|s| s.to_string()),
                    model: host["model"].as_str().map(|s| s.to_string()),
                    cpu: host["cpu"].as_str().map(|s| s.to_string()),
                    cpu_size: host["cpuSize"].as_i64().map(|v| v as i32),
                    memory: host["memory"].as_f64(),
                    physical_memory: host["physicalMemory"].as_f64(),
                    domain_limit: host["domainLimit"].as_i64().map(|v| v as i32),
                    extranet_ip: host["extranetIp"].as_str().map(|s| s.to_string()),
                    extranet_ip_v6: host["extranetIpV6"].as_str().map(|s| s.to_string()),
                    arch: host["arch"].as_str().map(|s| s.to_string()),
                    domain_cap_xml: host["domainCapXml"].as_str().map(|s| s.to_string()),
                    qemu_version: host["qemuVersion"].as_i64(),
                    libvirt_version: host["libvirtVersion"].as_i64(),
                    cached_at: Some(now),
                };

                repo.upsert(&record).await?;
                count += 1;
            }
        }

        info!("Synced {} hosts to local cache", count);
        Ok(count)
    }

    // ============================================
    // 资源池同步
    // ============================================

    /// 从 API 响应同步资源池数据到本地
    pub async fn sync_pools(&self, pools: &[Value]) -> Result<usize> {
        info!("Syncing {} pools to local cache", pools.len());
        let repo = PoolRepository::new(self.pool.clone());
        let now = Utc::now();
        let mut count = 0;

        for pool in pools {
            if let Some(id) = pool["id"].as_str() {
                let record = PoolRecord {
                    id: id.to_string(),
                    name: pool["name"].as_str().map(|s| s.to_string()),
                    status: pool["status"].as_i64().map(|v| v as i32),
                    vmc_id: pool["vmcId"].as_str().map(|s| s.to_string()),
                    cpu_over: pool["cpuOver"].as_i64().map(|v| v as i32),
                    memory_over: pool["memoryOver"].as_i64().map(|v| v as i32),
                    arch: pool["arch"].as_str().map(|s| s.to_string()),
                    create_time: parse_datetime(pool["createTime"].as_str()),
                    update_time: parse_datetime(pool["updateTime"].as_str()),
                    cached_at: Some(now),
                };

                repo.upsert(&record).await?;
                count += 1;
            }
        }

        info!("Synced {} pools to local cache", count);
        Ok(count)
    }

    // ============================================
    // OVS 同步
    // ============================================

    /// 从 API 响应同步 OVS 数据到本地
    pub async fn sync_ovs(&self, ovs_list: &[Value]) -> Result<usize> {
        info!("Syncing {} OVS records to local cache", ovs_list.len());
        let repo = OvsRepository::new(self.pool.clone());
        let now = Utc::now();
        let mut count = 0;

        for ovs in ovs_list {
            if let Some(id) = ovs["id"].as_str() {
                let record = OvsRecord {
                    id: id.to_string(),
                    name: ovs["name"].as_str().map(|s| s.to_string()),
                    vmc_id: ovs["vmcId"].as_str().map(|s| s.to_string()),
                    remark: ovs["remark"].as_str().map(|s| s.to_string()),
                    create_time: parse_datetime(ovs["createTime"].as_str()),
                    update_time: parse_datetime(ovs["updateTime"].as_str()),
                    cached_at: Some(now),
                };

                repo.upsert(&record).await?;
                count += 1;
            }
        }

        info!("Synced {} OVS records to local cache", count);
        Ok(count)
    }

    // ============================================
    // 级联端口组同步
    // ============================================

    /// 从 API 响应同步级联端口组数据到本地
    pub async fn sync_cascad_port_groups(&self, groups: &[Value]) -> Result<usize> {
        info!("Syncing {} cascad port groups to local cache", groups.len());
        let repo = CascadPortGroupRepository::new(self.pool.clone());
        let now = Utc::now();
        let mut count = 0;

        for group in groups {
            if let Some(id) = group["id"].as_str() {
                let record = CascadPortGroupRecord {
                    id: id.to_string(),
                    host_id: group["hostId"].as_str().map(|s| s.to_string()),
                    physical_nic: group["physicalNic"].as_str().map(|s| s.to_string()),
                    ovs_id: group["ovsId"].as_str().map(|s| s.to_string()),
                    create_time: parse_datetime(group["createTime"].as_str()),
                    update_time: parse_datetime(group["updateTime"].as_str()),
                    cached_at: Some(now),
                };

                repo.upsert(&record).await?;
                count += 1;
            }
        }

        info!("Synced {} cascad port groups to local cache", count);
        Ok(count)
    }

    // ============================================
    // 虚拟机同步
    // ============================================

    /// 从 API 响应同步虚拟机数据到本地
    pub async fn sync_domains(&self, domains: &[Value]) -> Result<usize> {
        info!("Syncing {} domains to local cache", domains.len());
        let repo = DomainRepository::new(self.pool.clone());
        let now = Utc::now();
        let mut count = 0;

        for domain in domains {
            if let Some(id) = domain["id"].as_str() {
                let record = DomainRecord {
                    id: id.to_string(),
                    name: domain["name"].as_str().map(|s| s.to_string()),
                    is_model: domain["isModel"].as_i64().map(|v| v as i32),
                    status: domain["status"].as_i64().map(|v| v as i32),
                    is_connected: domain["isConnected"].as_i64().map(|v| v as i32),
                    vmc_id: domain["vmcId"].as_str().map(|s| s.to_string()),
                    pool_id: domain["poolId"].as_str().map(|s| s.to_string()),
                    host_id: domain["hostId"].as_str().map(|s| s.to_string()),
                    last_successful_host_id: domain["lastSuccessfulHostId"]
                        .as_str()
                        .map(|s| s.to_string()),
                    cpu: domain["cpu"].as_i64().map(|v| v as i32),
                    memory: domain["memory"].as_i64().map(|v| v as i32),
                    iso_path: domain["isoPath"].as_str().map(|s| s.to_string()),
                    is_clone_domain: domain["isCloneDomain"].as_i64().map(|v| v as i32),
                    clone_type: domain["cloneType"].as_str().map(|s| s.to_string()),
                    mother_id: domain["motherId"].as_str().map(|s| s.to_string()),
                    snapshot_count: domain["snapshotCount"].as_i64().map(|v| v as i32),
                    freeze: domain["freeze"].as_i64().map(|v| v as i32),
                    last_freeze_time: parse_datetime(domain["lastFreezeTime"].as_str()),
                    command: domain["command"].as_str().map(|s| s.to_string()),
                    os_name: domain["osName"].as_str().map(|s| s.to_string()),
                    os_edition: domain["osEdition"].as_str().map(|s| s.to_string()),
                    system_type: domain["systemType"].as_str().map(|s| s.to_string()),
                    mainboard: domain["mainboard"].as_str().map(|s| s.to_string()),
                    bootloader: domain["bootloader"].as_str().map(|s| s.to_string()),
                    working_group: domain["workingGroup"].as_str().map(|s| s.to_string()),
                    desktop_pool_id: domain["desktopPoolId"].as_str().map(|s| s.to_string()),
                    user_id: domain["userId"].as_str().map(|s| s.to_string()),
                    remark: domain["remark"].as_str().map(|s| s.to_string()),
                    connect_time: parse_datetime(domain["connectTime"].as_str()),
                    disconnect_time: parse_datetime(domain["disconnectTime"].as_str()),
                    os_type: domain["osType"].as_str().map(|s| s.to_string()),
                    soundcard_type: domain["soundcardType"].as_str().map(|s| s.to_string()),
                    domain_xml: domain["domainXml"].as_str().map(|s| s.to_string()),
                    affinity_ip: domain["affinityIp"].as_str().map(|s| s.to_string()),
                    sockets: domain["sockets"].as_i64().map(|v| v as i32),
                    cores: domain["cores"].as_i64().map(|v| v as i32),
                    threads: domain["threads"].as_i64().map(|v| v as i32),
                    original_ip: domain["originalIp"].as_str().map(|s| s.to_string()),
                    original_mac: domain["originalMac"].as_str().map(|s| s.to_string()),
                    is_recycle: domain["isRecycle"].as_i64().map(|v| v as i32),
                    disable_alpha: domain["disableAlpha"].as_i64().map(|v| v as i32),
                    graphics_card_num: domain["graphicsCardNum"].as_i64().map(|v| v as i32),
                    independ_disk_cnt: domain["independDiskCnt"].as_i64().map(|v| v as i32),
                    mouse_mode: domain["mouseMode"].as_str().map(|s| s.to_string()),
                    domain_fake: domain["domainFake"].as_i64().map(|v| v as i32),
                    host_bios_enable: domain["hostBiosEnable"].as_i64().map(|v| v as i32),
                    host_model_enable: domain["hostModelEnable"].as_i64().map(|v| v as i32),
                    nested_virtual: domain["nestedVirtual"].as_i64().map(|v| v as i32),
                    admin_id: domain["adminId"].as_str().map(|s| s.to_string()),
                    admin_name: domain["adminName"].as_str().map(|s| s.to_string()),
                    allow_monitor: domain["allowMonitor"].as_i64().map(|v| v as i32),
                    agent_version: domain["agentVersion"].as_str().map(|s| s.to_string()),
                    gpu_type: domain["gpuType"].as_str().map(|s| s.to_string()),
                    auto_join_domain: domain["autoJoinDomain"].as_i64().map(|v| v as i32),
                    vgpu_type: domain["vgpuType"].as_str().map(|s| s.to_string()),
                    keyboard_bus: domain["keyboardBus"].as_str().map(|s| s.to_string()),
                    mouse_bus: domain["mouseBus"].as_str().map(|s| s.to_string()),
                    keep_alive: domain["keepAlive"].as_i64().map(|v| v as i32),
                    create_time: parse_datetime(domain["createTime"].as_str()),
                    update_time: parse_datetime(domain["updateTime"].as_str()),
                    cached_at: Some(now),
                };

                repo.upsert(&record).await?;
                count += 1;
            }
        }

        info!("Synced {} domains to local cache", count);
        Ok(count)
    }

    // ============================================
    // 存储池同步
    // ============================================

    /// 从 API 响应同步存储池数据到本地
    pub async fn sync_storage_pools(&self, pools: &[Value]) -> Result<usize> {
        info!("Syncing {} storage pools to local cache", pools.len());
        let repo = StoragePoolRepository::new(self.pool.clone());
        let now = Utc::now();
        let mut count = 0;

        for pool in pools {
            if let Some(id) = pool["id"].as_str() {
                let record = StoragePoolRecord {
                    id: id.to_string(),
                    name: pool["name"].as_str().map(|s| s.to_string()),
                    nfs_ip: pool["nfsIp"].as_str().map(|s| s.to_string()),
                    nfs_path: pool["nfsPath"].as_str().map(|s| s.to_string()),
                    status: pool["status"].as_i64().map(|v| v as i32),
                    pool_type: pool["poolType"].as_str().map(|s| s.to_string()),
                    path: pool["path"].as_str().map(|s| s.to_string()),
                    vmc_id: pool["vmcId"].as_str().map(|s| s.to_string()),
                    pool_id: pool["poolId"].as_str().map(|s| s.to_string()),
                    host_id: pool["hostId"].as_str().map(|s| s.to_string()),
                    is_share: pool["isShare"].as_i64().map(|v| v as i32),
                    is_iso: pool["isIso"].as_i64().map(|v| v as i32),
                    remark: pool["remark"].as_str().map(|s| s.to_string()),
                    source_host_name: pool["sourceHostName"].as_str().map(|s| s.to_string()),
                    source_name: pool["sourceName"].as_str().map(|s| s.to_string()),
                    create_time: parse_datetime(pool["createTime"].as_str()),
                    update_time: parse_datetime(pool["updateTime"].as_str()),
                    cached_at: Some(now),
                };

                repo.upsert(&record).await?;
                count += 1;
            }
        }

        info!("Synced {} storage pools to local cache", count);
        Ok(count)
    }

    // ============================================
    // 存储卷同步
    // ============================================

    /// 从 API 响应同步存储卷数据到本地
    pub async fn sync_storage_volumes(&self, volumes: &[Value]) -> Result<usize> {
        info!("Syncing {} storage volumes to local cache", volumes.len());
        let repo = StorageVolumeRepository::new(self.pool.clone());
        let now = Utc::now();
        let mut count = 0;

        for volume in volumes {
            if let Some(id) = volume["id"].as_str() {
                let record = StorageVolumeRecord {
                    id: id.to_string(),
                    name: volume["name"].as_str().map(|s| s.to_string()),
                    filename: volume["filename"].as_str().map(|s| s.to_string()),
                    storage_pool_id: volume["storagePoolId"].as_str().map(|s| s.to_string()),
                    domain_id: volume["domainId"].as_str().map(|s| s.to_string()),
                    is_start_disk: volume["isStartDisk"].as_i64().map(|v| v as i32),
                    size: volume["size"].as_i64(),
                    domains: volume["domains"].as_str().map(|s| s.to_string()),
                    is_recycle: volume["isRecycle"].as_i64().map(|v| v as i32),
                    read_iops_sec: volume["readIopsSec"].as_str().map(|s| s.to_string()),
                    write_iops_sec: volume["writeIopsSec"].as_str().map(|s| s.to_string()),
                    read_bytes_sec: volume["readBytesSec"].as_str().map(|s| s.to_string()),
                    write_bytes_sec: volume["writeBytesSec"].as_str().map(|s| s.to_string()),
                    bus_type: volume["busType"].as_str().map(|s| s.to_string()),
                    create_time: parse_datetime(volume["createTime"].as_str()),
                    update_time: parse_datetime(volume["updateTime"].as_str()),
                    cached_at: Some(now),
                };

                repo.upsert(&record).await?;
                count += 1;
            }
        }

        info!("Synced {} storage volumes to local cache", count);
        Ok(count)
    }

    // ============================================
    // 缓存有效性检查
    // ============================================

    /// 检查指定表的缓存是否有效
    pub async fn is_cache_valid(&self, table: &str) -> Result<bool> {
        let cutoff = Utc::now() - chrono::Duration::seconds(self.cache_ttl);

        let query = format!("SELECT COUNT(*) FROM {} WHERE cached_at >= ?", table);

        let row: (i64,) = sqlx::query_as(&query)
            .bind(&cutoff)
            .fetch_one(&self.pool)
            .await
            .map_err(crate::error::StorageError::DatabaseError)?;

        let is_valid = row.0 > 0;
        debug!(
            "Cache validity check for {}: {} (count: {})",
            table, is_valid, row.0
        );

        Ok(is_valid)
    }

    /// 检查虚拟机缓存是否有效
    pub async fn is_domains_cache_valid(&self) -> Result<bool> {
        self.is_cache_valid("domains").await
    }

    /// 检查主机缓存是否有效
    pub async fn is_hosts_cache_valid(&self) -> Result<bool> {
        self.is_cache_valid("hosts").await
    }

    // ============================================
    // 缓存清理
    // ============================================

    /// 清理所有过期缓存
    pub async fn cleanup_stale_cache(&self) -> Result<()> {
        info!("Cleaning up stale cache (TTL: {} seconds)", self.cache_ttl);

        let domain_repo = DomainRepository::new(self.pool.clone());
        let pool_repo = PoolRepository::new(self.pool.clone());
        let ovs_repo = OvsRepository::new(self.pool.clone());
        let cpg_repo = CascadPortGroupRepository::new(self.pool.clone());
        let sp_repo = StoragePoolRepository::new(self.pool.clone());
        let sv_repo = StorageVolumeRepository::new(self.pool.clone());

        let domains_cleared = domain_repo.clear_stale(self.cache_ttl).await?;
        let pools_cleared = pool_repo.clear_stale(self.cache_ttl).await?;
        let ovs_cleared = ovs_repo.clear_stale(self.cache_ttl).await?;
        let cpg_cleared = cpg_repo.clear_stale(self.cache_ttl).await?;
        let sp_cleared = sp_repo.clear_stale(self.cache_ttl).await?;
        let sv_cleared = sv_repo.clear_stale(self.cache_ttl).await?;

        info!(
            "Stale cache cleanup complete: domains={}, pools={}, ovs={}, cpg={}, sp={}, sv={}",
            domains_cleared, pools_cleared, ovs_cleared, cpg_cleared, sp_cleared, sv_cleared
        );

        Ok(())
    }
}

/// 解析日期时间字符串
fn parse_datetime(s: Option<&str>) -> Option<DateTime<Utc>> {
    s.and_then(|s| {
        // 尝试多种格式
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
            .or_else(|| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                    .map(|dt| dt.and_utc())
                    .ok()
            })
            .or_else(|| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .map(|dt| dt.and_utc())
                    .ok()
            })
    })
}

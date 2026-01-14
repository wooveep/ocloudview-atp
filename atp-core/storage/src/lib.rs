mod backup;
mod cache_manager;
mod connection;
mod error;
mod models;
mod repositories;

pub use backup::{BackupInfo, BackupManager};
pub use cache_manager::{VdiCacheManager, DEFAULT_CACHE_TTL};
pub use connection::StorageManager;
pub use error::{Result, StorageError};
pub use models::*;
pub use repositories::*;

use sqlx::SqlitePool;

/// 统一的数据访问层入口
pub struct Storage {
    _pool: SqlitePool,
    reports: ReportRepository,
    scenarios: ScenarioRepository,
    hosts: HostRepository,
    domains: DomainRepository,
    pools: PoolRepository,
    ovs: OvsRepository,
    cascad_port_groups: CascadPortGroupRepository,
    storage_pools: StoragePoolRepository,
    storage_volumes: StorageVolumeRepository,
}

impl Storage {
    /// 从 StorageManager 创建 Storage
    pub fn from_manager(manager: &StorageManager) -> Self {
        let pool = manager.pool().clone();
        Self {
            _pool: pool.clone(),
            reports: ReportRepository::new(pool.clone()),
            scenarios: ScenarioRepository::new(pool.clone()),
            hosts: HostRepository::new(pool.clone()),
            domains: DomainRepository::new(pool.clone()),
            pools: PoolRepository::new(pool.clone()),
            ovs: OvsRepository::new(pool.clone()),
            cascad_port_groups: CascadPortGroupRepository::new(pool.clone()),
            storage_pools: StoragePoolRepository::new(pool.clone()),
            storage_volumes: StorageVolumeRepository::new(pool),
        }
    }

    /// 获取报告仓储
    pub fn reports(&self) -> &ReportRepository {
        &self.reports
    }

    /// 获取场景仓储
    pub fn scenarios(&self) -> &ScenarioRepository {
        &self.scenarios
    }

    /// 获取主机仓储
    pub fn hosts(&self) -> &HostRepository {
        &self.hosts
    }

    /// 获取虚拟机仓储
    pub fn domains(&self) -> &DomainRepository {
        &self.domains
    }

    /// 获取资源池仓储
    pub fn pools(&self) -> &PoolRepository {
        &self.pools
    }

    /// 获取 OVS 仓储
    pub fn ovs(&self) -> &OvsRepository {
        &self.ovs
    }

    /// 获取级联端口组仓储
    pub fn cascad_port_groups(&self) -> &CascadPortGroupRepository {
        &self.cascad_port_groups
    }

    /// 获取存储池仓储
    pub fn storage_pools(&self) -> &StoragePoolRepository {
        &self.storage_pools
    }

    /// 获取存储卷仓储
    pub fn storage_volumes(&self) -> &StorageVolumeRepository {
        &self.storage_volumes
    }

    /// 获取数据库连接池
    pub fn pool(&self) -> &SqlitePool {
        &self._pool
    }
}

mod backup;
mod connection;
mod error;
mod models;
mod repositories;

pub use backup::{BackupInfo, BackupManager};
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
    domain_host_mappings: DomainHostMappingRepository,
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
            domain_host_mappings: DomainHostMappingRepository::new(pool),
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

    /// 获取虚拟机-主机映射仓储
    pub fn domain_host_mappings(&self) -> &DomainHostMappingRepository {
        &self.domain_host_mappings
    }
}

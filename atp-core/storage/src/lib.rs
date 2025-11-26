mod connection;
mod error;
mod models;
mod repositories;

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
}

impl Storage {
    /// 从 StorageManager 创建 Storage
    pub fn from_manager(manager: &StorageManager) -> Self {
        let pool = manager.pool().clone();
        Self {
            _pool: pool.clone(),
            reports: ReportRepository::new(pool.clone()),
            scenarios: ScenarioRepository::new(pool.clone()),
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

    // TODO: 添加其他 repository 访问器
    // pub fn hosts(&self) -> &HostRepository { ... }
    // pub fn metrics(&self) -> &MetricRepository { ... }
}

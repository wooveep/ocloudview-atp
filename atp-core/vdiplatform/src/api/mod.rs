//! VDI 平台 API 模块
//!
//! 提供完整的 VDI 平台 API 封装，包括：
//! - 虚拟机管理 (DomainApi)
//! - 桌面池管理 (DeskPoolApi)
//! - 主机管理 (HostApi)
//! - 模板管理 (ModelApi)
//! - 用户管理 (UserApi)
//! - 快照管理 (SnapshotApi)
//! - 存储管理 (StorageApi)
//! - 网络管理 (NetworkApi)
//! - 事件管理 (EventApi)
//! - 回收站管理 (RecycleApi)

pub mod domain;
pub mod desk_pool;
pub mod host;
pub mod model;
pub mod user;
pub mod snapshot;
pub mod storage;
pub mod network;
pub mod event;
pub mod recycle;

pub use domain::DomainApi;
pub use desk_pool::DeskPoolApi;
pub use host::HostApi;
pub use model::ModelApi;
pub use user::UserApi;
pub use snapshot::SnapshotApi;
pub use storage::StorageApi;
pub use network::NetworkApi;
pub use event::EventApi;
pub use recycle::RecycleApi;

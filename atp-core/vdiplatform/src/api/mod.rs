//! VDI 平台 API 模块

pub mod domain;
pub mod desk_pool;
pub mod host;
pub mod model;
pub mod user;

pub use domain::DomainApi;
pub use desk_pool::DeskPoolApi;
pub use host::HostApi;
pub use model::ModelApi;
pub use user::UserApi;

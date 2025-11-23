//! OCloudView VDI 平台测试模块
//!
//! 提供与 OCloudView 云桌面管理平台 API 交互的客户端实现。

pub mod client;
pub mod api;
pub mod models;
pub mod error;

pub use client::VdiClient;
pub use error::{VdiError, Result};

// 导出 API 模块
pub use api::{
    domain::DomainApi,
    desk_pool::DeskPoolApi,
    host::HostApi,
    model::ModelApi,
    user::UserApi,
};

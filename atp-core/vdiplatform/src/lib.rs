//! OCloudView VDI 平台测试模块
//!
//! 提供与 OCloudView 云桌面管理平台 API 交互的客户端实现。
//!
//! # 功能
//!
//! - **虚拟机管理** (`DomainApi`): 创建、克隆、启动、关闭、修改配置等
//! - **批量操作**: 批量启动/关闭/重启/删除虚拟机
//! - **模板管理** (`ModelApi`): 从模板批量创建虚拟机
//! - **桌面池管理** (`DeskPoolApi`): 桌面池 CRUD 操作
//! - **主机管理** (`HostApi`): 宿主机查询
//! - **用户管理** (`UserApi`): 用户查询
//! - **快照管理** (`SnapshotApi`): 虚拟机快照操作
//! - **存储管理** (`StorageApi`): 存储池和卷管理
//! - **网络管理** (`NetworkApi`): OVS/VLAN/网桥/网卡管理
//! - **事件管理** (`EventApi`): 异步任务跟踪
//! - **回收站管理** (`RecycleApi`): 删除恢复操作
//!
//! # 示例
//!
//! ```ignore
//! use atp_vdiplatform::{VdiClient, BatchTaskRequest, CloneDomainRequest};
//!
//! // 创建客户端
//! let client = VdiClient::new("http://vdi-server:8088")
//!     .login("admin", "password")
//!     .await?;
//!
//! // 批量启动虚拟机
//! let req = BatchTaskRequest::new(vec!["vm-1".into(), "vm-2".into()]);
//! client.domain().batch_start(req).await?;
//!
//! // 克隆虚拟机
//! let req = CloneDomainRequest::batch("clone-".into(), 5);
//! client.domain().clone("source-vm-id", req).await?;
//!
//! // 创建快照
//! client.snapshot().create("domain-id", "snapshot-name", None).await?;
//!
//! // 查询存储池
//! let pools = client.storage().list_all_pools().await?;
//! ```

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
    snapshot::SnapshotApi,
    storage::StorageApi,
    network::NetworkApi,
    event::EventApi,
    recycle::RecycleApi,
};

// 导出数据模型
pub use models::{
    // 状态码枚举
    DomainStatus, HostStatusCode,

    // 基础模型
    Domain, CreateDomainRequest, DeskPool, CreateDeskPoolRequest,
    Host, HostStatus, Model, User,
    ApiResponse, PageRequest, PageResponse,

    // 批量操作
    BatchTaskRequest, BatchTaskResponse, BatchTaskError,
    BatchDeleteRequest,

    // 克隆操作
    CloneDomainRequest, CloneDomainResponse,
    ModelCloneRequest, EventIdResponse,

    // 配置修改
    UpdateMemCpuRequest, BatchUpdateConfigRequest,

    // 网络管理
    NetworkConfigRequest, NicInfo,

    // 完整创建
    CreateDomainFullRequest, CreateDomainResponse,
    VolumeConfig, IsoConfig,
};

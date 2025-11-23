//! VDI 平台数据模型

use serde::{Deserialize, Serialize};

/// 虚拟机信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    /// 虚拟机 ID
    pub id: String,

    /// 虚拟机名称
    pub name: String,

    /// 状态
    pub status: String,

    /// 所在主机 ID
    pub host_id: String,

    /// CPU 核心数
    pub vcpu: u32,

    /// 内存大小 (MB)
    pub memory: u64,

    /// 创建时间
    pub created_at: Option<String>,
}

/// 创建虚拟机请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDomainRequest {
    /// 虚拟机名称
    pub name: String,

    /// 模板 ID
    pub template_id: String,

    /// CPU 核心数
    pub vcpu: u32,

    /// 内存大小 (MB)
    pub memory: u64,

    /// 磁盘大小 (GB)
    pub disk_size: u64,
}

/// 桌面池信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeskPool {
    /// 桌面池 ID
    pub id: String,

    /// 桌面池名称
    pub name: String,

    /// 状态
    pub status: String,

    /// 模板 ID
    pub template_id: String,

    /// 虚拟机数量
    pub vm_count: u32,

    /// 创建时间
    pub created_at: Option<String>,
}

/// 创建桌面池请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDeskPoolRequest {
    /// 桌面池名称
    pub name: String,

    /// 模板 ID
    pub template_id: String,

    /// 虚拟机数量
    pub count: u32,

    /// CPU 核心数
    pub vcpu: u32,

    /// 内存大小 (MB)
    pub memory: u64,
}

/// 主机信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    /// 主机 ID
    pub id: String,

    /// 主机 IP
    pub ip: String,

    /// 主机名称
    pub hostname: String,

    /// 状态
    pub status: String,

    /// CPU 核心数
    pub cpu_cores: u32,

    /// 总内存 (MB)
    pub total_memory: u64,

    /// 已用内存 (MB)
    pub used_memory: u64,
}

/// 主机状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostStatus {
    /// 主机 ID
    pub id: String,

    /// 状态
    pub status: String,

    /// 运行时间（秒）
    pub uptime: u64,

    /// CPU 使用率 (%)
    pub cpu_usage: f64,

    /// 内存使用率 (%)
    pub memory_usage: f64,
}

/// 模板信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// 模板 ID
    pub id: String,

    /// 模板名称
    pub name: String,

    /// 操作系统
    pub os: String,

    /// 版本
    pub version: String,

    /// 磁盘大小 (GB)
    pub disk_size: u64,
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户 ID
    pub id: String,

    /// 用户名
    pub username: String,

    /// 显示名称
    pub display_name: String,

    /// 邮箱
    pub email: Option<String>,
}

/// API 响应封装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// 响应码
    pub code: i32,

    /// 响应消息
    pub message: String,

    /// 响应数据
    pub data: Option<T>,
}

/// 分页查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRequest {
    /// 页码（从 1 开始）
    pub page: u32,

    /// 每页大小
    pub page_size: u32,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 10,
        }
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResponse<T> {
    /// 总记录数
    pub total: u64,

    /// 当前页码
    pub page: u32,

    /// 每页大小
    pub page_size: u32,

    /// 数据列表
    pub items: Vec<T>,
}

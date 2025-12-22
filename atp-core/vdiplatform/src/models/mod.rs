//! VDI 平台数据模型
//!
//! **当前数据来源**: VDI 平台 REST API (实时查询,无本地持久化)
//!
//! **问题**:
//! - 每次查询都需要调用 VDI API
//! - 无历史状态记录
//! - 离线无法查询
//! - 频繁 API 调用可能影响性能
//!
//! **TODO: 添加数据库缓存层** (优先级: 中)
//!
//! 建议实现方案:
//! 1. 创建 VmCacheManager 管理缓存
//! 2. 添加数据库表:
//!    - vm_cache: 缓存 VM 基本信息
//!    - vm_status_history: 记录状态变更历史
//! 3. 实现混合查询策略:
//!    - 优先从缓存读取 (缓存有效期如 5 分钟)
//!    - 缓存过期时从 API 查询并更新缓存
//!    - 提供强制刷新命令
//!
//! 参考实现: docs/DATA_STORAGE_ANALYSIS.md - 建议 2

use serde::{Deserialize, Serialize};

// ============================================
// 状态码定义
// ============================================

/// 虚拟机状态码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i64)]
pub enum DomainStatus {
    /// 关机
    Shutoff = 0,
    /// 运行中
    Running = 1,
    /// 挂起
    Paused = 2,
    /// 休眠
    Hibernated = 3,
    /// 操作中
    Operating = 5,
    /// 升级中
    Upgrading = 6,
    /// 未知状态
    Unknown = -1,
}

impl DomainStatus {
    /// 从 i64 状态码创建
    pub fn from_code(code: i64) -> Self {
        match code {
            0 => Self::Shutoff,
            1 => Self::Running,
            2 => Self::Paused,
            3 => Self::Hibernated,
            5 => Self::Operating,
            6 => Self::Upgrading,
            _ => Self::Unknown,
        }
    }

    /// 获取状态码
    pub fn code(&self) -> i64 {
        *self as i64
    }

    /// 获取中文显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Shutoff => "关机",
            Self::Running => "运行中",
            Self::Paused => "挂起",
            Self::Hibernated => "休眠",
            Self::Operating => "操作中",
            Self::Upgrading => "升级中",
            Self::Unknown => "未知",
        }
    }

    /// 获取带 emoji 的显示名称
    pub fn display_with_emoji(&self) -> &'static str {
        match self {
            Self::Shutoff => "关机 ⚪",
            Self::Running => "运行中 ✅",
            Self::Paused => "挂起 🟡",
            Self::Hibernated => "休眠 🌙",
            Self::Operating => "操作中 ⚙️",
            Self::Upgrading => "升级中 ⬆️",
            Self::Unknown => "未知 ⚠️",
        }
    }

    /// 检查是否可操作（非过渡状态）
    pub fn is_operable(&self) -> bool {
        matches!(self, Self::Shutoff | Self::Running | Self::Paused | Self::Hibernated)
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

impl From<i64> for DomainStatus {
    fn from(code: i64) -> Self {
        Self::from_code(code)
    }
}

impl std::fmt::Display for DomainStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// 主机状态码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i64)]
pub enum HostStatusCode {
    /// 离线
    Offline = 0,
    /// 在线
    Online = 1,
    /// 维护中
    Maintenance = 2,
    /// 未知状态
    Unknown = -1,
}

impl HostStatusCode {
    /// 从 i64 状态码创建
    pub fn from_code(code: i64) -> Self {
        match code {
            0 => Self::Offline,
            1 => Self::Online,
            2 => Self::Maintenance,
            _ => Self::Unknown,
        }
    }

    /// 获取状态码
    pub fn code(&self) -> i64 {
        *self as i64
    }

    /// 获取中文显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Offline => "离线",
            Self::Online => "在线",
            Self::Maintenance => "维护中",
            Self::Unknown => "未知",
        }
    }

    /// 获取带 emoji 的显示名称
    pub fn display_with_emoji(&self) -> &'static str {
        match self {
            Self::Offline => "离线 ❌",
            Self::Online => "在线 ✅",
            Self::Maintenance => "维护中 🔧",
            Self::Unknown => "未知 ⚠️",
        }
    }

    /// 检查是否在线
    pub fn is_online(&self) -> bool {
        matches!(self, Self::Online)
    }
}

impl From<i64> for HostStatusCode {
    fn from(code: i64) -> Self {
        Self::from_code(code)
    }
}

impl std::fmt::Display for HostStatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================
// 数据模型
// ============================================

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

// ============================================
// 批量操作相关数据结构
// ============================================

/// 批量任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTaskRequest {
    /// 虚拟机 ID 列表
    pub id_list: Vec<String>,

    /// 指定运行的主机 ID (用于批量启动)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_id: Option<String>,

    /// 强制关机标记 (1=强制, 0=不强制)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_force: Option<i32>,
}

impl BatchTaskRequest {
    /// 创建简单的批量任务请求
    pub fn new(id_list: Vec<String>) -> Self {
        Self {
            id_list,
            host_id: None,
            is_force: None,
        }
    }

    /// 设置目标主机
    pub fn with_host(mut self, host_id: String) -> Self {
        self.host_id = Some(host_id);
        self
    }

    /// 设置强制标记
    pub fn with_force(mut self, force: bool) -> Self {
        self.is_force = Some(if force { 1 } else { 0 });
        self
    }
}

/// 批量任务响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTaskResponse {
    /// 成功的虚拟机 ID 列表
    #[serde(default)]
    pub success_id_list: Vec<String>,

    /// 错误列表
    #[serde(default)]
    pub error_list: Vec<BatchTaskError>,
}

/// 批量任务错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTaskError {
    /// 虚拟机 ID
    pub id: Option<String>,

    /// 错误信息
    pub error: Option<String>,
}

/// 批量删除请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDeleteRequest {
    /// 虚拟机 ID 列表
    pub domain_id_list: Vec<String>,

    /// 是否移至回收站 (0=彻底删除, 1=移至回收站)
    pub is_recycle: i32,
}

impl BatchDeleteRequest {
    /// 创建彻底删除请求
    pub fn permanent(id_list: Vec<String>) -> Self {
        Self {
            domain_id_list: id_list,
            is_recycle: 0,
        }
    }

    /// 创建移至回收站请求
    pub fn to_recycle(id_list: Vec<String>) -> Self {
        Self {
            domain_id_list: id_list,
            is_recycle: 1,
        }
    }
}

// ============================================
// 克隆相关数据结构
// ============================================

/// 虚拟机克隆请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneDomainRequest {
    /// 克隆后的虚拟机名称
    pub name: String,

    /// 克隆数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<i32>,

    /// 存储池 ID (模板克隆时必传)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_pool_id: Option<String>,

    /// 模板克隆标志 (1=是, 0=否, 默认0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_clone: Option<i32>,
}

impl CloneDomainRequest {
    /// 创建单个克隆请求
    pub fn single(name: String) -> Self {
        Self {
            name,
            number: Some(1),
            storage_pool_id: None,
            template_clone: Some(0),
        }
    }

    /// 创建批量克隆请求
    pub fn batch(name_prefix: String, count: i32) -> Self {
        Self {
            name: name_prefix,
            number: Some(count),
            storage_pool_id: None,
            template_clone: Some(0),
        }
    }

    /// 设置存储池
    pub fn with_storage_pool(mut self, pool_id: String) -> Self {
        self.storage_pool_id = Some(pool_id);
        self
    }
}

/// 虚拟机克隆响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneDomainResponse {
    /// 克隆的虚拟机 UUID
    pub clone_uuid: Option<String>,

    /// 事件 ID (用于跟踪异步任务)
    pub event_id: Option<String>,
}

/// 从模板批量创建虚拟机请求 (链接克隆)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCloneRequest {
    /// 虚拟机名称前缀
    pub name: String,

    /// 创建数量
    pub amount: i32,

    /// 是否勾选了存储池
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check: Option<bool>,

    /// 存储池名称列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_name_list: Option<Vec<String>>,
}

impl ModelCloneRequest {
    /// 创建批量克隆请求
    pub fn new(name_prefix: String, amount: i32) -> Self {
        Self {
            name: name_prefix,
            amount,
            check: Some(false),
            storage_name_list: None,
        }
    }

    /// 设置存储池
    pub fn with_storage_pools(mut self, pools: Vec<String>) -> Self {
        self.check = Some(true);
        self.storage_name_list = Some(pools);
        self
    }
}

/// 事件 ID 响应 (用于异步任务)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventIdResponse {
    /// 事件 ID
    pub event_id: Option<String>,
}

// ============================================
// 配置修改相关数据结构
// ============================================

/// 修改 CPU/内存请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemCpuRequest {
    /// 虚拟机 ID 列表
    pub list_id: Vec<String>,

    /// CPU 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<i32>,

    /// 内存大小 (MB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<i32>,
}

impl UpdateMemCpuRequest {
    /// 创建修改请求
    pub fn new(id_list: Vec<String>) -> Self {
        Self {
            list_id: id_list,
            cpu: None,
            memory: None,
        }
    }

    /// 设置 CPU
    pub fn with_cpu(mut self, cpu: i32) -> Self {
        self.cpu = Some(cpu);
        self
    }

    /// 设置内存
    pub fn with_memory(mut self, memory: i32) -> Self {
        self.memory = Some(memory);
        self
    }
}

/// 批量修改其他配置请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateConfigRequest {
    /// 虚拟机 ID 列表
    pub id_list: Vec<String>,

    /// 虚拟伪装 (0=否, 1=是)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_fake: Option<i32>,

    /// GPU 型号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_type: Option<String>,

    /// 启用主机 BIOS (0=否, 1=是)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_bios_enable: Option<i32>,

    /// 启用主机型号 (0=否, 1=是)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_model_enable: Option<i32>,

    /// 嵌套虚拟化 (0=否, 1=是)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nested_virtual: Option<i32>,
}

impl BatchUpdateConfigRequest {
    /// 创建配置修改请求
    pub fn new(id_list: Vec<String>) -> Self {
        Self {
            id_list,
            domain_fake: None,
            gpu_type: None,
            host_bios_enable: None,
            host_model_enable: None,
            nested_virtual: None,
        }
    }
}

// ============================================
// 网络管理相关数据结构
// ============================================

/// 网络配置请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfigRequest {
    /// OVS ID
    pub ovs_id: String,

    /// VLAN ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan_id: Option<String>,

    /// MAC 地址 (修改时需要)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,

    /// 入站带宽峰值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_peak: Option<i32>,

    /// 入站带宽平均值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_average: Option<i32>,

    /// 入站带宽突发值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_burst: Option<i32>,

    /// 出站带宽峰值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_peak: Option<i32>,

    /// 出站带宽平均值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_average: Option<i32>,

    /// 出站带宽突发值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_burst: Option<i32>,
}

impl NetworkConfigRequest {
    /// 创建基本网络配置
    pub fn new(ovs_id: String) -> Self {
        Self {
            ovs_id,
            vlan_id: None,
            mac: None,
            in_peak: None,
            in_average: None,
            in_burst: None,
            out_peak: None,
            out_average: None,
            out_burst: None,
        }
    }

    /// 设置 VLAN
    pub fn with_vlan(mut self, vlan_id: String) -> Self {
        self.vlan_id = Some(vlan_id);
        self
    }

    /// 设置 MAC 地址
    pub fn with_mac(mut self, mac: String) -> Self {
        self.mac = Some(mac);
        self
    }

    /// 设置入站带宽限制
    pub fn with_inbound_limit(mut self, peak: i32, average: i32, burst: i32) -> Self {
        self.in_peak = Some(peak);
        self.in_average = Some(average);
        self.in_burst = Some(burst);
        self
    }

    /// 设置出站带宽限制
    pub fn with_outbound_limit(mut self, peak: i32, average: i32, burst: i32) -> Self {
        self.out_peak = Some(peak);
        self.out_average = Some(average);
        self.out_burst = Some(burst);
        self
    }
}

/// 网卡信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NicInfo {
    /// MAC 地址
    pub mac: Option<String>,

    /// OVS ID
    pub ovs_id: Option<String>,

    /// OVS 名称
    pub ovs_name: Option<String>,

    /// VLAN ID
    pub vlan_id: Option<String>,

    /// 网卡型号
    pub model: Option<String>,
}

// ============================================
// 完整创建虚拟机相关数据结构
// ============================================

/// 创建虚拟机完整请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainFullRequest {
    /// 虚拟机名称
    pub name: String,

    /// CPU 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<i32>,

    /// 内存大小 (MB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<i32>,

    /// CPU 核数
    pub cores: i32,

    /// CPU 线程数
    pub threads: i32,

    /// CPU 插槽数
    pub sockets: i32,

    /// 启动引导 ("bios" 或 "uefi")
    pub bootloader: String,

    /// 主板
    pub main_board: String,

    /// 是否自动加域 (0=否, 1=是)
    pub auto_join_domain: i32,

    /// 显卡数量
    pub graphics_card_num: i32,

    /// vGPU 类型
    pub vgpu_type: String,

    /// GPU 类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_type: Option<String>,

    /// 声卡类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soundcard_type: Option<String>,

    /// 系统类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_type: Option<String>,

    /// 存储池 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<String>,

    /// 数据中心 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmc_id: Option<String>,

    /// 备注
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,

    /// 网络列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_list: Option<Vec<NetworkConfigRequest>>,

    /// 磁盘列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_list: Option<Vec<VolumeConfig>>,

    /// ISO 列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso_list: Option<Vec<IsoConfig>>,
}

impl CreateDomainFullRequest {
    /// 创建基本配置的虚拟机请求
    pub fn basic(name: String, cpu: i32, memory: i32) -> Self {
        Self {
            name,
            cpu: Some(cpu),
            memory: Some(memory),
            cores: 1,
            threads: 1,
            sockets: cpu,
            bootloader: "bios".to_string(),
            main_board: "i440fx".to_string(),
            auto_join_domain: 0,
            graphics_card_num: 1,
            vgpu_type: "".to_string(),
            gpu_type: None,
            soundcard_type: None,
            os_type: None,
            pool_id: None,
            vmc_id: None,
            remark: None,
            network_list: None,
            volume_list: None,
            iso_list: None,
        }
    }

    /// 设置 UEFI 启动
    pub fn with_uefi(mut self) -> Self {
        self.bootloader = "uefi".to_string();
        self
    }

    /// 设置网络
    pub fn with_networks(mut self, networks: Vec<NetworkConfigRequest>) -> Self {
        self.network_list = Some(networks);
        self
    }

    /// 设置存储池
    pub fn with_storage_pool(mut self, pool_id: String) -> Self {
        self.pool_id = Some(pool_id);
        self
    }

    /// 设置操作系统类型
    pub fn with_os_type(mut self, os_type: String) -> Self {
        self.os_type = Some(os_type);
        self
    }
}

/// 磁盘配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeConfig {
    /// 磁盘 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// 总线类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus_type: Option<String>,

    /// 是否启动磁盘
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_start_disk: Option<i32>,

    /// 读速率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_bytes_sec: Option<String>,

    /// 读吞吐量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_iops_sec: Option<String>,

    /// 写速率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_bytes_sec: Option<String>,

    /// 写吞吐量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_iops_sec: Option<String>,
}

/// ISO 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IsoConfig {
    /// ISO 路径
    pub iso_path: String,

    /// 存储池 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_pool_id: Option<String>,
}

/// 创建虚拟机响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainResponse {
    /// 虚拟机 UUID
    pub uuid: Option<String>,
}

// ============================================
// 磁盘信息相关数据结构
// ============================================

/// 虚拟机磁盘信息（来自 VDI API /domain/{id}/disk）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    /// 磁盘 ID（卷 ID）
    pub id: String,

    /// 磁盘名称（如 win10Template）
    pub name: String,

    /// 磁盘文件名（如 b97923a3-dd90-4e24-82cf-74c25dca3ff5.qcow2）
    pub filename: String,

    /// 完整卷路径（如 /home/gluster3/xxx.qcow2）
    pub vol_full_path: String,

    /// 存储池 ID
    pub storage_pool_id: String,

    /// 所属虚拟机 ID
    pub domain_id: String,

    /// 是否为启动磁盘（1=是, 0=否）
    pub is_start_disk: i32,

    /// 磁盘大小（GB）
    pub size: u64,

    /// 是否在回收站（0=否, 1=是）
    #[serde(default)]
    pub is_recycle: i32,

    /// 总线类型（virtio/scsi/ide）
    pub bus_type: String,

    /// 存储池路径
    pub pool_path: String,

    /// 存储池名称
    pub pool_name: String,

    /// 是否共享（1=是, 0=否）
    #[serde(default)]
    pub is_share: i32,

    /// 存储池类型（gluster/nfs/dir/lvm 等）
    pub pool_type: String,

    /// 创建时间
    #[serde(default)]
    pub create_time: Option<String>,
}

impl DiskInfo {
    /// 是否为 Gluster 存储
    pub fn is_gluster(&self) -> bool {
        self.pool_type.to_lowercase() == "gluster"
    }

    /// 是否为 NFS 存储
    pub fn is_nfs(&self) -> bool {
        self.pool_type.to_lowercase() == "nfs"
    }

    /// 是否为本地存储
    pub fn is_local(&self) -> bool {
        let t = self.pool_type.to_lowercase();
        t == "dir" || t == "lvm"
    }

    /// 是否为启动盘
    pub fn is_boot_disk(&self) -> bool {
        self.is_start_disk == 1
    }

    /// 是否为共享存储
    pub fn is_shared(&self) -> bool {
        self.is_share == 1
    }

    /// 获取存储类型显示名称
    pub fn storage_type_display(&self) -> &'static str {
        match self.pool_type.to_lowercase().as_str() {
            "gluster" => "Gluster 分布式存储",
            "nfs" => "NFS 网络存储",
            "dir" => "本地文件存储",
            "lvm" => "LVM 逻辑卷",
            "share-lvm" => "共享 LVM",
            "gfs" => "GFS 文件系统",
            _ => "未知存储类型",
        }
    }
}

/// 存储类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    /// Gluster 分布式存储
    Gluster,
    /// NFS 网络存储
    Nfs,
    /// 本地文件存储
    Dir,
    /// LVM 逻辑卷
    Lvm,
    /// 共享 LVM
    ShareLvm,
    /// GFS 文件系统
    Gfs,
    /// 未知类型
    Unknown,
}

impl StorageType {
    /// 从字符串解析存储类型
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "gluster" => Self::Gluster,
            "nfs" => Self::Nfs,
            "dir" => Self::Dir,
            "lvm" => Self::Lvm,
            "share-lvm" | "sharelvm" => Self::ShareLvm,
            "gfs" => Self::Gfs,
            _ => Self::Unknown,
        }
    }

    /// 是否支持分布式文件定位
    pub fn supports_file_location(&self) -> bool {
        matches!(self, Self::Gluster)
    }

    /// 获取显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Gluster => "Gluster",
            Self::Nfs => "NFS",
            Self::Dir => "本地",
            Self::Lvm => "LVM",
            Self::ShareLvm => "共享LVM",
            Self::Gfs => "GFS",
            Self::Unknown => "未知",
        }
    }
}

impl std::fmt::Display for StorageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

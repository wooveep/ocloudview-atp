use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 测试报告数据库模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TestReportRecord {
    pub id: i64,
    pub scenario_name: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub total_steps: i32,
    pub success_count: i32,
    pub failed_count: i32,
    pub skipped_count: i32,
    pub passed: bool,
    pub tags: Option<String>, // JSON array
    pub created_at: DateTime<Utc>,
}

/// 执行步骤数据库模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExecutionStepRecord {
    pub id: i64,
    pub report_id: i64,
    pub step_index: i32,
    pub description: String,
    pub status: String, // 'Success', 'Failed', 'Skipped'
    pub error: Option<String>,
    pub duration_ms: Option<i64>,
    pub output: Option<String>,
}

/// 场景数据库模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScenarioRecord {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub definition: String,   // JSON/YAML
    pub tags: Option<String>, // JSON array
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 主机配置数据库模型（对标 VDI host 表 22 字段 + SSH 配置）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct HostRecord {
    pub id: String,
    pub host: String,
    pub uri: String,
    pub tags: Option<String>,     // JSON array
    pub metadata: Option<String>, // JSON object
    // SSH 配置
    pub ssh_username: Option<String>, // 默认 root
    pub ssh_password: Option<String>, // 密码认证
    pub ssh_port: Option<i32>,        // 默认 22
    pub ssh_key_path: Option<String>, // 密钥路径
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // VDI host 表扩展字段
    pub ip_v6: Option<String>,
    pub status: Option<i32>,
    pub pool_id: Option<String>,
    pub vmc_id: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub cpu: Option<String>,
    pub cpu_size: Option<i32>,
    pub memory: Option<f64>,
    pub physical_memory: Option<f64>,
    pub domain_limit: Option<i32>,
    pub extranet_ip: Option<String>,
    pub extranet_ip_v6: Option<String>,
    pub arch: Option<String>,
    pub domain_cap_xml: Option<String>,
    pub qemu_version: Option<i64>,
    pub libvirt_version: Option<i64>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// 性能指标数据库模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConnectionMetricRecord {
    pub id: i64,
    pub host_id: String,
    pub timestamp: DateTime<Utc>,
    pub total_connections: i32,
    pub active_connections: i32,
    pub total_requests: i64,
    pub total_errors: i64,
    pub avg_response_time: Option<f64>,
}

/// 资源池数据库模型（对标 VDI pool 表）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PoolRecord {
    pub id: String,
    pub name: Option<String>,
    pub status: Option<i32>,
    pub vmc_id: Option<String>,
    pub cpu_over: Option<i32>,
    pub memory_over: Option<i32>,
    pub arch: Option<String>,
    pub create_time: Option<DateTime<Utc>>,
    pub update_time: Option<DateTime<Utc>>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// OVS 网络数据库模型（对标 VDI ovs 表）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OvsRecord {
    pub id: String,
    pub name: Option<String>,
    pub vmc_id: Option<String>,
    pub remark: Option<String>,
    pub create_time: Option<DateTime<Utc>>,
    pub update_time: Option<DateTime<Utc>>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// 级联端口组数据库模型（对标 VDI cascad_port_group 表）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CascadPortGroupRecord {
    pub id: String,
    pub host_id: Option<String>,
    pub physical_nic: Option<String>,
    pub ovs_id: Option<String>,
    pub create_time: Option<DateTime<Utc>>,
    pub update_time: Option<DateTime<Utc>>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// 虚拟机数据库模型（对标 VDI domain 表 60 字段）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DomainRecord {
    pub id: String,
    pub name: Option<String>,
    pub is_model: Option<i32>,
    pub status: Option<i32>,
    pub is_connected: Option<i32>,
    pub vmc_id: Option<String>,
    pub pool_id: Option<String>,
    pub host_id: Option<String>,
    pub last_successful_host_id: Option<String>,
    pub cpu: Option<i32>,
    pub memory: Option<i32>,
    pub iso_path: Option<String>,
    pub is_clone_domain: Option<i32>,
    pub clone_type: Option<String>,
    pub mother_id: Option<String>,
    pub snapshot_count: Option<i32>,
    pub freeze: Option<i32>,
    pub last_freeze_time: Option<DateTime<Utc>>,
    pub command: Option<String>,
    pub os_name: Option<String>,
    pub os_edition: Option<String>,
    pub system_type: Option<String>,
    pub mainboard: Option<String>,
    pub bootloader: Option<String>,
    pub working_group: Option<String>,
    pub desktop_pool_id: Option<String>,
    pub user_id: Option<String>,
    pub remark: Option<String>,
    pub connect_time: Option<DateTime<Utc>>,
    pub disconnect_time: Option<DateTime<Utc>>,
    pub os_type: Option<String>,
    pub soundcard_type: Option<String>,
    pub domain_xml: Option<String>,
    pub affinity_ip: Option<String>,
    pub sockets: Option<i32>,
    pub cores: Option<i32>,
    pub threads: Option<i32>,
    pub original_ip: Option<String>,
    pub original_mac: Option<String>,
    pub is_recycle: Option<i32>,
    pub disable_alpha: Option<i32>,
    pub graphics_card_num: Option<i32>,
    pub independ_disk_cnt: Option<i32>,
    pub mouse_mode: Option<String>,
    pub domain_fake: Option<i32>,
    pub host_bios_enable: Option<i32>,
    pub host_model_enable: Option<i32>,
    pub nested_virtual: Option<i32>,
    pub admin_id: Option<String>,
    pub admin_name: Option<String>,
    pub allow_monitor: Option<i32>,
    pub agent_version: Option<String>,
    pub gpu_type: Option<String>,
    pub auto_join_domain: Option<i32>,
    pub vgpu_type: Option<String>,
    pub keyboard_bus: Option<String>,
    pub mouse_bus: Option<String>,
    pub keep_alive: Option<i32>,
    pub create_time: Option<DateTime<Utc>>,
    pub update_time: Option<DateTime<Utc>>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// 存储池数据库模型（对标 VDI storage_pool 表）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StoragePoolRecord {
    pub id: String,
    pub name: Option<String>,
    pub nfs_ip: Option<String>,
    pub nfs_path: Option<String>,
    pub status: Option<i32>,
    pub pool_type: Option<String>,
    pub path: Option<String>,
    pub vmc_id: Option<String>,
    pub pool_id: Option<String>,
    pub host_id: Option<String>,
    pub is_share: Option<i32>,
    pub is_iso: Option<i32>,
    pub remark: Option<String>,
    pub source_host_name: Option<String>,
    pub source_name: Option<String>,
    pub create_time: Option<DateTime<Utc>>,
    pub update_time: Option<DateTime<Utc>>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// 存储卷数据库模型（对标 VDI storage_volume 表）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StorageVolumeRecord {
    pub id: String,
    pub name: Option<String>,
    pub filename: Option<String>,
    pub storage_pool_id: Option<String>,
    pub domain_id: Option<String>,
    pub is_start_disk: Option<i32>,
    pub size: Option<i64>,
    pub domains: Option<String>,
    pub is_recycle: Option<i32>,
    pub read_iops_sec: Option<String>,
    pub write_iops_sec: Option<String>,
    pub read_bytes_sec: Option<String>,
    pub write_bytes_sec: Option<String>,
    pub bus_type: Option<String>,
    pub create_time: Option<DateTime<Utc>>,
    pub update_time: Option<DateTime<Utc>>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// 报告查询过滤器
#[derive(Debug, Default, Clone)]
pub struct ReportFilter {
    pub scenario_name: Option<String>,
    pub passed: Option<bool>,
    pub start_time_from: Option<DateTime<Utc>>,
    pub start_time_to: Option<DateTime<Utc>>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 场景查询过滤器
#[derive(Debug, Default, Clone)]
pub struct ScenarioFilter {
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

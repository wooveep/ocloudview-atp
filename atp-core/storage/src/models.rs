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

/// 主机配置数据库模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct HostRecord {
    pub id: String,
    pub host: String,
    pub uri: String,
    pub tags: Option<String>,     // JSON array
    pub metadata: Option<String>, // JSON object
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

/// 虚拟机-主机映射数据库模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DomainHostMappingRecord {
    pub domain_name: String,
    pub host_id: String,
    pub host_ip: String,
    pub host_name: Option<String>,
    pub os_type: Option<String>, // 操作系统类型: win10-64, linux, kylin, uos 等
    pub updated_at: DateTime<Utc>,
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

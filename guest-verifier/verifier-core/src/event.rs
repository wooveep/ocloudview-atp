//! 事件和结果定义

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub event_id: String,
    pub verified: bool,
    pub timestamp: i64,
    pub latency_ms: u64,
    pub details: serde_json::Value,
}

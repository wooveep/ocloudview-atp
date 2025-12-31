//! ATP 通用类型定义
//!
//! 此 crate 包含 verification-server 和 guest-verifier 之间共享的类型。

use serde::{Deserialize, Serialize};

/// 验证事件（服务端下发的验证请求）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// 事件类型 (keyboard, mouse, command)
    pub event_type: String,

    /// 事件数据
    pub data: serde_json::Value,

    /// 时间戳
    pub timestamp: i64,
}

/// 原始输入事件（客户端上报的输入事件）
///
/// 用于输入上报模式：客户端监听底层输入事件并实时转发给服务端。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawInputEvent {
    /// 消息类型标识（固定为 "raw_input_event"）
    #[serde(default = "default_raw_input_event_type")]
    pub message_type: String,

    /// 事件类型 ("keyboard" 或 "mouse")
    pub event_type: String,

    /// 按键代码或鼠标按钮
    pub code: u16,

    /// 事件值（1=按下，0=释放，2=重复）
    pub value: i32,

    /// 按键名称或鼠标操作（人类可读）
    pub name: String,

    /// 时间戳（毫秒）
    pub timestamp: i64,
}

fn default_raw_input_event_type() -> String {
    "raw_input_event".to_string()
}

impl Default for RawInputEvent {
    fn default() -> Self {
        Self {
            message_type: "raw_input_event".to_string(),
            event_type: String::new(),
            code: 0,
            value: 0,
            name: String::new(),
            timestamp: 0,
        }
    }
}

/// 已验证的输入事件摘要（用于调试和日志记录）
///
/// 包含验证后的输入事件信息，用于服务端记录和调试。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedInputEvent {
    /// 消息类型标识（固定为 "verified_input_event"）
    #[serde(default = "default_verified_input_event_type")]
    pub message_type: String,

    /// 事件类型 ("keyboard" 或 "mouse")
    pub event_type: String,

    /// 按键名称或鼠标操作
    pub key_or_action: String,

    /// 时间戳
    pub timestamp: i64,

    /// 额外数据（包含验证结果信息）
    pub details: serde_json::Value,
}

fn default_verified_input_event_type() -> String {
    "verified_input_event".to_string()
}

impl Default for VerifiedInputEvent {
    fn default() -> Self {
        Self {
            message_type: "verified_input_event".to_string(),
            event_type: String::new(),
            key_or_action: String::new(),
            timestamp: 0,
            details: serde_json::Value::Null,
        }
    }
}

// 为了向后兼容，保留 InputEvent 别名
#[deprecated(since = "0.1.0", note = "请使用 VerifiedInputEvent")]
pub type InputEvent = VerifiedInputEvent;

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    /// 消息类型标识（固定为 "verify_result"）
    #[serde(default = "default_verify_result_type")]
    pub message_type: String,

    /// 事件 ID (用于匹配)
    pub event_id: String,

    /// 是否验证成功
    pub verified: bool,

    /// 时间戳
    pub timestamp: i64,

    /// 延迟 (毫秒)
    pub latency_ms: u64,

    /// 详细信息
    pub details: serde_json::Value,
}

fn default_verify_result_type() -> String {
    "verify_result".to_string()
}

impl Default for VerifyResult {
    fn default() -> Self {
        Self {
            message_type: "verify_result".to_string(),
            event_id: String::new(),
            verified: false,
            timestamp: 0,
            latency_ms: 0,
            details: serde_json::Value::Null,
        }
    }
}

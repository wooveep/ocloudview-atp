use serde::{Deserialize, Serialize};

/// QMP 指令结构定义
#[derive(Debug, Serialize)]
pub struct QmpCommand<'a> {
    pub execute: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<&'a str>,
}

/// QMP 响应结构定义
#[derive(Debug, Deserialize)]
pub struct QmpResponse {
    #[serde(rename = "return")]
    pub ret: Option<serde_json::Value>,
    pub error: Option<QmpError>,
    pub event: Option<String>,
}

/// QMP 错误信息
#[derive(Debug, Deserialize)]
pub struct QmpError {
    #[serde(rename = "class")]
    pub error_class: String,
    pub desc: String,
}

/// QMP 问候信息 (Greeting)
#[derive(Debug, Deserialize)]
pub struct QmpGreeting {
    #[serde(rename = "QMP")]
    pub qmp: QmpInfo,
}

#[derive(Debug, Deserialize)]
pub struct QmpInfo {
    pub version: QmpVersion,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct QmpVersion {
    pub qemu: QemuVersion,
    pub package: String,
}

#[derive(Debug, Deserialize)]
pub struct QemuVersion {
    pub major: u32,
    pub minor: u32,
    pub micro: u32,
}

/// 按键事件类型
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyEventType {
    Down,
    Up,
    Press,
}

/// QMP 按键定义
#[derive(Debug, Serialize)]
pub struct QmpKey {
    #[serde(rename = "type")]
    pub key_type: String,  // "qcode"
    pub data: String,      // QKeyCode 字符串，如 "a", "shift", "ret"
}

impl QmpKey {
    pub fn new_qcode(qcode: &str) -> Self {
        Self {
            key_type: "qcode".to_string(),
            data: qcode.to_string(),
        }
    }
}

/// send-key 命令参数
#[derive(Debug, Serialize)]
pub struct SendKeyArgs {
    pub keys: Vec<QmpKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "hold-time")]
    pub hold_time: Option<u32>,  // 单位：毫秒
}

/// input-send-event 命令参数
#[derive(Debug, Serialize)]
pub struct InputSendEventArgs {
    pub events: Vec<InputEvent>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InputEvent {
    Key {
        data: KeyEventData,
    },
}

#[derive(Debug, Serialize)]
pub struct KeyEventData {
    pub key: QmpKey,
    pub down: bool,
}

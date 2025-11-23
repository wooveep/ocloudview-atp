#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

use serde::{Deserialize, Serialize};

/// 键盘事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    /// 事件类型 (press, release)
    pub event_type: EventType,

    /// 扫描码
    pub scancode: u32,

    /// 键码
    pub keycode: u32,

    /// 键名
    pub key_name: Option<String>,

    /// 时间戳（毫秒）
    pub timestamp: u64,

    /// 修饰键状态
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    Press,
    Release,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

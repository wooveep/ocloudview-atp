//! 测试场景定义

use serde::{Deserialize, Serialize};

/// 测试场景
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// 场景名称
    pub name: String,

    /// 场景描述
    pub description: Option<String>,

    /// 测试步骤
    pub steps: Vec<ScenarioStep>,

    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
}

/// 测试步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStep {
    /// 步骤名称
    pub name: Option<String>,

    /// 动作类型
    pub action: Action,

    /// 是否需要验证
    #[serde(default)]
    pub verify: bool,

    /// 超时时间（秒）
    pub timeout: Option<u64>,
}

/// 动作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// 发送按键
    SendKey { key: String },

    /// 发送文本
    SendText { text: String },

    /// 鼠标点击
    MouseClick { x: i32, y: i32, button: String },

    /// 执行命令
    ExecCommand { command: String },

    /// 等待
    Wait { duration: u64 },

    /// 自定义动作
    Custom { data: serde_json::Value },
}

// TODO: 实现场景加载和验证

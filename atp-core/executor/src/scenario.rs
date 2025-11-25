//! 测试场景定义

use serde::{Deserialize, Serialize};
use std::path::Path;

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

impl Scenario {
    /// 从 YAML 文件加载场景
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml_str(&content)
    }

    /// 从 YAML 字符串加载场景
    pub fn from_yaml_str(yaml: &str) -> crate::Result<Self> {
        serde_yaml::from_str(yaml)
            .map_err(|e| crate::ExecutorError::SerdeError(e.to_string()))
    }

    /// 从 JSON 文件加载场景
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json_str(&content)
    }

    /// 从 JSON 字符串加载场景
    pub fn from_json_str(json: &str) -> crate::Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| crate::ExecutorError::SerdeError(e.to_string()))
    }

    /// 导出为 YAML
    pub fn to_yaml(&self) -> crate::Result<String> {
        serde_yaml::to_string(self)
            .map_err(|e| crate::ExecutorError::SerdeError(e.to_string()))
    }

    /// 导出为 JSON
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::ExecutorError::SerdeError(e.to_string()))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_from_yaml() {
        let yaml = r#"
name: "测试场景"
description: "这是一个测试场景"
tags: ["test", "demo"]
steps:
  - name: "发送按键"
    action:
      type: send_key
      key: "a"
  - name: "等待"
    action:
      type: wait
      duration: 2
"#;
        let scenario = Scenario::from_yaml_str(yaml).unwrap();
        assert_eq!(scenario.name, "测试场景");
        assert_eq!(scenario.steps.len(), 2);
    }

    #[test]
    fn test_scenario_to_yaml() {
        let scenario = Scenario {
            name: "测试场景".to_string(),
            description: Some("描述".to_string()),
            tags: vec!["test".to_string()],
            steps: vec![
                ScenarioStep {
                    name: Some("发送按键".to_string()),
                    action: Action::SendKey { key: "a".to_string() },
                    verify: false,
                    timeout: None,
                },
            ],
        };

        let yaml = scenario.to_yaml().unwrap();
        assert!(yaml.contains("测试场景"));
    }
}

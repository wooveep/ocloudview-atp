//! 测试场景定义

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 测试场景
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
    /// 场景名称
    pub name: String,

    /// 场景描述
    pub description: Option<String>,

    /// 测试步骤
    pub steps: Vec<TestStep>,

    /// 场景标签
    #[serde(default)]
    pub tags: Vec<String>,

    /// 超时时间（秒）
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl TestScenario {
    /// 从 YAML 文件加载场景
    pub fn from_yaml(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_yaml::from_str(&content)
            .map_err(|e| crate::OrchestratorError::ScenarioParseError(e.to_string()))
    }

    /// 从 JSON 字符串加载场景
    pub fn from_json(json: &str) -> crate::Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| crate::OrchestratorError::ScenarioParseError(e.to_string()))
    }
}

/// 测试步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TestStep {
    /// VDI 平台操作
    VdiAction {
        #[serde(flatten)]
        action: VdiAction,

        /// 是否捕获输出
        #[serde(default)]
        capture_output: Option<String>,
    },

    /// 虚拟化层操作
    VirtualizationAction {
        #[serde(flatten)]
        action: VirtualizationAction,

        /// 是否验证
        #[serde(default)]
        verify: bool,
    },

    /// 等待
    Wait {
        /// 等待时长
        #[serde(with = "duration_serde")]
        duration: Duration,
    },

    /// 验证条件
    Verify {
        /// 验证条件
        condition: VerifyCondition,

        /// 超时时间
        #[serde(default, with = "option_duration_serde")]
        timeout: Option<Duration>,
    },
}

/// VDI 平台操作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum VdiAction {
    /// 创建桌面池
    CreateDeskPool {
        name: String,
        template_id: String,
        count: u32,
    },

    /// 启用桌面池
    EnableDeskPool {
        pool_id: String,
    },

    /// 禁用桌面池
    DisableDeskPool {
        pool_id: String,
    },

    /// 删除桌面池
    DeleteDeskPool {
        pool_id: String,
    },

    /// 启动虚拟机
    StartDomain {
        domain_id: String,
    },

    /// 关闭虚拟机
    ShutdownDomain {
        domain_id: String,
    },

    /// 重启虚拟机
    RebootDomain {
        domain_id: String,
    },

    /// 删除虚拟机
    DeleteDomain {
        domain_id: String,
    },

    /// 绑定用户
    BindUser {
        domain_id: String,
        user_id: String,
    },

    /// 获取桌面池虚拟机列表
    GetDeskPoolDomains {
        pool_id: String,
    },
}

/// 虚拟化层操作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum VirtualizationAction {
    /// 连接到虚拟机
    Connect {
        domain_id: String,
    },

    /// 发送键盘输入
    SendKeyboard {
        #[serde(default)]
        key: Option<String>,

        #[serde(default)]
        text: Option<String>,

        #[serde(default)]
        keys: Option<Vec<String>>,
    },

    /// 发送鼠标点击
    SendMouseClick {
        button: String,
        x: u32,
        y: u32,
    },

    /// 发送鼠标移动
    SendMouseMove {
        x: u32,
        y: u32,
    },

    /// 执行命令
    ExecuteCommand {
        command: String,
    },
}

/// 验证条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "condition", rename_all = "snake_case")]
pub enum VerifyCondition {
    /// 虚拟机状态
    DomainStatus {
        domain_id: String,
        expected_status: String,
    },

    /// 所有虚拟机运行中
    AllDomainsRunning {
        pool_id: String,
    },

    /// 命令执行成功
    CommandSuccess {
        domain_id: String,
    },
}

// 自定义 Duration 序列化/反序列化
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}s", duration.as_secs()))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_duration(&s).map_err(serde::de::Error::custom)
    }

    pub(super) fn parse_duration(s: &str) -> Result<Duration, String> {
        if let Some(s) = s.strip_suffix('s') {
            s.parse::<u64>()
                .map(Duration::from_secs)
                .map_err(|e| e.to_string())
        } else if let Some(s) = s.strip_suffix("ms") {
            s.parse::<u64>()
                .map(Duration::from_millis)
                .map_err(|e| e.to_string())
        } else {
            Err(format!("无效的时长格式: {}", s))
        }
    }
}

mod option_duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => serializer.serialize_some(&format!("{}s", d.as_secs())),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            Some(s) => super::duration_serde::parse_duration(&s)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_parse() {
        let yaml = r#"
name: "测试场景"
description: "这是一个测试场景"
steps:
  - type: wait
    duration: 5s
"#;
        let scenario = TestScenario::from_yaml_str(yaml);
        assert!(scenario.is_ok());
    }
}

impl TestScenario {
    fn from_yaml_str(yaml: &str) -> crate::Result<Self> {
        serde_yaml::from_str(yaml)
            .map_err(|e| crate::OrchestratorError::ScenarioParseError(e.to_string()))
    }
}

//! 测试场景定义

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 目标选择器 - 支持多种虚拟机选择模式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TargetSelector {
    /// 单个虚拟机名称 (向后兼容)
    Single(String),

    /// 高级选择器配置
    Advanced(TargetSelectorConfig),
}

impl Default for TargetSelector {
    fn default() -> Self {
        TargetSelector::Single("*".to_string())
    }
}

impl TargetSelector {
    /// 创建匹配所有虚拟机的选择器
    pub fn all() -> Self {
        TargetSelector::Advanced(TargetSelectorConfig {
            mode: TargetMode::All,
            pattern: None,
            names: None,
            exclude: None,
            limit: None,
        })
    }

    /// 创建通配符匹配选择器
    pub fn pattern(pattern: &str) -> Self {
        TargetSelector::Advanced(TargetSelectorConfig {
            mode: TargetMode::Pattern,
            pattern: Some(pattern.to_string()),
            names: None,
            exclude: None,
            limit: None,
        })
    }

    /// 创建精确匹配选择器
    pub fn exact(name: &str) -> Self {
        TargetSelector::Single(name.to_string())
    }

    /// 创建多个精确匹配选择器
    pub fn names(names: Vec<String>) -> Self {
        TargetSelector::Advanced(TargetSelectorConfig {
            mode: TargetMode::List,
            pattern: None,
            names: Some(names),
            exclude: None,
            limit: None,
        })
    }

    /// 检查给定的虚拟机名称是否匹配此选择器
    pub fn matches(&self, domain_name: &str) -> bool {
        match self {
            TargetSelector::Single(name) => {
                // 支持简单通配符
                if name == "*" {
                    true
                } else if name.contains('*') || name.contains('?') {
                    Self::glob_match(name, domain_name)
                } else {
                    name == domain_name
                }
            }
            TargetSelector::Advanced(config) => config.matches(domain_name),
        }
    }

    /// 简单的 glob 匹配实现
    fn glob_match(pattern: &str, text: &str) -> bool {
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();
        Self::glob_match_helper(&pattern_chars, &text_chars, 0, 0)
    }

    fn glob_match_helper(pattern: &[char], text: &[char], pi: usize, ti: usize) -> bool {
        if pi == pattern.len() && ti == text.len() {
            return true;
        }
        if pi == pattern.len() {
            return false;
        }

        match pattern[pi] {
            '*' => {
                // * 匹配零个或多个字符
                for i in ti..=text.len() {
                    if Self::glob_match_helper(pattern, text, pi + 1, i) {
                        return true;
                    }
                }
                false
            }
            '?' => {
                // ? 匹配单个字符
                if ti < text.len() {
                    Self::glob_match_helper(pattern, text, pi + 1, ti + 1)
                } else {
                    false
                }
            }
            c => {
                // 精确匹配
                if ti < text.len() && text[ti] == c {
                    Self::glob_match_helper(pattern, text, pi + 1, ti + 1)
                } else {
                    false
                }
            }
        }
    }

    /// 判断是否为多目标选择器
    pub fn is_multi_target(&self) -> bool {
        match self {
            TargetSelector::Single(name) => name.contains('*') || name.contains('?'),
            TargetSelector::Advanced(config) => {
                matches!(
                    config.mode,
                    TargetMode::All | TargetMode::Pattern | TargetMode::List
                )
            }
        }
    }
}

/// 目标选择器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSelectorConfig {
    /// 选择模式
    pub mode: TargetMode,

    /// 通配符/正则表达式模式 (用于 Pattern 模式)
    #[serde(default)]
    pub pattern: Option<String>,

    /// 虚拟机名称列表 (用于 List 模式)
    #[serde(default)]
    pub names: Option<Vec<String>>,

    /// 排除的虚拟机名称或模式
    #[serde(default)]
    pub exclude: Option<Vec<String>>,

    /// 最大匹配数量限制
    #[serde(default)]
    pub limit: Option<usize>,
}

impl TargetSelectorConfig {
    /// 检查虚拟机名称是否匹配
    pub fn matches(&self, domain_name: &str) -> bool {
        // 先检查排除列表
        if let Some(excludes) = &self.exclude {
            for exclude in excludes {
                if exclude.contains('*') || exclude.contains('?') {
                    if TargetSelector::glob_match(exclude, domain_name) {
                        return false;
                    }
                } else if exclude == domain_name {
                    return false;
                }
            }
        }

        // 根据模式匹配
        match self.mode {
            TargetMode::All => true,
            TargetMode::Single => {
                if let Some(pattern) = &self.pattern {
                    pattern == domain_name
                } else {
                    false
                }
            }
            TargetMode::Pattern => {
                if let Some(pattern) = &self.pattern {
                    TargetSelector::glob_match(pattern, domain_name)
                } else {
                    false
                }
            }
            TargetMode::List => {
                if let Some(names) = &self.names {
                    names.iter().any(|n| n == domain_name)
                } else {
                    false
                }
            }
            TargetMode::Regex => {
                if let Some(pattern) = &self.pattern {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(domain_name))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
        }
    }
}

/// 目标选择模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetMode {
    /// 单个精确匹配
    Single,
    /// 匹配所有虚拟机
    All,
    /// 通配符匹配 (支持 * 和 ?)
    Pattern,
    /// 名称列表
    List,
    /// 正则表达式匹配
    Regex,
}

/// 输入通道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InputChannelType {
    /// 使用 QMP 协议发送按键 (默认，通过 QEMU Monitor)
    #[default]
    Qmp,
    /// 使用 SPICE 协议发送按键 (通过 SPICE 输入通道)
    Spice,
}

/// 输入通道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputChannelConfig {
    /// 输入通道类型 (默认: qmp)
    #[serde(default)]
    pub channel_type: InputChannelType,

    /// 按键间隔时间 (毫秒，默认: 50)
    #[serde(default = "default_key_delay")]
    pub key_delay_ms: u64,

    /// 按键保持时间 (毫秒，默认: 100)
    #[serde(default = "default_key_hold")]
    pub key_hold_ms: u64,
}

fn default_key_delay() -> u64 {
    50
}

fn default_key_hold() -> u64 {
    100
}

impl Default for InputChannelConfig {
    fn default() -> Self {
        Self {
            channel_type: InputChannelType::Qmp,
            key_delay_ms: default_key_delay(),
            key_hold_ms: default_key_hold(),
        }
    }
}

/// 验证服务器配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerificationConfig {
    /// WebSocket 服务器地址 (默认: "0.0.0.0:8765")
    #[serde(default)]
    pub ws_addr: Option<String>,

    /// TCP 服务器地址 (默认: "0.0.0.0:8766")
    #[serde(default)]
    pub tcp_addr: Option<String>,

    /// 虚拟机内 guest-verifier 二进制路径 (默认: "/usr/local/bin/verifier-agent")
    #[serde(default)]
    pub guest_verifier_path: Option<String>,

    /// 等待客户端连接的超时时间（秒，默认: 30）
    #[serde(default)]
    pub connection_timeout: Option<u64>,

    /// 验证用的虚拟机 ID (可选，默认使用 target_domain)
    #[serde(default)]
    pub vm_id: Option<String>,

    /// 宿主机网络接口名称 (用于获取虚拟机可访问的 IP 地址)
    /// 例如: "virbr0", "br0", "eth0"
    /// 默认: "virbr0" (libvirt 默认网桥)
    #[serde(default)]
    pub host_interface: Option<String>,
}

/// 测试场景
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// 场景名称
    pub name: String,

    /// 场景描述
    pub description: Option<String>,

    /// 目标主机 (向后兼容, 优先使用 target_hosts)
    /// 支持通配符: "host*" 匹配 host1, host2 等
    #[serde(default)]
    pub target_host: Option<String>,

    /// 目标主机选择器 (新版, 支持通配符和多选)
    /// 优先级高于 target_host
    /// 默认: 匹配所有主机 (从 VDI 平台获取)
    #[serde(default)]
    pub target_hosts: Option<TargetSelector>,

    /// 目标虚拟机名称 (向后兼容, 优先使用 target_domains)
    #[serde(default)]
    pub target_domain: Option<String>,

    /// 目标虚拟机选择器 (新版, 支持通配符和多选)
    /// 优先级高于 target_domain
    #[serde(default)]
    pub target_domains: Option<TargetSelector>,

    /// 验证服务器配置 (可选,用于输入验证)
    #[serde(default)]
    pub verification: Option<VerificationConfig>,

    /// 输入通道配置 (用于发送键盘/鼠标事件)
    #[serde(default)]
    pub input_channel: InputChannelConfig,

    /// 测试步骤
    pub steps: Vec<ScenarioStep>,

    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,

    /// 并行执行配置 (当匹配多个虚拟机时)
    #[serde(default)]
    pub parallel: Option<ParallelConfig>,
}

/// 并行执行配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParallelConfig {
    /// 是否启用并行执行 (默认: false, 串行执行)
    #[serde(default)]
    pub enabled: bool,

    /// 最大并发数 (默认: 10)
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,

    /// 失败策略
    #[serde(default)]
    pub on_failure: FailureStrategy,
}

fn default_max_concurrent() -> usize {
    10
}

/// 失败策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FailureStrategy {
    /// 继续执行其他虚拟机 (默认)
    #[default]
    Continue,
    /// 立即停止所有执行
    StopAll,
    /// 快速失败 - 不启动新任务但等待已运行任务完成
    FailFast,
}

impl Scenario {
    /// 从 YAML 文件加载场景
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml_str(&content)
    }

    /// 从 YAML 字符串加载场景
    pub fn from_yaml_str(yaml: &str) -> crate::Result<Self> {
        serde_yaml::from_str(yaml).map_err(|e| crate::ExecutorError::SerdeError(e.to_string()))
    }

    /// 从 JSON 文件加载场景
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json_str(&content)
    }

    /// 从 JSON 字符串加载场景
    pub fn from_json_str(json: &str) -> crate::Result<Self> {
        serde_json::from_str(json).map_err(|e| crate::ExecutorError::SerdeError(e.to_string()))
    }

    /// 导出为 YAML
    pub fn to_yaml(&self) -> crate::Result<String> {
        serde_yaml::to_string(self).map_err(|e| crate::ExecutorError::SerdeError(e.to_string()))
    }

    /// 导出为 JSON
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::ExecutorError::SerdeError(e.to_string()))
    }

    /// 获取主机选择器 (优先使用 target_hosts, 向后兼容 target_host)
    /// 如果都未配置，默认返回 All (匹配所有主机)
    pub fn get_host_selector(&self) -> TargetSelector {
        // 优先使用新的 target_hosts 字段
        if let Some(selector) = &self.target_hosts {
            return selector.clone();
        }

        // 向后兼容: 使用 target_host 字段
        if let Some(host) = &self.target_host {
            return TargetSelector::Single(host.clone());
        }

        // 默认: 匹配所有主机
        TargetSelector::all()
    }

    /// 判断是否为多主机场景
    pub fn is_multi_host(&self) -> bool {
        self.get_host_selector().is_multi_target()
    }

    /// 从主机列表中筛选匹配的主机
    pub fn filter_hosts<'a>(&self, hosts: &'a [String]) -> Vec<&'a String> {
        let selector = self.get_host_selector();

        let mut matched: Vec<&String> =
            hosts.iter().filter(|name| selector.matches(name)).collect();

        // 应用数量限制
        if let TargetSelector::Advanced(config) = &selector {
            if let Some(limit) = config.limit {
                matched.truncate(limit);
            }
        }

        matched
    }

    /// 获取目标选择器 (优先使用 target_domains, 向后兼容 target_domain)
    pub fn get_target_selector(&self) -> Option<TargetSelector> {
        // 优先使用新的 target_domains 字段
        if let Some(selector) = &self.target_domains {
            return Some(selector.clone());
        }

        // 向后兼容: 使用 target_domain 字段
        self.target_domain
            .as_ref()
            .map(|name| TargetSelector::Single(name.clone()))
    }

    /// 判断是否为多目标场景
    pub fn is_multi_target(&self) -> bool {
        self.get_target_selector()
            .map(|s| s.is_multi_target())
            .unwrap_or(false)
    }

    /// 从虚拟机列表中筛选匹配的目标
    pub fn filter_targets<'a>(&self, domains: &'a [String]) -> Vec<&'a String> {
        let selector = match self.get_target_selector() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut matched: Vec<&String> = domains
            .iter()
            .filter(|name| selector.matches(name))
            .collect();

        // 应用数量限制
        if let TargetSelector::Advanced(config) = &selector {
            if let Some(limit) = config.limit {
                matched.truncate(limit);
            }
        }

        matched
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
    // ========================================
    // 协议操作 (已实现)
    // ========================================
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

    // ========================================
    // VDI 平台操作 (从 Orchestrator 迁移)
    // ========================================
    /// 创建桌面池
    VdiCreateDeskPool {
        name: String,
        template_id: String,
        count: u32,
    },

    /// 启用桌面池
    VdiEnableDeskPool { pool_id: String },

    /// 禁用桌面池
    VdiDisableDeskPool { pool_id: String },

    /// 删除桌面池
    VdiDeleteDeskPool { pool_id: String },

    /// 启动虚拟机
    VdiStartDomain { domain_id: String },

    /// 关闭虚拟机
    VdiShutdownDomain { domain_id: String },

    /// 重启虚拟机
    VdiRebootDomain { domain_id: String },

    /// 删除虚拟机
    VdiDeleteDomain { domain_id: String },

    /// 绑定用户
    VdiBindUser { domain_id: String, user_id: String },

    /// 获取桌面池虚拟机列表
    VdiGetDeskPoolDomains { pool_id: String },

    // ========================================
    // 验证步骤 (从 Orchestrator 迁移)
    // ========================================
    /// 验证虚拟机状态
    VerifyDomainStatus {
        domain_id: String,
        expected_status: String,
        #[serde(default)]
        timeout_secs: Option<u64>,
    },

    /// 验证所有虚拟机运行中
    VerifyAllDomainsRunning {
        pool_id: String,
        #[serde(default)]
        timeout_secs: Option<u64>,
    },

    /// 验证命令执行成功
    VerifyCommandSuccess {
        #[serde(default)]
        timeout_secs: Option<u64>,
    },
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
            target_host: None,
            target_hosts: None,
            target_domain: Some("test-vm".to_string()),
            target_domains: None,
            verification: None,
            input_channel: InputChannelConfig::default(),
            tags: vec!["test".to_string()],
            parallel: None,
            steps: vec![ScenarioStep {
                name: Some("发送按键".to_string()),
                action: Action::SendKey {
                    key: "a".to_string(),
                },
                verify: false,
                timeout: None,
            }],
        };

        let yaml = scenario.to_yaml().unwrap();
        assert!(yaml.contains("测试场景"));
    }

    // ==========================================
    // TargetSelector 测试
    // ==========================================

    #[test]
    fn test_target_selector_single() {
        let selector = TargetSelector::exact("test-vm");
        assert!(selector.matches("test-vm"));
        assert!(!selector.matches("other-vm"));
        assert!(!selector.is_multi_target());
    }

    #[test]
    fn test_target_selector_wildcard_star() {
        let selector = TargetSelector::Single("test-*".to_string());
        assert!(selector.matches("test-vm"));
        assert!(selector.matches("test-123"));
        assert!(selector.matches("test-"));
        assert!(!selector.matches("other-vm"));
        assert!(selector.is_multi_target());
    }

    #[test]
    fn test_target_selector_wildcard_question() {
        let selector = TargetSelector::Single("vm-?".to_string());
        assert!(selector.matches("vm-1"));
        assert!(selector.matches("vm-a"));
        assert!(!selector.matches("vm-12")); // ? 只匹配一个字符
        assert!(!selector.matches("vm-")); // ? 必须匹配一个字符
    }

    #[test]
    fn test_target_selector_all() {
        let selector = TargetSelector::all();
        assert!(selector.matches("any-vm"));
        assert!(selector.matches("test"));
        assert!(selector.is_multi_target());
    }

    #[test]
    fn test_target_selector_pattern() {
        let selector = TargetSelector::pattern("dev-*-vm");
        assert!(selector.matches("dev-123-vm"));
        assert!(selector.matches("dev--vm"));
        assert!(!selector.matches("prod-123-vm"));
    }

    #[test]
    fn test_target_selector_names() {
        let selector = TargetSelector::names(vec![
            "vm-1".to_string(),
            "vm-2".to_string(),
            "vm-3".to_string(),
        ]);
        assert!(selector.matches("vm-1"));
        assert!(selector.matches("vm-2"));
        assert!(selector.matches("vm-3"));
        assert!(!selector.matches("vm-4"));
    }

    #[test]
    fn test_target_selector_with_exclude() {
        let selector = TargetSelector::Advanced(TargetSelectorConfig {
            mode: TargetMode::Pattern,
            pattern: Some("test-*".to_string()),
            names: None,
            exclude: Some(vec!["test-skip".to_string(), "test-ignore-*".to_string()]),
            limit: None,
        });

        assert!(selector.matches("test-vm"));
        assert!(selector.matches("test-123"));
        assert!(!selector.matches("test-skip")); // 精确排除
        assert!(!selector.matches("test-ignore-1")); // 通配符排除
        assert!(!selector.matches("test-ignore-abc")); // 通配符排除
    }

    #[test]
    fn test_scenario_filter_targets() {
        let scenario = Scenario {
            name: "测试".to_string(),
            description: None,
            target_host: None,
            target_hosts: None,
            target_domain: None,
            target_domains: Some(TargetSelector::pattern("web-*")),
            verification: None,
            input_channel: InputChannelConfig::default(),
            steps: vec![],
            tags: vec![],
            parallel: None,
        };

        let all_domains = vec![
            "web-1".to_string(),
            "web-2".to_string(),
            "db-1".to_string(),
            "cache-1".to_string(),
        ];

        let matched = scenario.filter_targets(&all_domains);
        assert_eq!(matched.len(), 2);
        assert!(matched.contains(&&"web-1".to_string()));
        assert!(matched.contains(&&"web-2".to_string()));
    }

    #[test]
    fn test_scenario_filter_targets_with_limit() {
        let scenario = Scenario {
            name: "测试".to_string(),
            description: None,
            target_host: None,
            target_hosts: None,
            target_domain: None,
            target_domains: Some(TargetSelector::Advanced(TargetSelectorConfig {
                mode: TargetMode::All,
                pattern: None,
                names: None,
                exclude: None,
                limit: Some(2),
            })),
            verification: None,
            input_channel: InputChannelConfig::default(),
            steps: vec![],
            tags: vec![],
            parallel: None,
        };

        let all_domains = vec![
            "vm-1".to_string(),
            "vm-2".to_string(),
            "vm-3".to_string(),
            "vm-4".to_string(),
        ];

        let matched = scenario.filter_targets(&all_domains);
        assert_eq!(matched.len(), 2); // 限制为 2 个
    }

    // ==========================================
    // 主机选择器测试
    // ==========================================

    #[test]
    fn test_scenario_default_host_selector() {
        // 未配置主机时，默认匹配所有主机
        let yaml = r#"
name: "默认主机"
steps: []
"#;
        let scenario = Scenario::from_yaml_str(yaml).unwrap();
        let selector = scenario.get_host_selector();
        assert!(selector.matches("any-host"));
        assert!(scenario.is_multi_host());
    }

    #[test]
    fn test_scenario_single_host() {
        let yaml = r#"
name: "单主机"
target_host: "host1"
steps: []
"#;
        let scenario = Scenario::from_yaml_str(yaml).unwrap();
        let selector = scenario.get_host_selector();
        assert!(selector.matches("host1"));
        assert!(!selector.matches("host2"));
        assert!(!scenario.is_multi_host());
    }

    #[test]
    fn test_scenario_multi_hosts_pattern() {
        let yaml = r#"
name: "多主机"
target_hosts: "compute-*"
steps: []
"#;
        let scenario = Scenario::from_yaml_str(yaml).unwrap();
        let selector = scenario.get_host_selector();
        assert!(selector.matches("compute-01"));
        assert!(selector.matches("compute-node"));
        assert!(!selector.matches("storage-01"));
        assert!(scenario.is_multi_host());
    }

    #[test]
    fn test_scenario_filter_hosts() {
        let yaml = r#"
name: "筛选主机"
target_hosts:
  mode: pattern
  pattern: "node-*"
  exclude:
    - "node-maintenance"
steps: []
"#;
        let scenario = Scenario::from_yaml_str(yaml).unwrap();

        let all_hosts = vec![
            "node-01".to_string(),
            "node-02".to_string(),
            "node-maintenance".to_string(),
            "storage-01".to_string(),
        ];

        let matched = scenario.filter_hosts(&all_hosts);
        assert_eq!(matched.len(), 2);
        assert!(matched.contains(&&"node-01".to_string()));
        assert!(matched.contains(&&"node-02".to_string()));
    }

    #[test]
    fn test_scenario_backward_compatibility() {
        // 旧格式: 使用 target_domain
        let yaml = r#"
name: "旧格式测试"
target_domain: "legacy-vm"
steps: []
"#;
        let scenario = Scenario::from_yaml_str(yaml).unwrap();
        let selector = scenario.get_target_selector().unwrap();
        assert!(selector.matches("legacy-vm"));
        assert!(!selector.matches("other-vm"));
    }

    #[test]
    fn test_scenario_new_format_simple() {
        // 新格式: 简单字符串 (向后兼容)
        let yaml = r#"
name: "新格式简单"
target_domains: "test-*"
steps: []
"#;
        let scenario = Scenario::from_yaml_str(yaml).unwrap();
        let selector = scenario.get_target_selector().unwrap();
        assert!(selector.matches("test-vm"));
        assert!(selector.matches("test-123"));
        assert!(!selector.matches("prod-vm"));
    }

    #[test]
    fn test_scenario_new_format_advanced() {
        // 新格式: 高级配置
        let yaml = r#"
name: "新格式高级"
target_domains:
  mode: pattern
  pattern: "web-*"
  exclude:
    - "web-backup"
  limit: 5
steps: []
"#;
        let scenario = Scenario::from_yaml_str(yaml).unwrap();
        let selector = scenario.get_target_selector().unwrap();
        assert!(selector.matches("web-1"));
        assert!(!selector.matches("web-backup"));
    }

    #[test]
    fn test_scenario_target_all() {
        let yaml = r#"
name: "所有虚拟机"
target_domains:
  mode: all
  exclude:
    - "template-*"
steps: []
"#;
        let scenario = Scenario::from_yaml_str(yaml).unwrap();
        let selector = scenario.get_target_selector().unwrap();
        assert!(selector.matches("any-vm"));
        assert!(!selector.matches("template-base"));
    }
}

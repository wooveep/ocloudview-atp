//! 测试配置管理
//!
//! 支持从多个源加载测试配置:
//! - 环境变量 (优先级最高)
//! - 配置文件 (TOML/YAML/JSON)
//! - 默认值 (优先级最低)
//!
//! 配置文件搜索路径 (按优先级):
//! 1. `ATP_TEST_CONFIG` 环境变量指定的路径
//! 2. `./test.toml` (当前目录)
//! 3. `./tests/config.toml` (tests 目录)
//! 4. `~/.config/atp/test.toml` (用户配置目录)
//! 5. `/etc/atp/test.toml` (系统配置目录)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// ============================================
// 核心配置结构
// ============================================

/// 测试配置 (顶层)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// 环境配置
    #[serde(default)]
    pub environment: EnvironmentConfig,

    /// Libvirt 配置
    #[serde(default)]
    pub libvirt: LibvirtConfig,

    /// 虚拟机配置
    #[serde(default)]
    pub vm: VmConfig,

    /// 协议配置
    #[serde(default)]
    pub protocols: ProtocolsConfig,

    /// VDI 平台配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vdi: Option<VdiConfig>,

    /// 测试行为配置
    #[serde(default)]
    pub test: TestBehaviorConfig,

    /// 数据库配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseConfig>,
}

/// 环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// 测试模式 (unit/integration/e2e)
    #[serde(default = "default_test_mode")]
    pub mode: String,

    /// 日志级别 (debug/info/warn/error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

/// Libvirt 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibvirtConfig {
    /// 默认 URI
    #[serde(default = "default_libvirt_uri")]
    pub uri: String,

    /// 连接超时 (秒)
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,

    /// 心跳间隔 (秒)
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: u64,

    /// 自动重连
    #[serde(default = "default_auto_reconnect")]
    pub auto_reconnect: bool,

    /// 主机列表 (可选)
    #[serde(default)]
    pub hosts: HashMap<String, LibvirtHostConfig>,
}

/// Libvirt 主机配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibvirtHostConfig {
    pub id: String,
    pub host: String,
    pub uri: String,
}

/// 虚拟机配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// VM 名称
    #[serde(default = "default_vm_name")]
    pub name: String,

    /// 登录用户
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// 登录密码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// 等待启动时间 (秒)
    #[serde(default = "default_wait_boot")]
    pub wait_boot: u64,
}

/// 协议配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qmp: Option<QmpConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub qga: Option<QgaConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub spice: Option<SpiceConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtio_serial: Option<VirtioSerialConfig>,
}

/// QMP 协议配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmpConfig {
    #[serde(default = "default_qmp_socket_prefix")]
    pub socket_prefix: String,

    #[serde(default = "default_protocol_timeout")]
    pub timeout: u64,
}

/// QGA 协议配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QgaConfig {
    #[serde(default = "default_protocol_timeout")]
    pub timeout: u64,

    #[serde(default = "default_qga_wait_exec")]
    pub wait_exec: bool,
}

/// SPICE 协议配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceConfig {
    #[serde(default = "default_localhost")]
    pub host: String,

    #[serde(default = "default_spice_port")]
    pub port: u16,

    #[serde(default = "default_protocol_timeout")]
    pub timeout: u64,
}

/// VirtioSerial 协议配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtioSerialConfig {
    #[serde(default = "default_virtio_channel_prefix")]
    pub channel_prefix: String,

    #[serde(default = "default_protocol_timeout")]
    pub timeout: u64,
}

/// VDI 平台配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VdiConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,

    #[serde(default = "default_verify_ssl")]
    pub verify_ssl: bool,

    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,
}

/// 测试行为配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestBehaviorConfig {
    /// 默认超时 (秒)
    #[serde(default = "default_test_timeout")]
    pub timeout: u64,

    /// 失败重试次数
    #[serde(default = "default_test_retry")]
    pub retry: u32,

    /// 跳过慢速测试
    #[serde(default)]
    pub skip_slow: bool,

    /// 测试后清理资源
    #[serde(default = "default_test_cleanup")]
    pub cleanup: bool,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,

    #[serde(default = "default_auto_migrate")]
    pub auto_migrate: bool,

    #[serde(default)]
    pub cleanup_on_exit: bool,
}

// ============================================
// 默认值函数
// ============================================

fn default_test_mode() -> String {
    "integration".to_string()
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_libvirt_uri() -> String {
    "qemu:///system".to_string()
}
fn default_vm_name() -> String {
    "test-vm".to_string()
}
fn default_connect_timeout() -> u64 {
    10
}
fn default_heartbeat_interval() -> u64 {
    30
}
fn default_auto_reconnect() -> bool {
    true
}
fn default_wait_boot() -> u64 {
    30
}
fn default_qmp_socket_prefix() -> String {
    "/var/lib/libvirt/qemu/".to_string()
}
fn default_protocol_timeout() -> u64 {
    30
}
fn default_qga_wait_exec() -> bool {
    true
}
fn default_localhost() -> String {
    "localhost".to_string()
}
fn default_spice_port() -> u16 {
    5900
}
fn default_virtio_channel_prefix() -> String {
    "/var/lib/libvirt/qemu/channel/".to_string()
}
fn default_verify_ssl() -> bool {
    false
}
fn default_test_timeout() -> u64 {
    60
}
fn default_test_retry() -> u32 {
    3
}
fn default_test_cleanup() -> bool {
    true
}
fn default_auto_migrate() -> bool {
    true
}

// ============================================
// Default 实现
// ============================================

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            environment: EnvironmentConfig::default(),
            libvirt: LibvirtConfig::default(),
            vm: VmConfig::default(),
            protocols: ProtocolsConfig::default(),
            vdi: None,
            test: TestBehaviorConfig::default(),
            database: None,
        }
    }
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            mode: default_test_mode(),
            log_level: default_log_level(),
        }
    }
}

impl Default for LibvirtConfig {
    fn default() -> Self {
        Self {
            uri: default_libvirt_uri(),
            connect_timeout: default_connect_timeout(),
            heartbeat_interval: default_heartbeat_interval(),
            auto_reconnect: default_auto_reconnect(),
            hosts: HashMap::new(),
        }
    }
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            name: default_vm_name(),
            user: None,
            password: None,
            wait_boot: default_wait_boot(),
        }
    }
}

impl Default for ProtocolsConfig {
    fn default() -> Self {
        Self {
            qmp: Some(QmpConfig::default()),
            qga: Some(QgaConfig::default()),
            spice: Some(SpiceConfig::default()),
            virtio_serial: Some(VirtioSerialConfig::default()),
        }
    }
}

impl Default for QmpConfig {
    fn default() -> Self {
        Self {
            socket_prefix: default_qmp_socket_prefix(),
            timeout: default_protocol_timeout(),
        }
    }
}

impl Default for QgaConfig {
    fn default() -> Self {
        Self {
            timeout: default_protocol_timeout(),
            wait_exec: default_qga_wait_exec(),
        }
    }
}

impl Default for SpiceConfig {
    fn default() -> Self {
        Self {
            host: default_localhost(),
            port: default_spice_port(),
            timeout: default_protocol_timeout(),
        }
    }
}

impl Default for VirtioSerialConfig {
    fn default() -> Self {
        Self {
            channel_prefix: default_virtio_channel_prefix(),
            timeout: default_protocol_timeout(),
        }
    }
}

impl Default for TestBehaviorConfig {
    fn default() -> Self {
        Self {
            timeout: default_test_timeout(),
            retry: default_test_retry(),
            skip_slow: false,
            cleanup: default_test_cleanup(),
        }
    }
}

// ============================================
// 配置加载实现
// ============================================

impl TestConfig {
    /// 从多个源加载配置 (优先级: 环境变量 > 配置文件 > 默认值)
    pub fn load() -> Result<Self> {
        // 1. 从默认值开始
        let mut config = Self::default();

        // 2. 尝试加载配置文件
        if let Some(path) = Self::find_config_file() {
            tracing::debug!("Loading config from: {:?}", path);
            config = Self::load_from_file(&path)?;
        } else {
            tracing::debug!("No config file found, using defaults");
        }

        // 3. 从环境变量覆盖
        config.apply_env_vars()?;

        Ok(config)
    }

    /// 从指定文件加载配置
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        // 根据文件扩展名选择解析器
        let config = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML config: {:?}", path))?
        } else if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML config: {:?}", path))?
        } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse JSON config: {:?}", path))?
        } else {
            anyhow::bail!("Unsupported config file format: {:?}", path);
        };

        Ok(config)
    }

    /// 从指定路径字符串加载配置
    pub fn load_from_path(path: &str) -> Result<Self> {
        Self::load_from_file(Path::new(path))
    }

    /// 查找配置文件 (按优先级搜索)
    fn find_config_file() -> Option<PathBuf> {
        // 1. 环境变量指定的路径
        if let Ok(path) = env::var("ATP_TEST_CONFIG") {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        // 2. 当前目录
        let paths = vec![
            PathBuf::from("./test.toml"),
            PathBuf::from("./test.yaml"),
            PathBuf::from("./test.json"),
            PathBuf::from("./tests/config.toml"),
            PathBuf::from("./tests/config.yaml"),
        ];

        for path in &paths {
            if path.exists() {
                return Some(path.clone());
            }
        }

        // 3. 用户配置目录
        if let Some(home) = dirs::home_dir() {
            let user_paths = vec![
                home.join(".config/atp/test.toml"),
                home.join(".config/atp/test.yaml"),
            ];

            for path in &user_paths {
                if path.exists() {
                    return Some(path.clone());
                }
            }
        }

        // 4. 系统配置目录 (Linux)
        #[cfg(target_os = "linux")]
        {
            let system_paths = vec![
                PathBuf::from("/etc/atp/test.toml"),
                PathBuf::from("/etc/atp/test.yaml"),
            ];

            for path in &system_paths {
                if path.exists() {
                    return Some(path.clone());
                }
            }
        }

        None
    }

    /// 从环境变量覆盖配置
    fn apply_env_vars(&mut self) -> Result<()> {
        // Environment
        if let Ok(mode) = env::var("ATP_TEST_MODE") {
            self.environment.mode = mode;
        }
        if let Ok(level) = env::var("ATP_LOG_LEVEL") {
            self.environment.log_level = level;
        }

        // Libvirt
        if let Ok(uri) = env::var("ATP_TEST_HOST") {
            self.libvirt.uri = uri;
        }
        if let Ok(timeout) = env::var("ATP_CONNECT_TIMEOUT") {
            self.libvirt.connect_timeout = timeout
                .parse()
                .context("Invalid ATP_CONNECT_TIMEOUT value")?;
        }

        // VM
        if let Ok(name) = env::var("ATP_TEST_VM") {
            self.vm.name = name;
        }
        if let Ok(user) = env::var("ATP_TEST_VM_USER") {
            self.vm.user = Some(user);
        }
        if let Ok(password) = env::var("ATP_TEST_VM_PASSWORD") {
            self.vm.password = Some(password);
        }

        // Protocols - QMP
        if let Some(ref mut qmp) = self.protocols.qmp {
            if let Ok(prefix) = env::var("ATP_QMP_SOCKET") {
                qmp.socket_prefix = prefix;
            }
        }

        // Protocols - SPICE
        if let Some(ref mut spice) = self.protocols.spice {
            if let Ok(host) = env::var("ATP_SPICE_HOST") {
                spice.host = host;
            }
            if let Ok(port) = env::var("ATP_SPICE_PORT") {
                spice.port = port.parse().context("Invalid ATP_SPICE_PORT value")?;
            }
        }

        // VDI Platform
        if let Ok(base_url) = env::var("ATP_VDI_BASE_URL") {
            if self.vdi.is_none() {
                self.vdi = Some(VdiConfig {
                    base_url: base_url.clone(),
                    username: String::new(),
                    password: String::new(),
                    verify_ssl: default_verify_ssl(),
                    connect_timeout: default_connect_timeout(),
                });
            }
            self.vdi.as_mut().unwrap().base_url = base_url;
        }
        if let Ok(username) = env::var("ATP_VDI_USERNAME") {
            if let Some(ref mut vdi) = self.vdi {
                vdi.username = username;
            }
        }
        if let Ok(password) = env::var("ATP_VDI_PASSWORD") {
            if let Some(ref mut vdi) = self.vdi {
                vdi.password = password;
            }
        }
        if let Ok(verify_ssl) = env::var("ATP_VDI_VERIFY_SSL") {
            if let Some(ref mut vdi) = self.vdi {
                vdi.verify_ssl = verify_ssl.parse().unwrap_or(default_verify_ssl());
            }
        }

        // Test Behavior
        if let Ok(timeout) = env::var("ATP_TEST_TIMEOUT") {
            self.test.timeout = timeout
                .parse()
                .context("Invalid ATP_TEST_TIMEOUT value")?;
        }
        if let Ok(retry) = env::var("ATP_TEST_RETRY") {
            self.test.retry = retry.parse().context("Invalid ATP_TEST_RETRY value")?;
        }
        if let Ok(skip_slow) = env::var("ATP_TEST_SKIP_SLOW") {
            self.test.skip_slow = skip_slow.parse().unwrap_or(false);
        }

        Ok(())
    }

    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        // 验证必填字段
        if self.vm.name.is_empty() {
            anyhow::bail!("VM name cannot be empty");
        }

        // 验证 VDI 配置 (如果存在)
        if let Some(ref vdi) = self.vdi {
            if vdi.base_url.is_empty() {
                anyhow::bail!("VDI base_url cannot be empty");
            }
            if vdi.username.is_empty() {
                anyhow::bail!("VDI username cannot be empty");
            }
        }

        Ok(())
    }

    /// 保存配置到文件
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        // 根据文件扩展名选择格式
        let content = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::to_string_pretty(self).context("Failed to serialize to TOML")?
        } else if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::to_string(self).context("Failed to serialize to YAML")?
        } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::to_string_pretty(self).context("Failed to serialize to JSON")?
        } else {
            anyhow::bail!("Unsupported config file format: {:?}", path);
        };

        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TestConfig::default();
        assert_eq!(config.environment.mode, "integration");
        assert_eq!(config.vm.name, "test-vm");
        assert_eq!(config.libvirt.uri, "qemu:///system");
    }

    #[test]
    fn test_config_serialization() {
        let config = TestConfig::default();

        // Test TOML
        let toml = toml::to_string_pretty(&config).unwrap();
        assert!(toml.contains("mode = \"integration\""));

        // Test JSON
        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("\"mode\": \"integration\""));

        // Test YAML
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("mode: integration"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = TestConfig::default();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Empty VM name should fail
        config.vm.name = String::new();
        assert!(config.validate().is_err());
    }
}

# 测试配置实现方案

**创建日期**: 2025-12-01
**状态**: 设计文档
**优先级**: 高

---

## 概述

本文档描述测试配置的读取和管理机制的实现方案。

## 现状分析

### 当前实现

✅ **已有配置机制**:

1. **CLI 配置** (atp-application/cli/src/config.rs)
   - ✅ TOML 文件读取 (`~/.config/atp/config.toml`)
   - ✅ 主机配置管理
   - ✅ 序列化/反序列化支持
   - ⚠️ **局限**: 仅用于 CLI 主机管理,不适用于测试配置

2. **Transport 配置** (atp-core/transport/src/config.rs)
   - ✅ 结构化配置定义 (TransportConfig, PoolConfig, ReconnectConfig)
   - ✅ Serde 序列化支持
   - ✅ 默认值定义
   - ⚠️ **局限**: 没有文件读取功能,只能通过代码构造

3. **E2E 测试配置** (atp-core/executor/tests/e2e_tests.rs)
   - ✅ 环境变量读取 (ATP_TEST_VM, ATP_TEST_HOST)
   - ✅ 简单的辅助函数
   - ⚠️ **局限**: 仅支持环境变量,没有配置文件支持

### 缺失功能

❌ **需要实现**:

1. **统一的测试配置加载器**
   - 支持多种配置源 (环境变量 + 配置文件)
   - 配置优先级管理
   - 配置验证

2. **测试配置结构定义**
   - libvirt 配置
   - VM 配置
   - 协议配置
   - VDI 平台配置
   - 测试行为配置

3. **配置文件搜索路径**
   - 自动搜索多个位置
   - 支持自定义路径

---

## 设计方案

### 1. 配置结构设计

#### 1.1 核心配置结构

```rust
// atp-core/executor/src/test_config.rs (新文件)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
    pub vdi: Option<VdiConfig>,

    /// 测试行为配置
    #[serde(default)]
    pub test: TestBehaviorConfig,

    /// 数据库配置
    pub database: Option<DatabaseConfig>,
}

/// 环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// 测试模式 (unit/integration/e2e)
    #[serde(default = "default_test_mode")]
    pub mode: String,

    /// 日志级别
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

    /// 主机列表
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
    pub user: Option<String>,

    /// 登录密码
    pub password: Option<String>,

    /// 等待启动时间 (秒)
    #[serde(default = "default_wait_boot")]
    pub wait_boot: u64,
}

/// 协议配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolsConfig {
    pub qmp: Option<QmpConfig>,
    pub qga: Option<QgaConfig>,
    pub spice: Option<SpiceConfig>,
    pub virtio_serial: Option<VirtioSerialConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmpConfig {
    #[serde(default = "default_qmp_socket_prefix")]
    pub socket_prefix: String,

    #[serde(default = "default_protocol_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QgaConfig {
    #[serde(default = "default_protocol_timeout")]
    pub timeout: u64,

    #[serde(default = "default_qga_wait_exec")]
    pub wait_exec: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceConfig {
    #[serde(default = "default_localhost")]
    pub host: String,

    #[serde(default = "default_spice_port")]
    pub port: u16,

    #[serde(default = "default_protocol_timeout")]
    pub timeout: u64,
}

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

// ===== 默认值函数 =====

fn default_test_mode() -> String { "integration".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_libvirt_uri() -> String { "qemu:///system".to_string() }
fn default_vm_name() -> String { "test-vm".to_string() }
fn default_connect_timeout() -> u64 { 10 }
fn default_heartbeat_interval() -> u64 { 30 }
fn default_auto_reconnect() -> bool { true }
fn default_wait_boot() -> u64 { 30 }
fn default_qmp_socket_prefix() -> String { "/var/lib/libvirt/qemu/".to_string() }
fn default_protocol_timeout() -> u64 { 30 }
fn default_qga_wait_exec() -> bool { true }
fn default_localhost() -> String { "localhost".to_string() }
fn default_spice_port() -> u16 { 5900 }
fn default_virtio_channel_prefix() -> String { "/var/lib/libvirt/qemu/channel/".to_string() }
fn default_verify_ssl() -> bool { false }
fn default_test_timeout() -> u64 { 60 }
fn default_test_retry() -> u32 { 3 }
fn default_test_cleanup() -> bool { true }
fn default_auto_migrate() -> bool { true }

// ===== Default 实现 =====

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
```

#### 1.2 配置加载器

```rust
// atp-core/executor/src/test_config.rs (续)

use anyhow::{Context, Result};
use std::fs;
use std::env;
use std::path::Path;

impl TestConfig {
    /// 从多个源加载配置 (优先级: 环境变量 > 配置文件 > 默认值)
    pub fn load() -> Result<Self> {
        // 1. 从默认值开始
        let mut config = Self::default();

        // 2. 尝试加载配置文件
        if let Some(path) = Self::find_config_file() {
            config = Self::load_from_file(&path)?;
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
               || path.extension().and_then(|s| s.to_str()) == Some("yml") {
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

        // 4. 系统配置目录
        let system_paths = vec![
            PathBuf::from("/etc/atp/test.toml"),
            PathBuf::from("/etc/atp/test.yaml"),
        ];

        for path in &system_paths {
            if path.exists() {
                return Some(path.clone());
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
            self.libvirt.connect_timeout = timeout.parse()
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
                spice.port = port.parse()
                    .context("Invalid ATP_SPICE_PORT value")?;
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
                vdi.verify_ssl = verify_ssl.parse()
                    .unwrap_or(default_verify_ssl());
            }
        }

        // Test Behavior
        if let Ok(timeout) = env::var("ATP_TEST_TIMEOUT") {
            self.test.timeout = timeout.parse()
                .context("Invalid ATP_TEST_TIMEOUT value")?;
        }
        if let Ok(retry) = env::var("ATP_TEST_RETRY") {
            self.test.retry = retry.parse()
                .context("Invalid ATP_TEST_RETRY value")?;
        }
        if let Ok(skip_slow) = env::var("ATP_TEST_SKIP_SLOW") {
            self.test.skip_slow = skip_slow.parse()
                .unwrap_or(false);
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
}
```

### 2. 使用示例

#### 2.1 E2E 测试中使用

```rust
// atp-core/executor/tests/e2e_tests.rs (修改)

mod test_config; // 引入配置模块

use test_config::TestConfig;

/// 初始化测试环境 (使用配置文件)
async fn setup_test_runner() -> (ScenarioRunner, TestConfig) {
    // 加载测试配置
    let config = TestConfig::load()
        .expect("Failed to load test config");

    config.validate()
        .expect("Invalid test config");

    // 初始化日志
    let _ = tracing_subscriber::fmt()
        .with_env_filter(&config.environment.log_level)
        .try_init();

    // 创建传输管理器
    let transport_manager = Arc::new(TransportManager::default());

    // 添加主机 (从配置读取)
    let host_info = HostInfo {
        id: "test-host".to_string(),
        host: "localhost".to_string(),
        uri: config.libvirt.uri.clone(),
        tags: vec!["test".to_string()],
        metadata: HashMap::new(),
    };

    transport_manager.add_host(host_info).await
        .expect("Failed to add test host");

    // 创建协议注册表
    let protocol_registry = Arc::new(ProtocolRegistry::new());

    // 创建场景执行器
    let runner = ScenarioRunner::new(transport_manager, protocol_registry)
        .with_timeout(Duration::from_secs(config.test.timeout));

    (runner, config)
}

#[tokio::test]
#[ignore]
async fn test_qmp_keyboard_input() {
    let (mut runner, config) = setup_test_runner().await;

    let scenario = Scenario {
        name: "qmp-keyboard-test".to_string(),
        description: Some("QMP 键盘输入测试".to_string()),
        target_host: Some(config.libvirt.uri.clone()),
        target_domain: Some(config.vm.name.clone()), // 使用配置的 VM 名称
        steps: vec![
            // ... 测试步骤
        ],
        tags: vec!["e2e".to_string(), "qmp".to_string()],
    };

    let report = runner.run(&scenario).await
        .expect("Failed to run scenario");

    assert!(report.success, "Scenario should succeed");
}
```

#### 2.2 创建配置文件

```bash
# 在项目根目录创建 test.toml
cat > test.toml << 'EOF'
[environment]
mode = "e2e"
log_level = "debug"

[libvirt]
uri = "qemu:///system"
connect_timeout = 15

[vm]
name = "ubuntu-test-vm"
user = "test"
password = "test123"
wait_boot = 30

[protocols.qmp]
socket_prefix = "/var/lib/libvirt/qemu/"
timeout = 30

[protocols.qga]
timeout = 60
wait_exec = true

[protocols.spice]
host = "localhost"
port = 5900
timeout = 30

[test]
timeout = 120
retry = 2
skip_slow = false
cleanup = true
EOF
```

#### 2.3 使用环境变量

```bash
# 覆盖配置文件中的设置
export ATP_TEST_VM=my-test-vm
export ATP_TEST_HOST=qemu+ssh://root@192.168.1.100/system
export ATP_LOG_LEVEL=debug

# 运行测试
cargo test --test e2e_tests -- --nocapture
```

---

## 实施步骤

### Phase 1: 核心结构 (1-2天)

1. ✅ 创建 `atp-core/executor/src/test_config.rs`
2. ✅ 定义所有配置结构
3. ✅ 实现 Default trait
4. ✅ 添加序列化支持

### Phase 2: 配置加载 (1-2天)

5. ✅ 实现 `load()` 方法
6. ✅ 实现 `load_from_file()` 方法
7. ✅ 实现 `find_config_file()` 方法
8. ✅ 实现 `apply_env_vars()` 方法
9. ✅ 实现 `validate()` 方法

### Phase 3: 集成测试 (1天)

10. ⏳ 修改 `e2e_tests.rs` 使用新配置
11. ⏳ 创建测试配置文件示例
12. ⏳ 编写单元测试

### Phase 4: 文档和示例 (1天)

13. ⏳ 更新 TESTING_CONFIG_GUIDE.md
14. ⏳ 创建配置文件模板
15. ⏳ 编写使用示例

---

## 依赖

需要在 `atp-core/executor/Cargo.toml` 中添加:

```toml
[dependencies]
# 现有依赖...
dirs = "5.0"      # 用于获取用户目录
toml = "0.8"      # TOML 解析
serde_yaml = "0.9" # YAML 解析 (可选)

[dev-dependencies]
# 测试依赖...
```

---

## 优点

1. ✅ **统一配置**: 所有测试使用相同的配置结构
2. ✅ **灵活性**: 支持多种配置源和格式
3. ✅ **优先级**: 环境变量 > 配置文件 > 默认值
4. ✅ **可扩展**: 易于添加新的配置项
5. ✅ **向后兼容**: 环境变量方式仍然有效
6. ✅ **类型安全**: Rust 类型系统保证配置正确性

---

## 后续优化

1. 配置热重载
2. 配置加密 (敏感信息)
3. 配置模板生成工具
4. 配置验证规则引擎
5. 与 CI/CD 集成的最佳实践

---

**维护者**: OCloudView ATP Team
**最后更新**: 2025-12-01

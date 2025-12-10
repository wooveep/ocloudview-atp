# ATP 测试配置指南

**创建日期**: 2025-12-01
**版本**: v1.0
**适用范围**: 单元测试、集成测试、端到端测试

---

## 目录

1. [概述](#概述)
2. [测试环境配置](#测试环境配置)
3. [单元测试配置](#单元测试配置)
4. [集成测试配置](#集成测试配置)
5. [端到端测试配置](#端到端测试配置)
6. [VDI 平台测试配置](#vdi-平台测试配置)
7. [配置文件模板](#配置文件模板)
8. [故障排查](#故障排查)

---

## 概述

ATP 项目采用多层测试策略，支持通过配置文件和环境变量灵活配置测试环境。

### 测试层级

```
┌─────────────────────────────────────────────────────┐
│ 端到端测试 (E2E)                                      │
│ - 完整场景执行                                        │
│ - 需要实际虚拟机环境                                   │
│ - 配置: 环境变量 + YAML 场景                          │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│ 集成测试 (Integration)                                │
│ - 模块间交互测试                                      │
│ - 使用本地 libvirtd 或 Mock                           │
│ - 配置: 测试配置文件 + 环境变量                        │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│ 单元测试 (Unit)                                       │
│ - 独立模块测试                                        │
│ - 不依赖外部服务                                      │
│ - 配置: 硬编码测试数据                                │
└─────────────────────────────────────────────────────┘
```

### 配置优先级

```
环境变量 > 测试配置文件 > 默认值
```

---

## 测试环境配置

### 1. 环境变量

支持的环境变量:

```bash
# ============ 通用配置 ============
export ATP_TEST_MODE=unit|integration|e2e   # 测试模式
export ATP_LOG_LEVEL=debug|info|warn        # 日志级别

# ============ libvirt 配置 ============
export ATP_TEST_HOST=qemu:///system              # libvirt URI
export ATP_TEST_HOST_USER=root                   # SSH 用户 (用于远程主机)
export ATP_TEST_HOST_PORT=22                     # SSH 端口

# ============ 虚拟机配置 ============
export ATP_TEST_VM=test-vm                       # 测试虚拟机名称
export ATP_TEST_VM_USER=test                     # VM 登录用户
export ATP_TEST_VM_PASSWORD=password123          # VM 登录密码

# ============ VDI 平台配置 ============
export ATP_VDI_BASE_URL=http://192.168.1.11:8088 # VDI 平台 API 地址
export ATP_VDI_USERNAME=admin                    # VDI 平台用户名
export ATP_VDI_PASSWORD=admin123                 # VDI 平台密码
export ATP_VDI_VERIFY_SSL=false                  # 是否验证 SSL 证书

# ============ 协议配置 ============
export ATP_QMP_SOCKET=/var/lib/libvirt/qemu/     # QMP Socket 路径前缀
export ATP_SPICE_HOST=192.168.1.100              # SPICE 服务器地址
export ATP_SPICE_PORT=5900                       # SPICE 端口

# ============ 测试行为配置 ============
export ATP_TEST_TIMEOUT=60                       # 测试超时(秒)
export ATP_TEST_RETRY=3                          # 失败重试次数
export ATP_TEST_SKIP_SLOW=true                   # 跳过慢速测试
```

### 2. 测试配置文件

支持 TOML、YAML、JSON 格式的测试配置文件。

**默认路径优先级**:
1. `./test.toml` (当前目录)
2. `./tests/config.toml` (tests 目录)
3. `~/.config/atp/test.toml` (用户配置目录)
4. `/etc/atp/test.toml` (系统配置目录)

---

## 单元测试配置

### 特点

- ✅ 不依赖外部服务
- ✅ 快速执行 (< 1秒/测试)
- ✅ 使用硬编码测试数据
- ✅ Mock 外部依赖

### 运行单元测试

```bash
# 运行所有单元测试
cargo test --lib

# 运行特定模块的单元测试
cargo test --package atp-executor --lib
cargo test --package atp-transport --lib
cargo test --package atp-protocol --lib

# 运行特定测试
cargo test test_scenario_creation

# 显示详细输出
cargo test --lib -- --nocapture

# 跳过 #[ignore] 标记的测试
cargo test --lib -- --skip ignored
```

### 单元测试示例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_creation() {
        let scenario = Scenario {
            name: "test-scenario".to_string(),
            description: Some("A test scenario".to_string()),
            target_host: None,
            target_domain: None,
            steps: vec![],
            tags: vec!["test".to_string()],
        };

        assert_eq!(scenario.name, "test-scenario");
        assert_eq!(scenario.steps.len(), 0);
    }

    #[test]
    #[ignore] // 需要外部依赖
    fn test_with_external_dependency() {
        // ...
    }
}
```

---

## 集成测试配置

### 特点

- ⚠️ 需要本地 libvirtd 或 Mock 环境
- ⚠️ 中等执行时间 (1-10秒/测试)
- ✅ 测试模块间交互
- ✅ 支持配置文件和环境变量

### 环境准备

#### 方案 1: 使用本地 libvirtd (推荐)

```bash
# 1. 安装 libvirt
sudo apt-get install -y libvirt-daemon-system libvirt-clients

# 2. 启动服务
sudo systemctl start libvirtd

# 3. 验证连接
virsh list --all

# 4. 创建测试虚拟机 (可选)
sudo virt-install \
  --name test-vm \
  --ram 1024 \
  --vcpus 1 \
  --disk size=10 \
  --os-variant ubuntu20.04 \
  --graphics none
```

#### 方案 2: 使用 Mock libvirt (开发中)

```bash
# 设置 Mock 模式
export ATP_TEST_MODE=mock
export ATP_MOCK_LIBVIRT=true

# 运行集成测试
cargo test --tests
```

### 配置文件示例

**tests/integration_config.toml**:

```toml
[libvirt]
uri = "qemu:///system"
connect_timeout = 10
auto_reconnect = true

[libvirt.hosts]
local = { uri = "qemu:///system", host = "localhost" }
remote = { uri = "qemu+ssh://192.168.1.100/system", host = "192.168.1.100" }

[test]
timeout = 60          # 测试超时(秒)
retry = 3             # 失败重试次数
skip_slow = false     # 是否跳过慢速测试

[test.vm]
name = "test-vm"
user = "test"
password = "password123"

[protocols]
qmp_socket_prefix = "/var/lib/libvirt/qemu/"
spice_host = "192.168.1.100"
spice_port = 5900
```

### 运行集成测试

```bash
# 运行所有集成测试
cargo test --tests

# 使用配置文件
ATP_TEST_CONFIG=./tests/integration_config.toml cargo test --tests

# 使用环境变量
ATP_TEST_HOST=qemu:///system ATP_TEST_VM=test-vm cargo test --tests

# 运行特定集成测试
cargo test --test transport_integration_tests
cargo test --test protocol_integration_tests

# 显示详细输出
cargo test --tests -- --nocapture
```

---

## 端到端测试配置

### 特点

- ⚠️ **必须**有实际虚拟机环境
- ⚠️ 较长执行时间 (10-60秒/测试)
- ✅ 验证完整执行流程
- ✅ 支持 YAML/JSON 场景文件

### 环境准备

#### 1. 虚拟机要求

- ✅ 操作系统: Linux (Ubuntu/CentOS 推荐)
- ✅ 已安装 qemu-guest-agent (用于 QGA 测试)
- ✅ 已配置 SPICE (用于 SPICE 测试)
- ✅ 网络可达

#### 2. 配置 qemu-guest-agent

```bash
# 在虚拟机内安装
sudo apt-get install qemu-guest-agent  # Ubuntu/Debian
sudo yum install qemu-guest-agent      # CentOS/RHEL

# 启动服务
sudo systemctl start qemu-guest-agent
sudo systemctl enable qemu-guest-agent
```

#### 3. 配置 SPICE

**修改虚拟机 XML 配置**:

```bash
# 编辑虚拟机配置
virsh edit test-vm
```

**添加 SPICE 图形配置**:

```xml
<graphics type='spice' autoport='yes' listen='0.0.0.0'>
  <listen type='address' address='0.0.0.0'/>
</graphics>
<video>
  <model type='qxl' ram='65536' vram='65536' vgamem='16384' heads='1'/>
</video>
```

### E2E 测试配置

**tests/e2e_config.toml**:

```toml
[environment]
mode = "e2e"
log_level = "debug"

[libvirt]
uri = "qemu:///system"
connect_timeout = 15
auto_reconnect = true

[vm]
name = "test-vm"
user = "test"
password = "password123"
wait_boot = 30        # 等待启动时间(秒)

[protocols.qmp]
socket_prefix = "/var/lib/libvirt/qemu/"
timeout = 30

[protocols.qga]
timeout = 60
wait_exec = true      # 等待命令执行完成

[protocols.spice]
host = "192.168.1.100"
port = 5900
timeout = 30

[test]
timeout = 120         # E2E 测试超时(秒)
retry = 1             # 失败重试次数
cleanup = true        # 测试后清理
```

### 运行 E2E 测试

```bash
# 1. 启动测试虚拟机
virsh start test-vm

# 2. 设置环境变量
export ATP_TEST_HOST=qemu:///system
export ATP_TEST_VM=test-vm

# 3. 运行所有 E2E 测试
cargo test --test e2e_tests -- --nocapture

# 4. 运行特定 E2E 测试
cargo test --test e2e_tests test_qmp_keyboard_input -- --nocapture
cargo test --test e2e_tests test_qga_command_execution -- --nocapture
cargo test --test e2e_tests test_spice_mouse_operations -- --nocapture

# 5. 使用配置文件
ATP_TEST_CONFIG=./tests/e2e_config.toml cargo test --test e2e_tests -- --nocapture

# 6. 跳过慢速测试
ATP_TEST_SKIP_SLOW=true cargo test --test e2e_tests
```

### E2E 场景文件

**atp-core/executor/examples/scenarios/01-basic-keyboard.yaml**:

```yaml
name: "01-basic-keyboard"
description: "QMP 键盘输入测试"
target_host: "qemu:///system"
target_domain: "test-vm"
tags: ["e2e", "qmp", "keyboard"]

steps:
  - name: "等待系统就绪"
    action:
      wait:
        duration: 2
    verify: false

  - name: "发送 Enter 键"
    action:
      send_key:
        key: "enter"
    verify: true
    timeout: 10

  - name: "输入文本"
    action:
      send_text:
        text: "hello world"
    verify: true
    timeout: 15
```

**运行场景文件**:

```bash
# 使用 CLI 运行
cargo run --bin atp -- scenario run examples/scenarios/01-basic-keyboard.yaml

# 使用测试运行
cargo test --test e2e_tests test_load_scenario_from_yaml -- --nocapture
```

---

## VDI 平台测试配置

### 特点

- ⚠️ 需要 VDI 平台运行
- ⚠️ 需要认证凭据
- ✅ 测试 VDI API 集成
- ✅ 测试桌面池管理

### 环境准备

```bash
# 1. 确保 VDI 平台运行
curl http://192.168.1.11:8088/api/health

# 2. 配置环境变量
export ATP_VDI_BASE_URL=http://192.168.1.11:8088
export ATP_VDI_USERNAME=admin
export ATP_VDI_PASSWORD=admin123
export ATP_VDI_VERIFY_SSL=false

# 3. 运行 VDI 测试
cargo test --package atp-vdiplatform -- --nocapture
```

### VDI 测试配置

**tests/vdi_config.toml**:

```toml
[vdi]
base_url = "http://192.168.1.11:8088"
username = "admin"
password = "admin123"
verify_ssl = false
connect_timeout = 10
request_timeout = 30

[vdi.api]
domain = "/api/v1/domain"
desk_pool = "/api/v1/desk-pool"
host = "/api/v1/host"
model = "/api/v1/model"
user = "/api/v1/user"

[vdi.test]
# 测试用桌面池配置
test_pool_name = "test-pool"
test_pool_size = 2
test_template_id = "ubuntu20.04"
test_user_name = "test-user"

# 测试行为
cleanup_after_test = true
wait_vm_ready = 60    # 等待虚拟机就绪时间(秒)
```

### VDI 集成测试示例

```rust
#[tokio::test]
#[ignore] // 需要 VDI 平台
async fn test_desk_pool_lifecycle() {
    // 从环境变量读取配置
    let base_url = env::var("ATP_VDI_BASE_URL")
        .unwrap_or("http://192.168.1.11:8088".to_string());
    let username = env::var("ATP_VDI_USERNAME")
        .unwrap_or("admin".to_string());
    let password = env::var("ATP_VDI_PASSWORD")
        .unwrap_or("admin123".to_string());

    // 创建 VDI 客户端
    let mut client = VdiClient::new(&base_url, VdiConfig::default())
        .expect("Failed to create VDI client");

    // 登录
    client.login(&username, &password).await
        .expect("Failed to login");

    // 1. 创建桌面池
    // 2. 启用桌面池
    // 3. 验证虚拟机创建
    // 4. 清理资源
}
```

---

## 配置文件模板

### 完整测试配置模板

**tests/config.toml**:

```toml
# ============================================
# ATP 测试配置文件
# ============================================

[environment]
mode = "integration"       # unit | integration | e2e
log_level = "info"         # debug | info | warn | error

# ============================================
# libvirt 配置
# ============================================
[libvirt]
uri = "qemu:///system"
connect_timeout = 10
heartbeat_interval = 30
auto_reconnect = true

[libvirt.reconnect]
max_attempts = 5
initial_delay = 1
max_delay = 30
backoff_multiplier = 2.0

[libvirt.pool]
max_connections_per_host = 5
min_connections_per_host = 1
idle_timeout = 300
selection_strategy = "round_robin"  # round_robin | least_connections | random

[libvirt.hosts]
# 本地主机
local = { id = "local", host = "localhost", uri = "qemu:///system" }

# 远程主机
# remote1 = { id = "remote1", host = "192.168.1.100", uri = "qemu+ssh://root@192.168.1.100/system" }
# remote2 = { id = "remote2", host = "192.168.1.101", uri = "qemu+ssh://root@192.168.1.101/system" }

# ============================================
# 虚拟机配置
# ============================================
[vm]
name = "test-vm"
user = "test"
password = "password123"
wait_boot = 30
wait_shutdown = 10

# ============================================
# 协议配置
# ============================================
[protocols.qmp]
socket_prefix = "/var/lib/libvirt/qemu/"
timeout = 30
auto_negotiate = true

[protocols.qga]
timeout = 60
wait_exec = true
base64_input = true
base64_output = true

[protocols.spice]
host = "localhost"
port = 5900
timeout = 30
auth_method = "none"     # none | password | rsa

[protocols.virtio_serial]
channel_prefix = "/var/lib/libvirt/qemu/channel/"
timeout = 30

# ============================================
# VDI 平台配置
# ============================================
[vdi]
base_url = "http://192.168.1.11:8088"
username = "admin"
password = "admin123"
verify_ssl = false
connect_timeout = 10
request_timeout = 30
max_retries = 3

# ============================================
# 测试行为配置
# ============================================
[test]
timeout = 60              # 默认测试超时(秒)
retry = 3                 # 失败重试次数
skip_slow = false         # 是否跳过慢速测试
cleanup = true            # 测试后清理资源
parallel = false          # 是否并行运行测试

[test.e2e]
timeout = 120             # E2E 测试超时
wait_between_steps = 1    # 步骤间等待时间(秒)
capture_logs = true       # 是否捕获日志
save_reports = true       # 是否保存测试报告

# ============================================
# 数据库配置 (用于测试报告)
# ============================================
[database]
path = "./test_data.db"   # 测试数据库路径
auto_migrate = true       # 自动执行迁移
cleanup_on_exit = true    # 退出时清理
```

---

## 故障排查

### 常见问题

#### 1. libvirt 连接失败

**问题**: `Failed to connect to libvirt`

**解决方案**:

```bash
# 检查 libvirtd 服务
sudo systemctl status libvirtd

# 检查用户权限
groups | grep libvirt
sudo usermod -a -G libvirt $USER
newgrp libvirt

# 测试连接
virsh list --all
```

#### 2. QMP Socket 找不到

**问题**: `QMP socket not found: /var/lib/libvirt/qemu/test-vm.monitor`

**解决方案**:

```bash
# 检查虚拟机是否运行
virsh list --all

# 检查 socket 路径
sudo ls -la /var/lib/libvirt/qemu/

# 手动指定路径
export ATP_QMP_SOCKET=/path/to/qemu/
```

#### 3. SPICE 连接超时

**问题**: `SPICE connection timeout`

**解决方案**:

```bash
# 检查 SPICE 配置
virsh dumpxml test-vm | grep -A 5 "<graphics"

# 检查端口监听
sudo netstat -tulpn | grep 59

# 测试连接
telnet 192.168.1.100 5900
```

#### 4. 虚拟机无响应

**问题**: `Guest agent not responding`

**解决方案**:

```bash
# 在虚拟机内检查服务
sudo systemctl status qemu-guest-agent

# 重启服务
sudo systemctl restart qemu-guest-agent

# 检查通道
virsh qemu-agent-command test-vm '{"execute":"guest-ping"}'
```

#### 5. VDI 平台认证失败

**问题**: `VDI authentication failed`

**解决方案**:

```bash
# 测试 API 连接
curl -X POST http://192.168.1.11:8088/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'

# 检查环境变量
echo $ATP_VDI_BASE_URL
echo $ATP_VDI_USERNAME

# 禁用 SSL 验证
export ATP_VDI_VERIFY_SSL=false
```

### 调试技巧

#### 启用详细日志

```bash
# 设置日志级别
export RUST_LOG=debug
export ATP_LOG_LEVEL=debug

# 运行测试并查看详细输出
cargo test --test e2e_tests -- --nocapture
```

#### 使用测试选项

```bash
# 只运行一个测试
cargo test test_name -- --nocapture

# 显示测试时间
cargo test -- --nocapture --test-threads=1

# 保留失败的测试现场
ATP_TEST_CLEANUP=false cargo test
```

#### 检查测试报告

```bash
# 查看数据库中的测试报告
atp report list --limit 10

# 显示详细报告
atp report show <report-id>

# 导出报告
atp report export <report-id> --format json > report.json
```

---

## 持续集成 (CI) 配置

### GitHub Actions 示例

**.github/workflows/test.yml**:

```yaml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run unit tests
        run: cargo test --lib

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install libvirt
        run: |
          sudo apt-get update
          sudo apt-get install -y libvirt-daemon-system libvirt-clients
          sudo systemctl start libvirtd
      - name: Run integration tests
        run: cargo test --tests
        env:
          ATP_TEST_HOST: qemu:///system

  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup test environment
        run: |
          sudo apt-get update
          sudo apt-get install -y libvirt-daemon-system libvirt-clients qemu-kvm
          sudo systemctl start libvirtd
      - name: Create test VM
        run: |
          # 创建测试虚拟机的脚本
          ./scripts/setup_test_vm.sh
      - name: Run E2E tests
        run: cargo test --test e2e_tests
        env:
          ATP_TEST_HOST: qemu:///system
          ATP_TEST_VM: test-vm
```

---

## 总结

### 测试配置检查清单

- [ ] 环境变量已配置
- [ ] libvirt 服务已启动
- [ ] 测试虚拟机已创建和配置
- [ ] qemu-guest-agent 已安装
- [ ] SPICE 已配置 (如需要)
- [ ] VDI 平台已运行 (如需要)
- [ ] 测试配置文件已创建
- [ ] 网络连接正常
- [ ] 权限配置正确

### 推荐的测试工作流

1. **开发阶段**: 运行单元测试 (快速反馈)
   ```bash
   cargo test --lib
   ```

2. **集成阶段**: 运行集成测试 (模块交互)
   ```bash
   cargo test --tests
   ```

3. **发布前**: 运行完整 E2E 测试 (完整验证)
   ```bash
   cargo test --test e2e_tests -- --nocapture
   ```

4. **CI/CD**: 自动化所有测试层级

---

## 相关文档

- [E2E 测试指南](./E2E_TESTING_GUIDE.md)
- [单元测试文档](./STAGE8_TESTING.md)
- [VDI 平台测试](./VDI_PLATFORM_TESTING.md)
- [项目架构](./LAYERED_ARCHITECTURE.md)

---

**维护者**: OCloudView ATP Team
**最后更新**: 2025-12-01

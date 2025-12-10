# End-to-End 功能测试指南

本文档描述 ATP Executor 的端到端 (E2E) 测试框架和使用方法。

## 概述

端到端测试验证完整的执行流程：

```
Scenario → Executor → Protocol → VM
```

测试覆盖范围：
- ✅ QMP 协议 (键盘输入)
- ✅ QGA 协议 (命令执行)
- ✅ SPICE 协议 (鼠标操作)
- ✅ 混合协议场景
- ✅ 错误处理机制
- ✅ 超时处理
- ✅ 场景文件加载 (YAML/JSON)
- ✅ 性能测试

## 架构

### 测试文件结构

```
atp-core/executor/
├── tests/
│   ├── executor_tests.rs      # 单元测试 (44个测试)
│   └── e2e_tests.rs            # E2E测试 (新增)
└── examples/
    └── scenarios/              # 测试场景文件
        ├── README.md
        ├── 01-basic-keyboard.yaml
        ├── 02-command-execution.yaml
        ├── 03-mouse-operations.yaml
        ├── 04-mixed-protocols.yaml
        └── 05-error-handling.yaml
```

### E2E 测试套件

| 测试名称 | 协议 | 步骤数 | 描述 |
|---------|------|--------|------|
| `test_basic_scenario_wait` | - | 2 | 基础等待操作测试 |
| `test_qmp_keyboard_input` | QMP | 2 | 键盘输入测试 |
| `test_qga_command_execution` | QGA | 3 | 命令执行测试 |
| `test_spice_mouse_operations` | SPICE | 3 | 鼠标操作测试 |
| `test_mixed_protocol_scenario` | QMP+QGA+SPICE | 6 | 混合协议测试 |
| `test_load_scenario_from_yaml` | - | - | YAML 场景加载测试 |
| `test_load_scenario_from_json` | - | - | JSON 场景加载测试 |
| `test_command_failure_handling` | QGA | 3 | 错误处理测试 |
| `test_timeout_handling` | QGA | 1 | 超时处理测试 |
| `test_scenario_execution_performance` | QGA | 10 | 性能测试 |

**总计**: 10 个 E2E 测试

## 环境准备

### 1. 系统要求

- **操作系统**: Linux (Ubuntu 20.04+ / CentOS 7+ 推荐)
- **libvirt**: 已安装并运行
- **QEMU/KVM**: 已安装
- **权限**: 用户在 libvirt 组

### 2. 安装 libvirt

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y libvirt-daemon-system libvirt-clients qemu-kvm

# CentOS/RHEL
sudo yum install -y libvirt libvirt-client qemu-kvm

# 启动服务
sudo systemctl start libvirtd
sudo systemctl enable libvirtd

# 添加用户到 libvirt 组
sudo usermod -a -G libvirt $USER
newgrp libvirt

# 验证连接
virsh list --all
```

### 3. 准备测试虚拟机

#### 3.1 创建测试虚拟机

```bash
# 使用 virt-install 创建虚拟机
sudo virt-install \
  --name test-vm \
  --ram 2048 \
  --vcpus 2 \
  --disk path=/var/lib/libvirt/images/test-vm.qcow2,size=20 \
  --os-variant ubuntu20.04 \
  --network network=default \
  --graphics spice \
  --console pty,target_type=serial \
  --location http://archive.ubuntu.com/ubuntu/dists/focal/main/installer-amd64/ \
  --extra-args 'console=ttyS0,115200n8 serial'
```

#### 3.2 配置 QMP Socket

编辑虚拟机 XML 配置：

```bash
virsh edit test-vm
```

确保包含 QMP 配置：

```xml
<domain type='kvm'>
  <!-- ... -->
  <devices>
    <!-- QMP monitor socket -->
    <channel type='unix'>
      <source mode='bind' path='/var/lib/libvirt/qemu/monitor/test-vm-qmp.sock'/>
      <target type='virtio' name='org.qemu.guest_agent.0'/>
    </channel>
  </devices>
</domain>
```

#### 3.3 安装 QEMU Guest Agent

在虚拟机内执行：

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y qemu-guest-agent
sudo systemctl start qemu-guest-agent
sudo systemctl enable qemu-guest-agent

# CentOS/RHEL
sudo yum install -y qemu-guest-agent
sudo systemctl start qemu-guest-agent
sudo systemctl enable qemu-guest-agent

# 验证
sudo systemctl status qemu-guest-agent
```

从宿主机验证：

```bash
virsh qemu-agent-command test-vm '{"execute":"guest-ping"}'
# 应返回: {"return":{}}
```

#### 3.4 配置 SPICE 显示

编辑虚拟机 XML：

```xml
<domain type='kvm'>
  <!-- ... -->
  <devices>
    <!-- SPICE 图形显示 -->
    <graphics type='spice' autoport='yes'>
      <listen type='address' address='127.0.0.1'/>
    </graphics>

    <!-- SPICE 输入设备 -->
    <input type='tablet' bus='usb'/>
    <input type='mouse' bus='ps2'/>
  </devices>
</domain>
```

重启虚拟机使配置生效：

```bash
virsh shutdown test-vm
virsh start test-vm
```

### 4. 设置环境变量

```bash
# 在 ~/.bashrc 或 ~/.zshrc 中添加
export ATP_TEST_VM=test-vm
export ATP_TEST_HOST=qemu:///system

# 使配置生效
source ~/.bashrc
```

## 运行测试

### 1. 编译项目

```bash
cd /home/cloudyi/ocloudview-atp
cargo build --release
```

### 2. 运行单元测试 (不需要虚拟机)

```bash
cd atp-core/executor
cargo test --lib
```

应该看到 44 个单元测试全部通过。

### 3. 运行 E2E 测试 (需要虚拟机)

#### 运行所有 E2E 测试

```bash
cargo test --test e2e_tests -- --nocapture --ignored
```

参数说明：
- `--test e2e_tests`: 只运行 E2E 测试文件
- `--nocapture`: 显示 println! 输出
- `--ignored`: 运行标记为 `#[ignore]` 的测试

#### 运行特定 E2E 测试

```bash
# 1. 基础等待测试 (不需要协议连接)
cargo test --test e2e_tests test_basic_scenario_wait -- --nocapture --ignored

# 2. QMP 键盘输入测试
cargo test --test e2e_tests test_qmp_keyboard_input -- --nocapture --ignored

# 3. QGA 命令执行测试
cargo test --test e2e_tests test_qga_command_execution -- --nocapture --ignored

# 4. SPICE 鼠标操作测试
cargo test --test e2e_tests test_spice_mouse_operations -- --nocapture --ignored

# 5. 混合协议测试
cargo test --test e2e_tests test_mixed_protocol_scenario -- --nocapture --ignored

# 6. 错误处理测试
cargo test --test e2e_tests test_command_failure_handling -- --nocapture --ignored

# 7. 超时测试
cargo test --test e2e_tests test_timeout_handling -- --nocapture --ignored

# 8. 性能测试
cargo test --test e2e_tests test_scenario_execution_performance -- --nocapture --ignored
```

#### 运行场景加载测试 (不需要虚拟机)

```bash
cargo test --test e2e_tests test_load_scenario -- --nocapture
```

### 4. 启用调试日志

```bash
RUST_LOG=debug cargo test --test e2e_tests test_name -- --nocapture --ignored
```

日志级别：
- `error`: 仅错误
- `warn`: 警告和错误
- `info`: 信息、警告和错误
- `debug`: 调试信息
- `trace`: 详细追踪信息

## 测试输出解读

### 成功的测试输出

```
running 1 test

=== QGA 命令执行测试报告 ===
{
  "scenario_name": "qga-command-test",
  "description": "QGA 命令执行测试",
  "tags": ["e2e", "qga", "command"],
  "passed": true,
  "steps_executed": 3,
  "passed_count": 3,
  "failed_count": 0,
  "duration_ms": 1234,
  "steps": [
    {
      "step_index": 0,
      "description": "执行 echo 命令",
      "status": "Success",
      "error": null,
      "duration_ms": 123,
      "output": "Hello from QGA\n"
    },
    {
      "step_index": 1,
      "description": "执行 uname 命令",
      "status": "Success",
      "error": null,
      "duration_ms": 156,
      "output": "Linux test-vm 5.4.0-42-generic ...\n"
    },
    {
      "step_index": 2,
      "description": "执行 date 命令",
      "status": "Success",
      "error": null,
      "duration_ms": 98,
      "output": "Sun Dec  1 10:30:45 UTC 2025\n"
    }
  ]
}

通过步骤: 3/3

test test_qga_command_execution ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out
```

### 失败的测试输出

```
=== 错误处理测试报告 ===
{
  "scenario_name": "error-handling-test",
  "description": "错误处理测试",
  "passed": false,
  "steps_executed": 2,
  "passed_count": 1,
  "failed_count": 1,
  "steps": [
    {
      "step_index": 0,
      "description": "成功的命令",
      "status": "Success",
      "duration_ms": 100
    },
    {
      "step_index": 1,
      "description": "失败的命令",
      "status": "Failed",
      "error": "命令执行失败 (退出码: 1): ...",
      "duration_ms": 95
    }
  ]
}

test test_command_failure_handling ... ok
```

注意：这个测试**预期失败**是正常的，它验证了错误处理机制。

## 使用场景文件测试

### 从 YAML 文件运行场景

场景文件位于 `atp-core/executor/examples/scenarios/`。

#### 方法 1: 使用测试框架

测试会自动加载和验证这些场景文件。

#### 方法 2: 使用 ATP CLI (未来实现)

```bash
# 构建 CLI
cargo build --release -p atp-cli

# 运行场景
./target/release/atp scenario run \
  --file atp-core/executor/examples/scenarios/01-basic-keyboard.yaml \
  --host qemu:///system \
  --vm test-vm
```

### 创建自定义场景

创建 `my-scenario.yaml`：

```yaml
name: "my-test-scenario"
description: "我的自定义测试场景"
target_host: "qemu:///system"
target_domain: "test-vm"

tags:
  - "custom"
  - "test"

steps:
  - name: "步骤 1: 获取系统信息"
    action:
      type: exec_command
      command: "uname -r"
    verify: false
    timeout: 10

  - name: "步骤 2: 等待 2 秒"
    action:
      type: wait
      duration: 2
    verify: false

  - name: "步骤 3: 发送键盘输入"
    action:
      type: send_text
      text: "Hello ATP"
    verify: false
    timeout: 10
```

## 故障排查

### 问题 1: libvirt 连接失败

**错误**:
```
Error: Failed to connect to libvirt: Cannot connect to qemu:///system
```

**解决方案**:
```bash
# 检查 libvirtd 服务
sudo systemctl status libvirtd

# 如果未运行，启动服务
sudo systemctl start libvirtd

# 检查权限
groups | grep libvirt

# 如果不在 libvirt 组，添加用户
sudo usermod -a -G libvirt $USER
newgrp libvirt
```

### 问题 2: QMP 连接失败

**错误**:
```
WARN QMP 协议连接失败: Connection refused
```

**解决方案**:
```bash
# 检查 QMP socket 是否存在
ls -la /var/lib/libvirt/qemu/monitor/*-qmp.sock

# 检查虚拟机 XML 配置
virsh dumpxml test-vm | grep -A 5 "channel type='unix'"

# 如果没有 QMP 配置，添加并重启虚拟机
virsh edit test-vm
# 添加 QMP channel 配置
virsh shutdown test-vm
virsh start test-vm
```

### 问题 3: QGA 命令执行失败

**错误**:
```
Error: QGA 协议未初始化
```

**解决方案**:
```bash
# 在虚拟机内检查 guest agent
ssh user@test-vm
sudo systemctl status qemu-guest-agent

# 如果未运行
sudo systemctl start qemu-guest-agent
sudo systemctl enable qemu-guest-agent

# 从宿主机测试
virsh qemu-agent-command test-vm '{"execute":"guest-ping"}'

# 如果返回 error，检查虚拟机 XML 配置
virsh dumpxml test-vm | grep -A 5 "channel type"
```

### 问题 4: SPICE 连接失败

**错误**:
```
WARN SPICE 协议连接失败: Connection refused
```

**解决方案**:
```bash
# 检查 SPICE 配置
virsh dumpxml test-vm | grep -A 5 "graphics type='spice'"

# 查看 SPICE 端口
virsh domdisplay test-vm

# 如果没有 SPICE 配置，修改 XML
virsh edit test-vm
# 添加 SPICE 图形配置
virsh shutdown test-vm
virsh start test-vm
```

### 问题 5: 虚拟机不存在

**错误**:
```
Error: Domain not found: No domain with matching name 'test-vm'
```

**解决方案**:
```bash
# 列出所有虚拟机
virsh list --all

# 如果虚拟机存在但未运行
virsh start test-vm

# 如果虚拟机不存在，设置正确的虚拟机名称
export ATP_TEST_VM=actual-vm-name
```

### 问题 6: 权限问题

**错误**:
```
Error: Permission denied
```

**解决方案**:
```bash
# 检查 socket 权限
sudo ls -la /var/lib/libvirt/qemu/monitor/
sudo ls -la /var/lib/libvirt/qemu/channel/

# 确保用户在 libvirt 和 kvm 组
groups | grep -E 'libvirt|kvm'

# 添加到组
sudo usermod -a -G libvirt $USER
sudo usermod -a -G kvm $USER

# 重新登录或使用
newgrp libvirt
```

## 性能基准

基于标准测试虚拟机的预期性能：

| 操作类型 | 预期延迟 | 说明 |
|---------|---------|------|
| QMP 键盘输入 | < 100ms | 单个按键 |
| QGA 命令执行 | < 500ms | 简单命令 (echo, date) |
| SPICE 鼠标操作 | < 200ms | 单次点击 |
| 场景初始化 | < 2s | 连接所有协议 |
| 场景清理 | < 1s | 断开所有连接 |

## 持续集成 (CI)

### GitHub Actions 示例

创建 `.github/workflows/e2e-tests.yml`：

```yaml
name: E2E Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  e2e-tests:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Install dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y libvirt-daemon-system qemu-kvm
        sudo systemctl start libvirtd
        sudo usermod -a -G libvirt $USER

    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true

    - name: Run unit tests
      run: |
        cd atp-core/executor
        cargo test --lib

    - name: Run E2E tests
      run: |
        export ATP_TEST_VM=test-vm
        cargo test --test e2e_tests -- --ignored
```

## 下一步

- [ ] 添加 VDI 平台操作的 E2E 测试
- [ ] 实现数据库集成的 E2E 测试
- [ ] 添加性能压力测试 (50+ VMs)
- [ ] 实现自动化测试报告生成
- [ ] 添加 Windows Guest 测试支持

## 相关文档

- [Executor 实现总结](../../docs/STAGE4_EXECUTOR_IMPLEMENTATION.md)
- [协议层实现](../../docs/STAGE2_PROTOCOL_IMPLEMENTATION.md)
- [测试策略](../../docs/STAGE8_TESTING.md)
- [场景文件参考](../examples/scenarios/README.md)

---

**最后更新**: 2025-12-01
**维护者**: OCloudView ATP Team

# Guest 验证器 (Guest Verifier)

Guest 验证器是一个运行在虚拟机内部的 Agent，用于验证主机发送的输入事件（键盘、鼠标）和命令执行是否真正到达 Guest OS。

## 架构

```
┌─────────────────────────────────────────┐
│          Verifier API (接口层)           │
│     (与主框架通信 - WebSocket/TCP)       │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│        Verifier Core (验证器核心)        │
│  ┌────────┐  ┌────────┐  ┌──────────┐  │
│  │ 键盘   │  │ 鼠标   │  │ 命令执行 │  │
│  └────────┘  └────────┘  └──────────┘  │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│    Transport Layer (底层通道层)          │
│  ┌──────────┐  ┌──────────────────┐    │
│  │WebSocket │  │       TCP        │    │
│  └──────────┘  └──────────────────┘    │
└─────────────────────────────────────────┘
```

## 模块结构

### verifier-core (核心库)

- **传输层** (`transport/`)
  - `websocket.rs` - WebSocket 传输实现
  - `tcp.rs` - TCP 传输实现
- **验证器接口** (`verifier.rs`)
  - `Verifier` trait - 验证器抽象接口
  - `VerifierType` - 验证器类型枚举
- **事件和结果** (`event.rs`)
  - `Event` - 测试事件结构
  - `VerifyResult` - 验证结果结构

### verifier-agent (Agent 应用)

- **验证器实现** (`verifiers/`)
  - `keyboard.rs` - 键盘验证器
    - Linux: 使用 evdev 监听键盘事件
    - Windows: TODO (使用 Hook API)
  - `mouse.rs` - 鼠标验证器
    - Linux: 使用 evdev 监听鼠标事件
    - Windows: TODO (使用 Hook API)
  - `command.rs` - 命令执行验证器
- **Agent 主程序** (`main.rs`)
  - 命令行参数解析
  - 验证器初始化
  - 事件循环
  - 自动重连

## 功能特性

### 已实现 ✅

1. **传输层**
   - ✅ WebSocket 传输（支持 ws:// 和 wss://）
   - ✅ TCP 传输（基于长度前缀的消息格式）
   - ✅ 自动重连机制
   - ✅ 错误处理和日志记录

2. **Linux 验证器**
   - ✅ 键盘验证器（evdev）
     - 自动发现所有键盘设备
     - 非阻塞事件监听
     - 按键名称匹配（不区分大小写）
   - ✅ 鼠标验证器（evdev）
     - 自动发现所有鼠标设备
     - 支持按键事件（左键、右键、中键）
     - 支持鼠标移动事件
   - ✅ 命令验证器
     - 异步命令执行
     - stdout/stderr 捕获
     - 退出码验证
     - 输出内容匹配

3. **Agent 特性**
   - ✅ CLI 参数解析
   - ✅ 可配置的验证器启用/禁用
   - ✅ 可配置的日志级别
   - ✅ 自动重连（可配置间隔）
   - ✅ 优雅的错误处理

### 待实现 📋

1. **Windows 验证器**
   - [ ] 键盘验证器（Windows Hook API）
   - [ ] 鼠标验证器（Windows Hook API）

2. **高级功能**
   - [ ] TLS/SSL 支持（WebSocket wss://）
   - [ ] 认证机制
   - [ ] 性能指标上报
   - [ ] 配置文件支持

## 使用方法

### 编译

```bash
cd guest-verifier
cargo build --release
```

### 运行

#### 基本用法（自动获取 VM ID）

```bash
# Agent 会自动尝试获取 VM ID:
# 1. 从 DMI/SMBIOS 读取 (如果 libvirt 配置了 sysinfo)
# 2. 从系统主机名读取
./target/release/verifier-agent -s ws://192.168.1.100:8080
```

自动获取 VM ID 的优先级:
1. **DMI/SMBIOS** (`/sys/class/dmi/id/product_serial`) - 推荐，需要 libvirt 配置
2. **系统主机名** (`/etc/hostname`) - 回退方案

#### 基本用法（手动指定 VM ID）

```bash
./target/release/verifier-agent -s ws://192.168.1.100:8080 --vm-id vm-001
```

**注意**: `--vm-id` 参数用于标识虚拟机，在多 VM 并发测试时用于区分不同客户端，确保一对一的事件-结果匹配。手动指定会覆盖自动检测结果。

#### 指定传输类型为 TCP

```bash
./target/release/verifier-agent -s 192.168.1.100:8080 -t tcp --vm-id vm-001
```

#### 只启用键盘和鼠标验证器

```bash
./target/release/verifier-agent -s ws://192.168.1.100:8080 -v keyboard -v mouse
```

#### 自定义日志级别

```bash
./target/release/verifier-agent -s ws://192.168.1.100:8080 -l debug
```

#### 禁用自动重连

```bash
./target/release/verifier-agent -s ws://192.168.1.100:8080 --auto-reconnect false
```

### 命令行选项

```
Options:
  -s, --server <SERVER>
          服务器地址 (例如: localhost:8080 或 ws://localhost:8080)
          [default: localhost:8080]

      --vm-id <VM_ID>
          虚拟机 ID（用于标识客户端）
          如果不指定，会自动尝试获取:
            1. 从 DMI/SMBIOS 读取 (libvirt sysinfo)
            2. 从系统主机名读取

  -t, --transport <TRANSPORT>
          传输类型 [websocket, tcp]
          [default: websocket]

  -v, --verifiers <VERIFIERS>
          启用的验证器类型 (可多次指定)
          [可选值: keyboard, mouse, command, all]

  -l, --log-level <LOG_LEVEL>
          日志级别
          [default: info]

      --auto-reconnect
          自动重连

      --reconnect-interval <RECONNECT_INTERVAL>
          重连间隔（秒）
          [default: 5]

  -h, --help
          显示帮助信息
```

## 事件格式

### 键盘事件

```json
{
  "event_type": "keyboard",
  "data": {
    "event_id": "uuid-12345",
    "key": "A",
    "timeout_ms": 5000
  },
  "timestamp": 1234567890
}
```

### 鼠标事件

```json
{
  "event_type": "mouse",
  "data": {
    "event_id": "uuid-12345",
    "action": "left_click",
    "timeout_ms": 5000
  },
  "timestamp": 1234567890
}
```

支持的鼠标操作：
- `left_click` / `left` - 左键点击
- `right_click` / `right` - 右键点击
- `middle_click` / `middle` - 中键点击
- `move` - 鼠标移动

### 命令事件

```json
{
  "event_type": "command",
  "data": {
    "event_id": "uuid-12345",
    "command": "ls",
    "args": ["-la", "/tmp"],
    "expected_exit_code": 0,
    "expected_stdout_contains": "total",
    "expected_stderr_contains": null
  },
  "timestamp": 1234567890
}
```

## 验证结果格式

```json
{
  "event_id": "uuid-12345",
  "verified": true,
  "timestamp": 1234567890,
  "latency_ms": 15,
  "details": {
    "key": "A",
    "platform": "linux",
    "method": "evdev"
  }
}
```

## Linux 权限要求

在 Linux 系统上，验证器需要访问 `/dev/input/event*` 设备。有两种方式：

### 方式 1: 以 root 运行（不推荐）

```bash
sudo ./target/release/verifier-agent -s ws://192.168.1.100:8080
```

### 方式 2: 添加用户到 input 组（推荐）

```bash
# 添加用户到 input 组
sudo usermod -a -G input $USER

# 重新登录使组成员生效
# 然后正常运行
./target/release/verifier-agent -s ws://192.168.1.100:8080
```

## 开发指南

### 添加新的验证器

1. 在 `verifier-agent/src/verifiers/` 创建新文件
2. 实现 `Verifier` trait
3. 在 `verifiers/mod.rs` 中导出
4. 在 `main.rs` 中添加初始化逻辑

### 添加新的传输层

1. 在 `verifier-core/src/transport/` 创建新文件
2. 实现 `VerifierTransport` trait
3. 在 `transport/mod.rs` 中导出
4. 在 `lib.rs` 中重新导出

## 故障排查

### 键盘/鼠标验证器初始化失败

**问题**: `初始化键盘验证器失败: 未找到键盘设备`

**解决方案**:
1. 确认用户在 `input` 组中: `groups $USER`
2. 检查设备文件权限: `ls -l /dev/input/event*`
3. 确认设备存在: `cat /proc/bus/input/devices`

### WebSocket 连接失败

**问题**: `WebSocket 连接失败: Connection refused`

**解决方案**:
1. 确认服务器地址正确
2. 确认服务器正在运行
3. 检查防火墙设置
4. 尝试使用 TCP 传输: `-t tcp`

### 事件超时

**问题**: 事件验证总是超时（verified: false）

**解决方案**:
1. 增加超时时间（在事件的 `timeout_ms` 字段）
2. 确认输入设备正常工作
3. 检查日志中的设备检测信息

## 代码统计

- **verifier-core**: ~500 行
  - transport/websocket.rs: ~200 行
  - transport/tcp.rs: ~200 行
  - 其他: ~100 行
- **verifier-agent**: ~950 行
  - verifiers/keyboard.rs: ~300 行
  - verifiers/mouse.rs: ~300 行
  - verifiers/command.rs: ~250 行
  - main.rs: ~350 行 (含 VM ID 自动检测)

**总计**: ~1,450 行代码

## ATP 平台集成

### 配置 libvirt 以支持 SMBIOS VM ID

为了让 Guest Agent 能够自动获取 VM ID，需要在创建虚拟机时配置 SMBIOS 信息：

```rust
// ATP Executor 创建 VM 时
fn create_vm_with_smbios(vm_name: &str) -> String {
    format!(r#"
<domain type='kvm'>
  <name>{vm_name}</name>
  <sysinfo type='smbios'>
    <system>
      <entry name='manufacturer'>OCloudView ATP</entry>
      <entry name='product'>ATP Test VM</entry>
      <entry name='serial'>{vm_name}</entry>
    </system>
  </sysinfo>
  <os>
    <type arch='x86_64'>hvm</type>
    <smbios mode='sysinfo'/>
  </os>
  <!-- 其他配置... -->
</domain>
"#, vm_name = vm_name)
}
```

验证 SMBIOS 配置（在 Guest 内）：

```bash
# 查看 product_serial
cat /sys/class/dmi/id/product_serial
# 应该输出: ubuntu-test-01

# 或使用 dmidecode (需要 root)
sudo dmidecode -s system-serial-number
```

### 确保 VM 名称一致性

**关键**: ATP 平台使用的 `domain_name` 必须与 Guest 获取的 `vm_id` 一致。

**方案 1: 使用 SMBIOS (推荐)**
```rust
// ATP Executor
let vm_name = "ubuntu-test-01";
create_vm_with_smbios(vm_name);  // SMBIOS serial = "ubuntu-test-01"

// Guest Agent
// 自动从 /sys/class/dmi/id/product_serial 读取 "ubuntu-test-01"

// 验证事件
verification_service.verify_event(vm_name, event, timeout).await?;
// ✅ vm_id 匹配
```

**方案 2: 使用主机名 (简单但需要确保一致)**
```rust
// ATP Executor 创建 VM 时设置主机名 = domain name
// 通过 cloud-init 或 guest-exec 设置

// Guest Agent
// 自动从 /etc/hostname 读取

// ✅ 只要主机名正确设置，vm_id 就会匹配
```

**方案 3: 手动指定 (最灵活)**
```bash
# Guest 内通过 systemd service 启动，从配置文件读取
# /etc/atp-verifier.conf
VM_ID=ubuntu-test-01

# 启动脚本
/usr/local/bin/verifier-agent \
    --server ws://192.168.122.1:8765 \
    --vm-id $VM_ID
```

### 完整集成示例

```rust
// atp-core/executor/src/runner.rs

use verification_server::{
    service::VerificationService,
    types::Event,
};

impl ScenarioRunner {
    async fn execute_keyboard_action(&mut self, action: &KeyboardAction) -> Result<()> {
        let vm_name = &action.target.vm_name; // 例如: "ubuntu-test-01"

        // 1. 发送键盘事件到 VM (通过 VirtIO Input)
        self.send_keyboard_to_vm(vm_name, &action.key).await?;

        // 2. 验证事件是否到达 Guest
        let event = Event {
            event_type: "keyboard".to_string(),
            data: serde_json::json!({
                "key": action.key,
                "timeout_ms": 5000,
            }),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        // 使用 vm_name 作为 vm_id 发送验证请求
        // Guest Agent 会自动从 SMBIOS 或主机名获取相同的 vm_id
        match self.verification_service
            .verify_event(vm_name, event, Some(Duration::from_secs(10)))
            .await
        {
            Ok(result) => {
                if result.verified {
                    info!("✅ 键盘事件已验证: latency={}ms", result.latency_ms);
                } else {
                    warn!("❌ 键盘事件验证失败");
                }
            }
            Err(VerificationError::Timeout) => {
                error!("⏱️  验证超时: Guest Agent 可能未运行或未响应");
            }
            Err(VerificationError::ClientNotConnected) => {
                error!("🔌 客户端未连接: Guest Agent 未连接到验证服务器");
            }
            Err(e) => {
                error!("❌ 验证失败: {}", e);
            }
        }

        Ok(())
    }
}
```

## 许可证

MIT OR Apache-2.0

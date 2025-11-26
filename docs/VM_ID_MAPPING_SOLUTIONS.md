# VM ID 映射方案设计

## 问题描述

ATP 平台通过 libvirt 管理虚拟机，知道 VM 的 domain name（如 "ubuntu-test-01"）。Guest Verifier Agent 运行在 Guest OS 内部，需要一个 VM ID 来标识自己。

**核心问题**: 如何让 Guest Verifier Agent 自动获取与 ATP 平台一致的 VM ID，避免手动配置？

## 方案对比

### 方案 1: 通过系统主机名自动获取 ✅ 最简单

**原理**: Guest OS 的主机名通常与 VM 名称一致

**实现**:
```rust
fn get_vm_id() -> Result<String> {
    // Linux
    let hostname = std::fs::read_to_string("/etc/hostname")?
        .trim()
        .to_string();
    Ok(hostname)
}
```

**优点**:
- 实现简单，无依赖
- 适用于大多数场景
- 性能开销小

**缺点**:
- 依赖主机名设置正确
- 如果主机名与 VM 名称不一致则失效

**适用场景**: 主机名与 libvirt domain name 一致的环境

---

### 方案 2: 通过 DMI/SMBIOS 获取 VM 名称 ✅ 推荐

**原理**: QEMU/KVM 可以通过 SMBIOS 传递 VM 元数据到 Guest

**Libvirt 配置**:
```xml
<domain type='kvm'>
  <name>ubuntu-test-01</name>
  <sysinfo type='smbios'>
    <system>
      <entry name='manufacturer'>OCloudView ATP</entry>
      <entry name='product'>Test VM</entry>
      <entry name='serial'>ubuntu-test-01</entry>
    </system>
  </sysinfo>
  ...
</domain>
```

**Guest 读取方式**:
```bash
# Linux
sudo dmidecode -s system-serial-number
# 输出: ubuntu-test-01

# 或者直接读取 sysfs
cat /sys/class/dmi/id/product_serial
```

**Rust 实现**:
```rust
fn get_vm_id_from_dmi() -> Result<String> {
    // 方式 1: 读取 sysfs (无需 root)
    if let Ok(serial) = std::fs::read_to_string("/sys/class/dmi/id/product_serial") {
        let vm_id = serial.trim().to_string();
        if !vm_id.is_empty() && vm_id != "Not Specified" {
            return Ok(vm_id);
        }
    }

    // 方式 2: 使用 dmidecode (需要 root)
    let output = std::process::Command::new("dmidecode")
        .args(["-s", "system-serial-number"])
        .output()?;

    let vm_id = String::from_utf8(output.stdout)?
        .trim()
        .to_string();

    Ok(vm_id)
}
```

**优点**:
- 可靠，数据由 hypervisor 注入
- 不依赖 Guest 配置
- 标准化方法

**缺点**:
- 需要 libvirt XML 配置
- sysfs 读取可能需要权限（通常不需要）

**适用场景**: 推荐用于生产环境

---

### 方案 3: 通过 cloud-init/virtio-serial 传递 ✅ 最灵活

**原理**: 使用 virtio-serial 通道或 cloud-init 传递 VM 元数据

**方式 A: cloud-init metadata**

Libvirt 配置:
```xml
<domain>
  ...
  <metadata>
    <cloudinit:config xmlns:cloudinit="http://cloudini.org/xmlns/libvirt/domain/1.0">
      <instance-id>ubuntu-test-01</instance-id>
    </cloudinit:config>
  </metadata>
</domain>
```

Guest 读取:
```bash
# cloud-init 会将 instance-id 写入
cat /var/lib/cloud/data/instance-id
```

**方式 B: virtio-serial 通道**

已在 ATP 平台实现的 virtio-serial 自定义协议可以复用：

```rust
// Host 端发送 VM 信息
let vm_info = json!({
    "type": "vm_info",
    "vm_id": "ubuntu-test-01",
    "domain_name": "ubuntu-test-01"
});
virtio_serial.send(vm_info).await?;

// Guest 端接收
let vm_info = virtio_serial.receive().await?;
let vm_id = vm_info["vm_id"].as_str().unwrap();
```

**优点**:
- 非常灵活，可以传递任意元数据
- 实时动态传递
- 可以传递额外信息（测试 ID、场景 ID 等）

**缺点**:
- 需要实现通信协议
- 依赖 virtio-serial 通道

**适用场景**: 需要传递复杂元数据的场景

---

### 方案 4: 通过 QEMU Guest Agent (QGA) 查询 🔄 复杂但强大

**原理**: QGA 可以查询 Guest 信息，也可以由 Host 通过 QGA 设置环境变量

**方式 A: QGA 查询主机名**
```rust
// Guest 内通过 QGA 客户端库查询
use qga_client::QgaClient;

let qga = QgaClient::new("/dev/virtio-ports/org.qemu.guest_agent.0")?;
let hostname = qga.exec("hostname")?;
```

**方式 B: Host 通过 QGA 设置环境变量**
```rust
// Host 端
let qga = QgaConnection::new(domain_name)?;
qga.exec(&format!("echo 'export ATP_VM_ID={}' >> /etc/environment", vm_id))?;

// Guest 端读取
let vm_id = std::env::var("ATP_VM_ID")?;
```

**优点**:
- QGA 已广泛部署
- 可以双向通信

**缺点**:
- 需要 QGA 运行
- 实现复杂
- 环境变量方式需要重新登录

**适用场景**: 已有 QGA 基础设施的环境

---

## 推荐方案组合

### 阶段 1: 短期方案（当前实现）

**使用方案 1 (主机名) + 手动指定 fallback**

```rust
fn get_vm_id(manual_override: Option<String>) -> Result<String> {
    // 1. 优先使用手动指定
    if let Some(vm_id) = manual_override {
        info!("使用手动指定的 VM ID: {}", vm_id);
        return Ok(vm_id);
    }

    // 2. 尝试读取主机名
    if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
        let vm_id = hostname.trim().to_string();
        if !vm_id.is_empty() {
            info!("使用主机名作为 VM ID: {}", vm_id);
            return Ok(vm_id);
        }
    }

    // 3. 失败则返回错误
    Err(anyhow::anyhow!("无法自动获取 VM ID，请使用 --vm-id 手动指定"))
}
```

**CLI 调整**:
```bash
# 自动获取（主机名）
./verifier-agent --server ws://host:8765

# 手动指定（覆盖）
./verifier-agent --server ws://host:8765 --vm-id ubuntu-test-01
```

---

### 阶段 2: 中期方案（推荐生产使用）

**使用方案 2 (DMI/SMBIOS) 作为主要方案**

```rust
fn get_vm_id(manual_override: Option<String>) -> Result<String> {
    // 1. 优先使用手动指定
    if let Some(vm_id) = manual_override {
        info!("使用手动指定的 VM ID: {}", vm_id);
        return Ok(vm_id);
    }

    // 2. 尝试从 DMI/SMBIOS 读取
    if let Ok(vm_id) = get_vm_id_from_dmi() {
        if !vm_id.is_empty() && vm_id != "Not Specified" {
            info!("从 DMI/SMBIOS 获取 VM ID: {}", vm_id);
            return Ok(vm_id);
        }
    }

    // 3. 回退到主机名
    if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
        let vm_id = hostname.trim().to_string();
        if !vm_id.is_empty() {
            warn!("DMI 不可用，使用主机名作为 VM ID: {}", vm_id);
            return Ok(vm_id);
        }
    }

    // 4. 失败则返回错误
    Err(anyhow::anyhow!("无法自动获取 VM ID，请使用 --vm-id 手动指定"))
}

fn get_vm_id_from_dmi() -> Result<String> {
    // 优先使用 sysfs (不需要 root)
    if let Ok(serial) = std::fs::read_to_string("/sys/class/dmi/id/product_serial") {
        return Ok(serial.trim().to_string());
    }

    // 如果 sysfs 不可用，尝试其他字段
    if let Ok(uuid) = std::fs::read_to_string("/sys/class/dmi/id/product_uuid") {
        return Ok(uuid.trim().to_string());
    }

    Err(anyhow::anyhow!("无法从 DMI 读取 VM ID"))
}
```

**ATP 平台端配置**:
```rust
// 在创建 VM 时自动添加 SMBIOS 配置
let domain_xml = format!(r#"
<domain type='kvm'>
  <name>{vm_name}</name>
  <sysinfo type='smbios'>
    <system>
      <entry name='manufacturer'>OCloudView ATP</entry>
      <entry name='product'>ATP Test VM</entry>
      <entry name='serial'>{vm_name}</entry>
      <entry name='uuid'>{vm_name}</entry>
    </system>
  </sysinfo>
  <os>
    <smbios mode='sysinfo'/>
  </os>
  ...
</domain>
"#, vm_name = domain_name);
```

---

### 阶段 3: 长期方案（最灵活）

**使用方案 3 (virtio-serial) 实时传递**

ATP 平台已实现 virtio-serial 自定义协议，可以复用：

**Host 端 (ATP Executor)**:
```rust
impl ScenarioRunner {
    async fn start_verification_agent(&self, vm_name: &str) -> Result<()> {
        // 1. 通过 virtio-serial 发送 VM 信息
        let vm_info = json!({
            "type": "vm_init",
            "vm_id": vm_name,
            "test_id": self.test_id,
            "scenario_id": self.scenario_id,
        });

        self.virtio_serial_manager
            .send_to_guest(vm_name, vm_info)
            .await?;

        // 2. Guest Agent 会自动接收并使用
        Ok(())
    }
}
```

**Guest 端 (Verifier Agent)**:
```rust
async fn get_vm_id_from_virtio_serial() -> Result<String> {
    // 连接到 virtio-serial 端口
    let mut serial = tokio::fs::File::open("/dev/virtio-ports/org.atp.config").await?;

    // 读取配置消息
    let mut buffer = vec![0u8; 4096];
    let n = serial.read(&mut buffer).await?;

    // 解析 JSON
    let config: serde_json::Value = serde_json::from_slice(&buffer[..n])?;
    let vm_id = config["vm_id"].as_str()
        .ok_or_else(|| anyhow::anyhow!("配置中没有 vm_id"))?
        .to_string();

    info!("从 virtio-serial 获取 VM ID: {}", vm_id);
    Ok(vm_id)
}
```

---

## 实现优先级

### 立即实现 (Phase 1) - 本次更新

✅ **自动获取主机名 + 手动指定 fallback**

- 修改 `--vm-id` 参数为可选
- 实现自动主机名获取
- 保留手动指定选项

**代码变更**:
```rust
// Args 结构体已经有 vm_id: Option<String>

// 在 main 函数中添加自动获取逻辑
let vm_id = match args.vm_id {
    Some(id) => {
        info!("使用手动指定的 VM ID: {}", id);
        id
    }
    None => {
        info!("尝试自动获取 VM ID...");
        get_hostname_as_vm_id()?
    }
};
```

### 短期实现 (Phase 2) - 1-2 周

✅ **添加 DMI/SMBIOS 支持**

- 实现 `get_vm_id_from_dmi()`
- 添加到自动获取逻辑
- 更新 ATP 平台创建 VM 时添加 SMBIOS 配置

### 中期实现 (Phase 3) - 1-2 月

🔄 **集成 virtio-serial 配置通道**

- 复用现有 virtio-serial 实现
- Host 端自动发送 VM 配置
- Guest 端优先从 virtio-serial 读取

---

## ATP 平台端对应关系

### 当前实现

ATP Executor 发送验证事件时需要指定 VM ID：

```rust
impl ScenarioRunner {
    async fn execute_keyboard_action(&mut self, action: &KeyboardAction) -> Result<()> {
        let vm_name = &action.target.vm_name; // 例如: "ubuntu-test-01"

        // 1. 发送键盘事件到 VM
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
        match self.verification_service
            .verify_event(vm_name, event, Some(Duration::from_secs(10)))
            .await
        {
            Ok(result) => {
                if result.verified {
                    info!("键盘事件已验证: latency={}ms", result.latency_ms);
                } else {
                    warn!("键盘事件验证失败");
                }
            }
            Err(e) => {
                error!("验证失败: {}", e);
            }
        }

        Ok(())
    }
}
```

### 确保一致性

**关键**: ATP 使用的 `vm_name` 必须与 Guest 获取的 `vm_id` 一致

**方案 1 (主机名)**:
- ATP 创建 VM 时设置主机名 = domain name
- Guest 读取主机名作为 vm_id
- ✅ 自动一致

**方案 2 (SMBIOS)**:
- ATP 创建 VM 时设置 SMBIOS serial = domain name
- Guest 读取 SMBIOS serial 作为 vm_id
- ✅ 自动一致，更可靠

**方案 3 (virtio-serial)**:
- ATP 启动后通过 virtio-serial 发送 domain name
- Guest 接收并使用
- ✅ 实时动态，最灵活

---

## 示例：完整流程

### 使用方案 2 (SMBIOS) 的完整流程

**1. ATP 平台创建 VM**

```rust
// atp-core/executor/src/runner.rs

impl ScenarioRunner {
    async fn create_test_vm(&self, vm_name: &str) -> Result<()> {
        let domain_xml = format!(r#"
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
"#, vm_name = vm_name);

        self.hypervisor.create_domain(&domain_xml).await?;
        Ok(())
    }
}
```

**2. Guest 内启动 Verifier Agent**

```bash
# Guest OS 内 (systemd service 或 init script)
/usr/local/bin/verifier-agent \
    --server ws://192.168.122.1:8765 \
    --log-level info

# Agent 会自动读取 SMBIOS 获取 VM ID
# 从 /sys/class/dmi/id/product_serial 读取到 "ubuntu-test-01"
```

**3. ATP 平台发送验证事件**

```rust
// ATP Executor 执行测试
let vm_name = "ubuntu-test-01";

// 发送键盘事件
self.keyboard_manager.send_key(vm_name, "a").await?;

// 验证事件（使用相同的 vm_name）
self.verification_service
    .verify_event(vm_name, keyboard_event, timeout)
    .await?;
```

**4. 验证成功**

```
ATP: verify_event(vm_id="ubuntu-test-01", ...)
  ↓
VerificationService: 创建 event_id, 发送到 client "ubuntu-test-01"
  ↓
ClientManager: 查找 clients["ubuntu-test-01"] ✅
  ↓
WebSocket/TCP: 发送事件到 Guest
  ↓
Guest Agent: 接收事件, vm_id="ubuntu-test-01" (从 SMBIOS 读取)
  ↓
Guest Agent: 验证成功, 返回结果
  ↓
VerificationService: 匹配 event_id, 返回结果给 ATP
```

---

## 建议

### 当前阶段

1. ✅ **立即实现**: 添加主机名自动获取 + 手动 fallback
2. 📝 **文档更新**: 说明 VM 创建时需要设置正确的主机名

### 下一步

1. 🔄 **SMBIOS 支持**: 实现 DMI 读取，提升可靠性
2. 🔄 **ATP 集成**: 在 VM 创建模板中添加 SMBIOS 配置

### 长期规划

1. 🔮 **virtio-serial**: 实现动态配置传递
2. 🔮 **元数据扩展**: 传递更多测试相关信息（test_id, scenario_id 等）

---

## 总结

| 方案 | 可靠性 | 实现难度 | 灵活性 | 推荐阶段 |
|------|--------|----------|--------|----------|
| 主机名 | ⭐⭐⭐ | ⭐ (简单) | ⭐⭐ | Phase 1 ✅ |
| SMBIOS | ⭐⭐⭐⭐⭐ | ⭐⭐ (中等) | ⭐⭐⭐ | Phase 2 🎯 |
| cloud-init | ⭐⭐⭐⭐ | ⭐⭐⭐ (复杂) | ⭐⭐⭐ | 可选 |
| virtio-serial | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ (复杂) | ⭐⭐⭐⭐⭐ | Phase 3 🔮 |
| QGA | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ (复杂) | ⭐⭐⭐⭐ | 不推荐 |

**最佳实践**:
- 短期使用**主机名自动获取**
- 中期迁移到 **SMBIOS**
- 长期考虑 **virtio-serial** 动态配置

# 快速开始指南

本指南帮助您快速搭建 OCloudView ATP（自动化测试平台）的开发环境并运行第一个测试。

## 前置条件

- **操作系统**: Linux (Ubuntu 20.04+ 推荐) 或 WSL2
- **Rust**: 1.70 或更高版本
- **Libvirt**: 7.0 或更高版本
- **QEMU/KVM**: 6.0 或更高版本

## 快速安装

### 1. 安装系统依赖

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y qemu-kvm libvirt-daemon-system libvirt-clients bridge-utils \
                    build-essential pkg-config libssl-dev

# 安装 Rust (如果未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 添加用户到 libvirt 组
sudo usermod -aG libvirt $USER
sudo usermod -aG kvm $USER

# 应用组权限 (需要重新登录或使用)
newgrp libvirt
```

### 2. 克隆项目

```bash
git clone https://github.com/wooveep/ocloudview-atp.git
cd ocloudview-atp
```

### 3. 构建项目

```bash
# 构建整个工作区
cargo build --workspace

# 或构建 release 版本
cargo build --workspace --release
```

### 4. 运行测试

```bash
# 运行单元测试
cargo test --workspace

# 检查代码
cargo clippy --workspace
```

## 第一个例子：使用传输层

### 创建传输管理器并连接到主机

创建文件 `examples/basic_transport.rs`:

```rust
use atp_transport::{TransportManager, TransportConfig, HostInfo};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 创建传输管理器
    let config = TransportConfig::default();
    let manager = TransportManager::new(config);

    // 2. 添加主机
    let host = HostInfo::new("local", "localhost")
        .with_uri("qemu:///system");
    manager.add_host(host).await?;

    // 3. 在主机上执行任务
    let result = manager.execute_on_host("local", |conn| async move {
        println!("✓ 成功连接到主机: {:?}", conn.host_info());
        Ok(())
    }).await?;

    println!("执行结果: {:?}", result);

    Ok(())
}
```

运行：

```bash
cargo run --example basic_transport
```

## 使用 QMP 协议发送按键

创建文件 `examples/qmp_keyboard.rs`:

```rust
use atp_transport::{TransportManager, HostInfo};
use atp_protocol::{QmpProtocolBuilder, Protocol};
use virt::connect::Connect;
use virt::domain::Domain;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 连接到 libvirt
    let conn = Connect::open("qemu:///system")?;

    // 2. 获取虚拟机（假设名为 "test-vm"）
    let domain = Domain::lookup_by_name(&conn, "test-vm")?;

    // 3. 创建 QMP 协议
    let builder = QmpProtocolBuilder::new();
    let mut qmp = builder.build();

    // 4. 连接到虚拟机
    qmp.connect(&domain).await?;
    println!("✓ 已连接到 QMP");

    // 5. 发送按键
    qmp.send_key("a").await?;
    println!("✓ 已发送按键 'a'");

    // 6. 发送按键组合
    qmp.send_keys(vec!["ctrl", "c"], None).await?;
    println!("✓ 已发送 Ctrl+C");

    // 7. 查询状态
    let status = qmp.query_status().await?;
    println!("✓ 虚拟机状态: {:?}", status);

    Ok(())
}
```

运行（需要有名为 "test-vm" 的虚拟机在运行）：

```bash
cargo run --example qmp_keyboard
```

## 使用 QGA 协议执行命令

创建文件 `examples/qga_command.rs`:

```rust
use atp_protocol::{QgaProtocolBuilder, Protocol};
use virt::connect::Connect;
use virt::domain::Domain;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 连接到 libvirt
    let conn = Connect::open("qemu:///system")?;

    // 2. 获取虚拟机（需要安装并运行 qemu-guest-agent）
    let domain = Domain::lookup_by_name(&conn, "test-vm")?;

    // 3. 创建 QGA 协议
    let builder = QgaProtocolBuilder::new().with_timeout(30);
    let mut qga = builder.build();

    // 4. 连接到 Guest Agent
    qga.connect(&domain).await?;
    println!("✓ 已连接到 QGA");

    // 5. 测试连通性
    qga.ping().await?;
    println!("✓ QGA Ping 成功");

    // 6. 执行 Shell 命令
    let result = qga.exec_shell("ls -la /tmp").await?;

    if let Some(stdout) = result.decode_stdout() {
        println!("命令输出:\n{}", stdout);
    }

    if let Some(exit_code) = result.exit_code {
        println!("退出码: {}", exit_code);
    }

    Ok(())
}
```

运行（需要虚拟机内安装并运行 qemu-guest-agent）：

```bash
cargo run --example qga_command
```

## 使用 VDI 平台 API

创建文件 `examples/vdi_basic.rs`:

```rust
use atp_vdiplatform::VdiClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 创建 VDI 客户端
    let mut client = VdiClient::new("http://192.168.1.11:8088");

    // 2. 登录
    client.login("admin", "password").await?;
    println!("✓ 登录成功");

    // 3. 查询主机列表
    let hosts = client.host().list().await?;
    println!("✓ 查询到 {} 个主机", hosts.len());

    for host in hosts {
        println!("  - 主机: {} ({})", host.name, host.ip);
    }

    // 4. 查询虚拟机列表
    let domains = client.domain().list().await?;
    println!("✓ 查询到 {} 个虚拟机", domains.len());

    Ok(())
}
```

运行（需要 VDI 平台访问权限）：

```bash
cargo run --example vdi_basic
```

## 运行测试场景

### 1. 创建测试场景文件

创建文件 `examples/scenarios/basic-test.yaml`:

```yaml
name: "基础键盘测试"
description: "测试虚拟机键盘输入功能"

steps:
  # 步骤 1: 连接到虚拟机
  - type: virtualization_action
    action: connect
    domain_id: "test-vm"

  # 步骤 2: 发送键盘输入
  - type: virtualization_action
    action: send_keyboard
    text: "Hello from ATP!"

  # 步骤 3: 等待
  - type: wait
    duration: 2s

  # 步骤 4: 执行命令验证
  - type: virtualization_action
    action: execute_command
    command: "echo 'Test complete'"
```

### 2. 执行场景

```rust
use atp_orchestrator::TestScenario;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载场景
    let scenario = TestScenario::from_yaml("examples/scenarios/basic-test.yaml")?;

    println!("场景名称: {}", scenario.name);
    println!("场景描述: {:?}", scenario.description);
    println!("步骤数量: {}", scenario.steps.len());

    // TODO: 场景执行功能正在开发中

    Ok(())
}
```

## 验证安装

运行以下命令验证安装：

```bash
# 1. 检查 Rust 版本
rustc --version
# 应输出: rustc 1.70.0 或更高

# 2. 检查 Libvirt
virsh version
# 应输出 libvirt 版本信息

# 3. 检查虚拟机列表
virsh list --all

# 4. 检查项目编译
cargo check --workspace
# 应无错误

# 5. 运行测试
cargo test --workspace
# 所有测试应通过
```

## 常见问题

### Q1: 权限被拒绝

```bash
# 错误: permission denied
# 解决: 确保用户在 libvirt 组中
groups | grep libvirt

# 如果不在，执行
sudo usermod -aG libvirt $USER
newgrp libvirt
```

### Q2: 无法连接到 libvirt

```bash
# 错误: Failed to connect socket
# 解决: 启动 libvirt 服务
sudo systemctl start libvirtd
sudo systemctl enable libvirtd
```

### Q3: 虚拟机未运行

```bash
# 启动虚拟机
virsh start test-vm

# 查看虚拟机状态
virsh list --all
```

### Q4: QGA 连接失败

虚拟机内需要安装并运行 QEMU Guest Agent：

```bash
# Ubuntu/Debian 虚拟机内
sudo apt install qemu-guest-agent
sudo systemctl start qemu-guest-agent
sudo systemctl enable qemu-guest-agent

# CentOS/RHEL 虚拟机内
sudo yum install qemu-guest-agent
sudo systemctl start qemu-guest-agent
sudo systemctl enable qemu-guest-agent
```

### Q5: 编译错误 - 找不到 qemu_agent_command

确保启用了 `qemu` 特性：

```toml
# 检查 atp-core/Cargo.toml
[workspace.dependencies]
virt = { version = "0.4", features = ["qemu"] }
```

## 下一步

现在您已经完成了基础设置，可以：

1. **阅读完整文档**:
   - [分层架构设计](docs/LAYERED_ARCHITECTURE.md)
   - [开发指南](docs/DEVELOPMENT.md)
   - [QGA 使用指南](docs/QGA_GUIDE.md)

2. **查看示例**:
   - 浏览 `examples/` 目录下的示例代码
   - 运行 `cargo run --example <示例名>`

3. **开始开发**:
   - 查看 [TODO.md](TODO.md) 了解待实现功能
   - 参考 [DEVELOPMENT.md](docs/DEVELOPMENT.md) 了解开发流程
   - 阅读 [贡献指南](docs/DEVELOPMENT.md#贡献指南)

4. **运行测试**:
   - `cargo test --workspace` 运行所有测试
   - `cargo test --package atp-transport` 运行特定模块测试

## 获取帮助

- **文档**: [docs/](docs/)
- **问题反馈**: GitHub Issues
- **代码示例**: [examples/](examples/)

---

**欢迎使用 OCloudView ATP！**

如果遇到问题，请查阅 [开发指南](docs/DEVELOPMENT.md) 或提交 Issue。

---

**最后更新**: 2025-11-24
**维护者**: OCloudView ATP Team

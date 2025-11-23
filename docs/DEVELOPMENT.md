# 开发指南

## 环境准备

### 系统要求

**开发环境：**
- Linux (推荐 Ubuntu 20.04+ 或 WSL2)
- Rust 1.70+
- QEMU/KVM 6.0+
- Libvirt 7.0+

**可选（用于 VDI 平台测试）：**
- OCloudView VDI 平台访问权限
- HTTP 客户端工具

### 安装依赖

#### Ubuntu/Debian

```bash
# 安装 QEMU/KVM 和 Libvirt
sudo apt update
sudo apt install qemu-kvm libvirt-daemon-system libvirt-clients bridge-utils

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装开发工具
sudo apt install build-essential pkg-config libssl-dev

# 将当前用户加入 libvirt 组
sudo usermod -aG libvirt $USER
sudo usermod -aG kvm $USER

# 重新登录以应用组权限
newgrp libvirt
```

## 项目结构

项目采用 Cargo Workspace 管理，包含以下核心模块：

```
ocloudview-atp/
├── atp-core/                 # 核心框架 (Workspace)
│   ├── transport/            # 传输层：连接管理
│   ├── protocol/             # 协议层：QMP/QGA/VirtioSerial
│   ├── vdiplatform/          # VDI 平台客户端
│   ├── orchestrator/         # 场景编排器
│   └── executor/             # 执行器（开发中）
├── examples/                 # 示例和测试场景
├── docs/                     # 文档
├── config/                   # 配置文件
└── TODO.md                   # 开发任务清单
```

### 核心模块说明

#### 1. transport - 传输层
连接管理和主机通信：
- **HostConnection**: 单主机连接管理（自动重连、心跳）
- **ConnectionPool**: 连接池（多策略、扩缩容）
- **TransportManager**: 多主机管理和并发执行

#### 2. protocol - 协议层
虚拟化协议实现：
- **QMP Protocol**: QEMU Machine Protocol（键盘、鼠标）
- **QGA Protocol**: QEMU Guest Agent（命令执行）
- **VirtioSerial**: 自定义协议（开发中）
- **SPICE**: 预留接口

#### 3. vdiplatform - VDI 平台
OCloudView VDI 平台集成：
- **VdiClient**: HTTP API 客户端
- **DomainApi**: 虚拟机管理
- **DeskPoolApi**: 桌面池管理
- **HostApi/ModelApi/UserApi**: 其他 API

#### 4. orchestrator - 场景编排
测试场景定义和编排：
- **TestScenario**: 场景定义（YAML/JSON）
- **ScenarioExecutor**: 场景执行引擎
- **VdiVirtualizationAdapter**: VDI 与虚拟化层适配

## 快速开始

### 1. 克隆项目

```bash
git clone https://github.com/wooveep/ocloudview-atp.git
cd ocloudview-atp
```

### 2. 构建项目

```bash
# 构建整个工作区
cargo build --workspace

# 构建特定模块
cd atp-core
cargo build --package atp-transport
cargo build --package atp-protocol
cargo build --package atp-vdiplatform

# 运行测试
cargo test --workspace

# 检查代码
cargo clippy --workspace
cargo fmt --check
```

### 3. 验证环境

```bash
# 检查 Libvirt 连接
virsh list --all

# 检查 QEMU 版本
qemu-system-x86_64 --version

# 测试虚拟机操作
virsh start test-vm-01
virsh list --all
```

## 开发工作流

### 传输层开发

#### 创建和使用传输管理器

```rust
use atp_transport::{TransportManager, TransportConfig, HostInfo};

#[tokio::main]
async fn main() -> Result<()> {
    // 创建传输管理器
    let config = TransportConfig::default();
    let manager = TransportManager::new(config);

    // 添加主机
    let host = HostInfo::new("host1", "192.168.1.100")
        .with_uri("qemu+tcp://192.168.1.100/system");
    manager.add_host(host).await?;

    // 执行任务
    manager.execute_on_host("host1", |conn| async move {
        // 使用连接
        println!("连接成功: {:?}", conn.host_info());
        Ok(())
    }).await?;

    Ok(())
}
```

#### 配置连接池

```rust
use atp_transport::{TransportConfig, PoolConfig, SelectionStrategy};

let config = TransportConfig {
    pool: PoolConfig {
        max_connections_per_host: 10,
        min_connections_per_host: 2,
        idle_timeout: 300,
        selection_strategy: SelectionStrategy::LeastConnections,
    },
    connect_timeout: 30,
    heartbeat_interval: 60,
    auto_reconnect: true,
    ..Default::default()
};
```

### 协议层开发

#### 使用 QMP 协议

```rust
use atp_protocol::{QmpProtocol, QmpProtocolBuilder, Protocol};

#[tokio::main]
async fn main() -> Result<()> {
    // 创建协议
    let builder = QmpProtocolBuilder::new();
    let mut qmp = builder.build();

    // 连接到 domain
    qmp.connect(&domain).await?;

    // 发送按键
    qmp.send_key("a").await?;
    qmp.send_keys(vec!["ctrl", "c"], None).await?;

    // 查询状态
    let status = qmp.query_status().await?;
    println!("VM 状态: {:?}", status);

    Ok(())
}
```

#### 使用 QGA 协议

```rust
use atp_protocol::{QgaProtocol, QgaProtocolBuilder, GuestExecCommand};

#[tokio::main]
async fn main() -> Result<()> {
    let builder = QgaProtocolBuilder::new().with_timeout(60);
    let mut qga = builder.build();

    qga.connect(&domain).await?;

    // 执行 shell 命令
    let result = qga.exec_shell("ls -la /tmp").await?;
    if let Some(stdout) = result.decode_stdout() {
        println!("输出: {}", stdout);
    }

    Ok(())
}
```

### VDI 平台开发

#### 使用 VDI 客户端

```rust
use atp_vdiplatform::VdiClient;

#[tokio::main]
async fn main() -> Result<()> {
    // 创建客户端
    let mut client = VdiClient::new("http://192.168.1.11:8088");

    // 登录
    client.login("admin", "password").await?;

    // 创建虚拟机
    let domain = client.domain()
        .create(CreateDomainRequest {
            name: "test-vm".to_string(),
            // ... 其他参数
        })
        .await?;

    // 启动虚拟机
    client.domain().start(&domain.id).await?;

    Ok(())
}
```

### 场景编排开发

#### 定义测试场景（YAML）

```yaml
name: "键盘输入测试"
description: "测试虚拟机键盘输入功能"

steps:
  # 连接到虚拟机
  - type: virtualization_action
    action: connect
    domain_id: "vm-001"

  # 发送键盘输入
  - type: virtualization_action
    action: send_keyboard
    text: "Hello World"

  # 等待
  - type: wait
    duration: 2s

  # 验证
  - type: verify
    condition: command_success
    domain_id: "vm-001"
```

#### 执行场景

```rust
use atp_orchestrator::{TestScenario, ScenarioExecutor};

#[tokio::main]
async fn main() -> Result<()> {
    // 加载场景
    let scenario = TestScenario::from_yaml("examples/basic-keyboard.yaml")?;

    // 创建执行器
    let executor = ScenarioExecutor::new(
        vdi_client,
        transport_manager,
        protocol_registry,
    );

    // 执行场景
    let report = executor.execute(&scenario).await?;

    println!("测试报告: {:#?}", report);

    Ok(())
}
```

## 调试技巧

### 1. 启用详细日志

```bash
# 设置日志级别
export RUST_LOG=debug

# 或针对特定模块
export RUST_LOG=atp_transport=debug,atp_protocol=info

# 运行程序
cargo run
```

### 2. 使用 Rust 调试器

```bash
# 安装 rust-gdb
rustup component add rust-src

# 调试程序
rust-gdb target/debug/your-program

# 或使用 VS Code / CLion 的图形调试器
```

### 3. 检查 Libvirt 连接

```bash
# 测试连接
virsh -c qemu+tcp://192.168.1.100/system list

# 查看虚拟机详情
virsh dominfo vm-name

# 查看 QMP Socket 路径
virsh qemu-monitor-command vm-name --hmp info version
```

### 4. 监控 QMP 通信

使用 `socat` 创建 QMP 代理：

```bash
# 创建代理（需要 root 权限）
sudo socat -v \
  UNIX-LISTEN:/tmp/qmp-monitor.sock,fork \
  UNIX-CONNECT:/var/lib/libvirt/qemu/domain-1-vm/monitor.sock

# 然后修改程序连接到 /tmp/qmp-monitor.sock
```

### 5. 测试 QGA 命令

```bash
# 手动执行 QGA 命令
virsh qemu-agent-command vm-name '{"execute":"guest-ping"}'

# 获取 Guest 信息
virsh qemu-agent-command vm-name '{"execute":"guest-info"}'

# 执行命令
virsh qemu-agent-command vm-name \
  '{"execute":"guest-exec","arguments":{"path":"/bin/sh","arg":["-c","ls -la"],"capture-output":true}}'
```

## 常见问题

### Q1: 编译错误 - virt crate 找不到 qemu_agent_command

**原因**: 未启用 `qemu` 特性

**解决方案**:
```toml
# atp-core/Cargo.toml
[workspace.dependencies]
virt = { version = "0.4", features = ["qemu"] }
```

### Q2: 连接池连接数不足

**解决方案**:
```rust
let config = TransportConfig {
    pool: PoolConfig {
        max_connections_per_host: 20,  // 增加最大连接数
        min_connections_per_host: 5,   // 增加最小连接数
        ..Default::default()
    },
    ..Default::default()
};
```

### Q3: QMP Socket 路径错误

**问题**: 当前实现使用简化的路径模式

**临时解决方案**:
```bash
# 1. 查找实际路径
sudo find /var/lib/libvirt/qemu -name "monitor.sock"

# 2. 创建符号链接（临时）
sudo ln -s /actual/path/monitor.sock /expected/path/monitor.sock
```

**长期解决方案**: 等待从 libvirt XML 读取路径的功能实现

### Q4: 权限错误

```bash
# 确保用户在 libvirt 组
groups | grep libvirt

# 如果不在，添加用户
sudo usermod -aG libvirt $USER
newgrp libvirt

# 检查 socket 权限
sudo ls -l /var/lib/libvirt/qemu/
```

### Q5: VDI API 连接超时

**解决方案**:
```rust
let config = VdiConfig {
    timeout: Duration::from_secs(30),  // 增加超时时间
    retry_times: 3,                     // 启用重试
    ..Default::default()
};
```

## 性能优化

### 1. 连接池优化

```rust
// 使用最少连接策略以获得更好的负载均衡
let config = PoolConfig {
    selection_strategy: SelectionStrategy::LeastConnections,
    max_connections_per_host: 10,
    ..Default::default()
};
```

### 2. 并发执行优化

```rust
// 在多个主机上并发执行任务
let results = manager.execute_on_hosts(
    &["host1", "host2", "host3"],
    |conn| async move {
        // 任务逻辑
        Ok(())
    }
).await;
```

### 3. 协议调优

```rust
// QGA: 调整超时时间
let qga = QgaProtocolBuilder::new()
    .with_timeout(120)  // 对于长时间运行的命令
    .build();

// 批量操作时减少等待时间
```

## 测试指南

### 单元测试

```bash
# 运行所有单元测试
cargo test --workspace

# 运行特定模块测试
cargo test --package atp-transport
cargo test --package atp-protocol

# 运行特定测试
cargo test test_connection_pool

# 显示测试输出
cargo test -- --nocapture
```

### 集成测试

```bash
# 运行集成测试（需要 libvirt 环境）
cargo test --test integration_tests -- --ignored

# 或使用环境变量
TEST_INTEGRATION=1 cargo test
```

### 性能测试

```bash
# 运行基准测试
cargo bench --workspace

# 或针对特定模块
cargo bench --package atp-transport
```

## 代码规范

### Rust 代码风格

```bash
# 格式化代码
cargo fmt --all

# 检查代码质量
cargo clippy --all -- -D warnings

# 检查未使用的依赖
cargo machete
```

### 提交规范

遵循 Conventional Commits：

```
feat: 添加新功能
fix: 修复 bug
docs: 文档更新
refactor: 代码重构
test: 测试相关
chore: 构建/工具链更新
```

### 文档规范

- 所有公共 API 必须有文档注释
- 使用 `cargo doc` 生成文档
- 包含使用示例

```rust
/// 创建新的传输管理器
///
/// # 示例
///
/// ```
/// use atp_transport::TransportManager;
///
/// let manager = TransportManager::new(Default::default());
/// ```
pub fn new(config: TransportConfig) -> Self {
    // ...
}
```

## 贡献指南

1. **Fork 项目** 并创建特性分支
2. **遵循代码规范** （rustfmt + clippy）
3. **添加测试** 覆盖新功能
4. **更新文档** 包括 README 和 API 文档
5. **提交 PR** 并说明变更内容

### PR 检查清单

- [ ] 代码通过 `cargo fmt` 和 `cargo clippy`
- [ ] 所有测试通过
- [ ] 添加了必要的单元测试
- [ ] 更新了相关文档
- [ ] 更新了 TODO.md（如适用）

## 参考资源

### 官方文档
- [QEMU QMP 协议](https://qemu.readthedocs.io/en/latest/interop/qmp-intro.html)
- [Libvirt API](https://libvirt.org/html/index.html)
- [Tokio 异步运行时](https://tokio.rs/)
- [Rust 异步编程](https://rust-lang.github.io/async-book/)

### 项目文档
- [分层架构设计](LAYERED_ARCHITECTURE.md)
- [连接模式设计](CONNECTION_MODES.md)
- [阶段1实现总结](STAGE1_TRANSPORT_IMPLEMENTATION.md)
- [阶段2实现总结](STAGE2_PROTOCOL_IMPLEMENTATION.md)
- [VDI平台测试](VDI_PLATFORM_TESTING.md)
- [QGA使用指南](QGA_GUIDE.md)

### 相关库
- [virt-rs](https://docs.rs/virt/) - Libvirt Rust 绑定
- [reqwest](https://docs.rs/reqwest/) - HTTP 客户端
- [serde](https://serde.rs/) - 序列化/反序列化
- [tokio](https://tokio.rs/) - 异步运行时

---

**最后更新**: 2025-11-24
**维护者**: OCloudView ATP Team

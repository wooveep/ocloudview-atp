# OCloudView ATP - 重构计划

## 当前状态分析

### 现有代码结构
```
test-controller/src/
├── qmp/              # QMP 协议实现
├── qga/              # QGA 协议实现
├── libvirt/          # Libvirt 管理
├── keymapping/       # 键值映射
├── vm_actor/         # VM Actor 模型
└── orchestrator/     # 测试编排器
```

### 问题
1. 缺乏清晰的分层
2. Libvirt 连接没有池化
3. 不支持多主机管理
4. 协议层耦合在具体实现中
5. 缺少统一的应用层接口

## 新架构设计

### 分层模型
```
Layer 4: 应用层 (Application)
  ├── CLI 接口
  ├── HTTP API
  └── 测试场景

Layer 3: 协议层 (Protocol)
  ├── Protocol Trait (统一接口)
  ├── QMP 实现
  ├── QGA 实现
  └── 自定义协议

Layer 2: 传输层 (Transport)
  ├── 连接池
  ├── 多主机管理
  └── 并发执行

Layer 1: Libvirt 适配层
  └── virt crate 封装
```

## 重构步骤

### Phase 1: 传输层重构 ✅

#### Step 1.1: 创建配置模块
```rust
// transport/config.rs
pub struct TransportConfig {
    pub max_connections_per_host: usize,
    pub connect_timeout: Duration,
    pub heartbeat_interval: Duration,
}
```

#### Step 1.2: 创建连接管理
```rust
// transport/connection.rs
pub struct HostConnection {
    host_info: HostInfo,
    connection: Arc<Mutex<Connect>>,
    state: ConnectionState,
}
```

#### Step 1.3: 创建连接池
```rust
// transport/pool.rs
pub struct ConnectionPool {
    hosts: HashMap<String, Vec<HostConnection>>,
    config: PoolConfig,
}
```

#### Step 1.4: 创建传输管理器
```rust
// transport/manager.rs
pub struct TransportManager {
    pool: Arc<RwLock<ConnectionPool>>,
    executor: TaskExecutor,
}
```

### Phase 2: 协议层抽象

#### Step 2.1: 定义协议 Trait
```rust
// protocol/mod.rs
#[async_trait]
pub trait Protocol: Send + Sync {
    async fn connect(&mut self, domain: &Domain) -> Result<()>;
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn receive(&mut self) -> Result<Vec<u8>>;
    fn protocol_type(&self) -> ProtocolType;
}
```

#### Step 2.2: 重构 QMP 为协议实现
```rust
// protocol/qmp.rs
pub struct QmpProtocol {
    client: QmpClient,
}

impl Protocol for QmpProtocol {
    // 实现协议接口
}
```

#### Step 2.3: 重构 QGA 为协议实现
```rust
// protocol/qga.rs
pub struct QgaProtocol {
    client: QgaClient,
}

impl Protocol for QgaProtocol {
    // 实现协议接口
}
```

#### Step 2.4: 协议注册机制
```rust
// protocol/registry.rs
pub struct ProtocolRegistry {
    protocols: HashMap<String, Box<dyn Protocol>>,
}
```

### Phase 3: 应用层实现

#### Step 3.1: CLI 基础框架
```rust
// application/cli/main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Keyboard(KeyboardCmd),
    Mouse(MouseCmd),
    Command(ExecCmd),
}
```

#### Step 3.2: HTTP API 服务
```rust
// application/http-api/main.rs
use axum::{Router, routing::post};

async fn keyboard_send() -> impl IntoResponse { }
async fn mouse_click() -> impl IntoResponse { }

let app = Router::new()
    .route("/api/v1/keyboard/send", post(keyboard_send))
    .route("/api/v1/mouse/click", post(mouse_click));
```

### Phase 4: Guest 确认模块独立化

#### Step 4.1: 创建独立项目
```
guest-verifier/
├── Cargo.toml (workspace)
├── verifier-core/      # 核心库
├── verifier-agent/     # Agent 实现
└── verifier-web/       # Web 实现
```

#### Step 4.2: 定义验证器接口
```rust
// verifier-core/src/verifier.rs
#[async_trait]
pub trait Verifier: Send + Sync {
    async fn verify(&self, event: Event) -> Result<VerifyResult>;
    fn verifier_type(&self) -> VerifierType;
}
```

#### Step 4.3: 定义通信接口
```rust
// verifier-core/src/transport.rs
#[async_trait]
pub trait VerifierTransport: Send + Sync {
    async fn connect(&mut self, endpoint: &str) -> Result<()>;
    async fn send_result(&mut self, result: &VerifyResult) -> Result<()>;
    async fn receive_event(&mut self) -> Result<Event>;
}
```

## 迁移策略

### 渐进式迁移
1. 保留现有代码不变
2. 创建新的分层结构
3. 逐步迁移功能到新结构
4. 添加兼容层
5. 最终删除旧代码

### 兼容性保证
```rust
// 提供兼容包装器
pub mod compat {
    pub use crate::libvirt::LibvirtManager;
    // ... 其他兼容性导出
}
```

## 新目录结构

```
ocloudview-atp/
├── Cargo.toml (workspace root)
├── atp-core/                      # 核心框架 workspace
│   ├── Cargo.toml
│   ├── transport/                # 传输层 crate
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── protocol/                 # 协议层 crate
│   │   ├── Cargo.toml
│   │   └── src/
│   └── executor/                 # 执行器 crate
│       ├── Cargo.toml
│       └── src/
├── atp-application/              # 应用层 workspace
│   ├── Cargo.toml
│   ├── cli/                     # CLI crate
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── http-api/                # HTTP API crate
│   │   ├── Cargo.toml
│   │   └── src/
│   └── scenarios/               # 场景库 crate
│       ├── Cargo.toml
│       └── src/
├── guest-verifier/               # Guest 确认模块 workspace
│   ├── Cargo.toml
│   ├── verifier-core/           # 核心库
│   ├── verifier-agent/          # Agent 实现
│   └── verifier-web/            # Web 实现
├── test-controller/              # 旧代码（逐步迁移）
│   └── src/
│       ├── qmp/ (→ protocol/qmp)
│       ├── qga/ (→ protocol/qga)
│       ├── libvirt/ (→ transport)
│       └── ...
└── docs/
    ├── LAYERED_ARCHITECTURE.md   # 分层架构
    ├── REFACTORING_PLAN.md       # 重构计划（本文档）
    ├── API.md                    # API 文档
    └── MIGRATION_GUIDE.md        # 迁移指南
```

## 依赖关系

```
应用层 (cli, http-api)
    ↓ 依赖
协议层 (protocol)
    ↓ 依赖
传输层 (transport)
    ↓ 依赖
Libvirt (virt crate)
```

## 时间规划

### Week 1: 传输层
- Day 1-2: config, connection
- Day 3-4: pool, manager
- Day 5: 测试和文档

### Week 2: 协议层
- Day 1-2: Protocol trait 和 registry
- Day 3: QMP 协议适配
- Day 4: QGA 协议适配
- Day 5: 测试和文档

### Week 3: 应用层基础
- Day 1-3: CLI 实现
- Day 4-5: HTTP API 基础

### Week 4: Guest 确认模块
- Day 1-2: 核心接口设计
- Day 3-4: Agent 实现
- Day 5: 集成测试

## 测试策略

### 单元测试
- 每个模块独立测试
- Mock Libvirt 连接
- 协议解析测试

### 集成测试
- 端到端测试
- 多主机场景
- 并发测试

### 性能测试
- 连接池性能
- 并发能力
- 延迟测试

## 文档计划

### 需要创建的文档
1. [x] LAYERED_ARCHITECTURE.md - 分层架构
2. [x] REFACTORING_PLAN.md - 重构计划
3. [ ] API.md - API 参考
4. [ ] MIGRATION_GUIDE.md - 迁移指南
5. [ ] PROTOCOL_SPEC.md - 协议规范
6. [ ] VERIFIER_GUIDE.md - 验证器开发指南

## 风险与应对

### 风险 1: 现有代码迁移成本高
**应对**: 渐进式迁移，保持兼容层

### 风险 2: 性能可能下降
**应对**: 充分的性能测试，优化热点路径

### 风险 3: API 不稳定
**应对**: 版本管理，向后兼容

### 风险 4: 协议抽象过度
**应对**: 保持简单，根据实际需求调整

## 下一步行动

1. ✅ 创建分层架构文档
2. ✅ 创建重构计划文档
3. 🔄 实现传输层核心组件
4. ⏳ 实现协议层抽象
5. ⏳ 创建基础 CLI
6. ⏳ 重构 Guest 确认模块

## 参考资源

- [Rust 异步编程](https://rust-lang.github.io/async-book/)
- [Libvirt API 文档](https://libvirt.org/html/)
- [QMP 协议规范](https://qemu.readthedocs.io/en/latest/interop/qmp-intro.html)
- [Actor 模型](https://en.wikipedia.org/wiki/Actor_model)

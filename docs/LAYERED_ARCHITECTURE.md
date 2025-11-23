# OCloudView ATP - 分层架构设计

## 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                     应用层 (Application)                      │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  CLI 接口    │  │  HTTP API    │  │  测试场景库       │   │
│  └─────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                     协议层 (Protocol)                         │
│  ┌──────┐  ┌──────┐  ┌────────────┐  ┌──────────────┐     │
│  │ QMP  │  │ QGA  │  │ VirtioSerial│  │ SPICE(预留) │     │
│  └──────┘  └──────┘  └────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                   传输层 (Transport)                          │
│  ┌──────────────┐  ┌─────────────┐  ┌────────────────┐    │
│  │ Libvirt 连接 │  │  连接池管理  │  │  多主机支持     │    │
│  └──────────────┘  └─────────────┘  └────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              ↓
                    ┌──────────────────┐
                    │  Hypervisor       │
                    │  (QEMU/KVM)       │
                    └──────────────────┘
                              ↓
                    ┌──────────────────┐
                    │  Guest OS         │
                    └──────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│            Guest 确认模块 (独立，C/S 架构)                     │
│  ┌─────────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  底层通道层      │  │  验证器实现   │  │  API 接口    │  │
│  │ (WebSocket/TCP) │  │ (键盘/鼠标等) │  │  (与主框架)  │  │
│  └─────────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              ↑
                    ┌──────────────────┐
                    │  Web 管理平台     │
                    │  (纯前端)         │
                    └──────────────────┘
```

## 1. 传输层 (Transport Layer)

### 职责
- 管理与 Libvirt 的长连接
- 支持多主机节点
- 连接池管理
- 并发执行支持

### 核心组件

#### 1.1 HostConnection
```rust
pub struct HostConnection {
    host: String,
    uri: String,
    conn: Arc<Mutex<Connect>>,
    state: ConnectionState,
}
```

#### 1.2 ConnectionPool
```rust
pub struct ConnectionPool {
    hosts: HashMap<String, Vec<HostConnection>>,
    config: PoolConfig,
}
```

#### 1.3 TransportManager
```rust
pub struct TransportManager {
    pool: ConnectionPool,
    executor: TaskExecutor,
}
```

### 特性
- ✅ 自动重连
- ✅ 心跳检测
- ✅ 负载均衡
- ✅ 故障转移

## 2. 协议层 (Protocol Layer)

### 职责
- 提供统一的协议抽象接口
- 支持 QMP、QGA
- 支持自定义 virtio-serial 协议
- 预留 SPICE 协议接口

### 核心接口

#### 2.1 Protocol Trait
```rust
#[async_trait]
pub trait Protocol: Send + Sync {
    async fn connect(&mut self, domain: &Domain) -> Result<()>;
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn receive(&mut self) -> Result<Vec<u8>>;
    async fn disconnect(&mut self) -> Result<()>;
    fn protocol_type(&self) -> ProtocolType;
}
```

#### 2.2 ProtocolType
```rust
pub enum ProtocolType {
    QMP,
    QGA,
    VirtioSerial(String),  // 自定义协议名称
    Spice,                  // 预留
}
```

#### 2.3 具体实现
- `QmpProtocol`: QMP 协议实现
- `QgaProtocol`: QGA 协议实现
- `CustomVirtioProtocol`: 自定义 virtio-serial 协议
- `SpiceProtocol`: SPICE 协议（预留）

### 协议注册机制
```rust
pub struct ProtocolRegistry {
    protocols: HashMap<String, Box<dyn Protocol>>,
}

impl ProtocolRegistry {
    pub fn register(&mut self, name: &str, protocol: Box<dyn Protocol>);
    pub fn get(&self, name: &str) -> Option<&dyn Protocol>;
}
```

## 3. 应用层 (Application Layer)

### 3.1 CLI 接口

#### 基础命令
```bash
# 键盘操作
atp keyboard send --host host1 --vm vm1 --key 'a'
atp keyboard send --host host1 --vm vm1 --text "Hello World"

# 鼠标操作
atp mouse click --host host1 --vm vm1 --button left --x 100 --y 200
atp mouse move --host host1 --vm vm1 --x 500 --y 300

# 命令执行
atp command exec --host host1 --vm vm1 --cmd "ls -la"

# 脚本执行
atp script run --file test.yaml
```

#### 高级功能（后续）
```bash
# 并发执行
atp keyboard send --concurrent --hosts host1,host2,host3 --key 'a'

# 循环执行
atp keyboard send --loop 100 --interval 1s --key 'a'

# 组合操作
atp combo run --file combo.yaml
```

### 3.2 HTTP API 接口

#### RESTful API
```
POST   /api/v1/hosts                    # 添加主机
GET    /api/v1/hosts                    # 列出主机
GET    /api/v1/hosts/:id/vms            # 列出虚拟机

POST   /api/v1/keyboard/send            # 发送按键
POST   /api/v1/mouse/click              # 鼠标点击
POST   /api/v1/mouse/move               # 鼠标移动
POST   /api/v1/command/exec             # 执行命令

POST   /api/v1/scripts/run              # 运行脚本
GET    /api/v1/scripts/:id/status       # 获取脚本状态

GET    /api/v1/verify/status            # 获取验证状态
```

#### WebSocket API
```
ws://host:port/api/v1/ws/events         # 实时事件流
ws://host:port/api/v1/ws/logs           # 实时日志流
```

### 3.3 测试场景库

#### 场景定义（YAML）
```yaml
# keyboard_test.yaml
name: "键盘测试场景"
description: "测试键盘输入功能"
steps:
  - action: keyboard.send_key
    key: "a"
    verify: true

  - action: keyboard.send_text
    text: "Hello World"
    verify: true

  - action: wait
    duration: 1s
```

#### 场景执行器
```rust
pub struct ScenarioExecutor {
    transport: Arc<TransportManager>,
    protocols: Arc<ProtocolRegistry>,
}

impl ScenarioExecutor {
    pub async fn execute(&self, scenario: &Scenario) -> Result<TestReport>;
}
```

## 4. Guest 确认模块 (独立项目)

### 架构设计

```
┌─────────────────────────────────────────┐
│          Verifier API (接口层)           │
│     (与主框架通信 - WebSocket/gRPC)      │
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
│  │WebSocket │  │  TCP/Unix Socket │    │
│  └──────────┘  └──────────────────┘    │
└─────────────────────────────────────────┘
```

### 核心组件

#### 4.1 Verifier Trait
```rust
#[async_trait]
pub trait Verifier: Send + Sync {
    async fn verify(&self, event: Event) -> Result<VerifyResult>;
    fn verifier_type(&self) -> VerifierType;
}

pub enum VerifierType {
    Keyboard,
    Mouse,
    Command,
    Custom(String),
}
```

#### 4.2 通道层抽象
```rust
#[async_trait]
pub trait VerifierTransport: Send + Sync {
    async fn connect(&mut self, endpoint: &str) -> Result<()>;
    async fn send_result(&mut self, result: &VerifyResult) -> Result<()>;
    async fn receive_event(&mut self) -> Result<Event>;
}
```

#### 4.3 实现
- `WebSocketTransport`: WebSocket 通道
- `TcpTransport`: TCP 通道
- `KeyboardVerifier`: 键盘验证器
- `MouseVerifier`: 鼠标验证器
- `CommandVerifier`: 命令验证器

### API 定义（JSON）

#### 事件格式
```json
{
  "type": "keyboard",
  "action": "keydown",
  "key": "a",
  "timestamp": 1234567890,
  "metadata": {}
}
```

#### 验证结果格式
```json
{
  "event_id": "uuid",
  "verified": true,
  "timestamp": 1234567890,
  "latency_ms": 15,
  "details": {}
}
```

## 5. Web 管理平台

### 技术栈
- 前端框架: React / Vue 3
- UI 组件: Ant Design / Element Plus
- 状态管理: Redux / Pinia
- 通信: Axios + WebSocket

### 功能模块

#### 5.1 主机管理
- 添加/删除主机
- 主机状态监控
- 虚拟机列表

#### 5.2 测试控制台
- 实时发送键盘/鼠标事件
- 命令执行
- 脚本编辑器

#### 5.3 场景管理
- 场景列表
- 场景编辑（可视化/YAML）
- 场景执行

#### 5.4 监控面板
- 实时日志
- 性能指标
- 验证结果展示

## 6. SPICE 协议支持（预留）

### 功能规划
- 输入通道测试
- 重定向通道测试
- 分辨率变化测试
- 视频流接收
- 转流媒体（RTSP/HLS）
- 编码能力验证

### 接口定义
```rust
pub trait SpiceChannel: Send + Sync {
    async fn connect(&mut self, params: &SpiceParams) -> Result<()>;
    async fn test_input(&mut self) -> Result<TestResult>;
    async fn test_display(&mut self) -> Result<TestResult>;
    async fn capture_video(&mut self) -> Result<VideoStream>;
}
```

## 7. 设计原则

### 7.1 易扩展
- 插件化架构
- Protocol Trait 允许添加新协议
- Verifier Trait 允许添加新验证器
- 场景系统支持自定义步骤

### 7.2 分层
- 传输层：底层连接管理
- 协议层：协议抽象
- 应用层：业务逻辑
- 清晰的层级边界

### 7.3 解耦
- Guest 确认模块独立
- Web 前端独立
- 通过接口通信
- 最小依赖

### 7.4 热插拔
- 动态协议注册
- 动态验证器注册
- 运行时配置重载

### 7.5 自动化
- 场景驱动测试
- 批量执行
- 结果自动收集
- 报告自动生成

## 8. 项目结构

```
ocloudview-atp/
├── atp-core/                       # 核心框架 (Rust Workspace)
│   ├── Cargo.toml                 # Workspace 配置
│   ├── transport/                 # 传输层
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── protocol/                  # 协议层
│   │   ├── Cargo.toml
│   │   └── src/
│   └── executor/                  # 执行器
│       ├── Cargo.toml
│       └── src/
├── atp-application/               # 应用层 (Rust Workspace)
│   ├── Cargo.toml
│   ├── cli/                      # CLI 应用
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── http-api/                 # HTTP API 服务
│   │   ├── Cargo.toml
│   │   └── src/
│   └── scenarios/                # 场景库
│       ├── Cargo.toml
│       └── src/
├── guest-verifier/                # Guest 确认模块 (独立项目)
│   ├── Cargo.toml                # Workspace 配置
│   ├── verifier-core/            # 核心库
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── verifier-agent/           # Agent 实现
│   │   ├── Cargo.toml
│   │   └── src/
│   └── verifier-web/             # Web 实现
│       ├── index.html
│       └── agent.js
├── web-console/                   # Web 管理平台
│   ├── package.json
│   ├── src/
│   └── public/
├── docs/                          # 文档
│   ├── ARCHITECTURE.md           # 架构文档
│   ├── API.md                    # API 文档
│   └── DEVELOPMENT.md            # 开发指南
└── examples/                      # 示例
    ├── scenarios/                # 测试场景
    └── scripts/                  # 脚本
```

## 9. 开发路线图

### Phase 1: 基础实现（当前）
- [x] 传输层基础实现
- [ ] QMP/QGA 协议集成
- [ ] CLI 基础命令
- [ ] Guest 确认模块基础版

### Phase 2: 应用层
- [ ] HTTP API 服务
- [ ] 场景执行引擎
- [ ] Web 控制台基础版

### Phase 3: 高级功能
- [ ] 并发执行
- [ ] 循环/组合操作
- [ ] 性能优化
- [ ] 监控与报告

### Phase 4: 扩展功能
- [ ] SPICE 协议支持
- [ ] 视频流处理
- [ ] 更多验证器
- [ ] 插件系统

## 10. 性能指标

### 目标
- 单主机并发: 50+ VMs
- 操作延迟: < 10ms
- 验证延迟: < 20ms
- API 响应: < 100ms
- WebSocket 延迟: < 50ms

### 监控指标
- 连接数
- 请求/秒
- 错误率
- 平均延迟
- P99 延迟

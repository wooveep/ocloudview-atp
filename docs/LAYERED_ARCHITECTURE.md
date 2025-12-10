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
│               VDI 平台测试层 (VDI Platform)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ VDI Client   │  │ 场景编排器    │  │ 集成适配器   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└───────────────────────┬─────────────────────────────────────┘
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
└───────────────────────┬─────────────────────────────────────┘
                        ↓
         ┌──────────────────────────────────┐
         │     OCloudView VDI 平台          │
         │  (云桌面管理系统 API 接口)        │
         └──────────────┬───────────────────┘
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
- 管理与 Libvirt 的长连接（QMP/QGA/VirtioSerial）
- 管理 Spice 多通道连接
- 支持多主机节点
- 连接池管理
- 并发执行支持

### 连接模式

#### 1.1 Libvirt 复用模式
用于 **QMP、QGA、VirtioSerial** 协议：
- **长连接复用**：所有协议共享同一个 libvirt 连接
- **连接管理**：由 libvirt 库负责连接的建立、维护和心跳检测
- **通信方式**：通过 libvirt API 与虚拟机交互

支持的 URI 格式：
```
qemu+tcp://192.168.1.10/system        # TCP (无加密)
qemu+tls://192.168.1.10/system        # TLS (加密)
qemu+ssh://root@192.168.1.10:22/system  # SSH (默认)
```

#### 1.2 Spice 独立多通道模式
用于 **Spice** 协议：
- **多通道架构**：每个 VM 有多个独立的 Spice 通道
- **独立连接**：每个通道是独立的 TCP/WebSocket 连接
- **不依赖 libvirt**：直接连接到 Spice 服务器
- **动态通道管理**：通道可以动态创建和销毁

Spice 通道类型：Main、Display、Inputs、Cursor、Playback、Record、UsbRedir 等

详细设计见 [连接模式设计文档](./CONNECTION_MODES.md)

### 核心组件

#### 1.3 HostConnection (Libvirt 模式)
```rust
pub struct HostConnection {
    host_info: HostInfo,
    connection: Arc<Mutex<Option<Connect>>>,  // Libvirt 连接
    state: Arc<Mutex<ConnectionState>>,
    config: Arc<TransportConfig>,
}
```

#### 1.4 SpiceVmConnection (Spice 模式)
```rust
pub struct SpiceVmConnection {
    vm_id: String,
    server_addr: String,
    port: u16,
    channels: Arc<RwLock<HashMap<SpiceChannelType, SpiceChannel>>>,  // 多通道
    state: Arc<Mutex<ConnectionState>>,
}
```

#### 1.5 ConnectionPool
```rust
pub struct ConnectionPool {
    hosts: HashMap<String, Vec<HostConnection>>,
    config: PoolConfig,
}
```

#### 1.6 SpiceConnectionManager
```rust
pub struct SpiceConnectionManager {
    vm_connections: Arc<RwLock<HashMap<String, SpiceVmConnection>>>,
}
```

#### 1.7 TransportManager
```rust
pub struct TransportManager {
    libvirt_pool: ConnectionPool,        // Libvirt 连接池
    spice_manager: SpiceConnectionManager,  // Spice 连接管理器
    config: Arc<TransportConfig>,
}
```

### 特性
- ✅ 自动重连
- ✅ 心跳检测
- ✅ 负载均衡
- ✅ 故障转移

## 2. VDI 平台测试层 (VDI Platform Testing Layer)

### 职责
- 通过 API 接口测试 OCloudView 云桌面平台功能
- 与虚拟化层测试进行深度集成
- 实现端到端的自动化测试场景
- 支持桌面池、虚拟机、模板等资源的自动化管理

### 核心组件

#### 2.1 VDI Client
```rust
pub struct VdiClient {
    base_url: String,
    http_client: reqwest::Client,
    access_token: Arc<RwLock<Option<String>>>,
    config: VdiConfig,
}

impl VdiClient {
    pub async fn login(&mut self, username: &str, password: &str) -> Result<()>;
    pub fn domain(&self) -> DomainApi;       // 虚拟机 API
    pub fn desk_pool(&self) -> DeskPoolApi;  // 桌面池 API
    pub fn host(&self) -> HostApi;           // 主机 API
}
```

#### 2.2 场景编排器
```rust
pub struct ScenarioOrchestrator {
    vdi_client: Arc<VdiClient>,
    transport_manager: Arc<TransportManager>,
    protocol_registry: Arc<ProtocolRegistry>,
}

pub enum TestStep {
    VdiAction(VdiAction),              // VDI 平台操作
    VirtualizationAction(VirtualizationAction),  // 虚拟化层操作
    Wait { duration: Duration },       // 等待
    Verify(VerifyCondition),           // 验证
}
```

#### 2.3 集成适配器
```rust
pub struct VdiVirtualizationAdapter {
    vdi_client: Arc<VdiClient>,
    transport_manager: Arc<TransportManager>,
}

impl VdiVirtualizationAdapter {
    /// 通过 VDI API 创建虚拟机并建立传输层连接
    pub async fn create_and_connect(
        &self,
        req: CreateDomainRequest,
    ) -> Result<(Domain, Arc<HostConnection>)>;
}
```

### VDI 平台 API 功能

#### 虚拟机管理 (Domain)
- 生命周期：创建、启动、关闭、重启、删除
- 状态管理：暂停、恢复、睡眠、唤醒、冻结
- 用户管理：绑定用户、解绑用户
- 资源管理：CPU/内存调整、磁盘管理

#### 桌面池管理 (Desk Pool)
- 桌面池 CRUD：创建、查询、更新、删除
- 状态管理：启用、禁用、激活
- 模板管理：切换模板
- 虚拟机列表：获取桌面池中的虚拟机

#### 主机管理 (Host)
- 主机信息：查询主机信息、硬件信息
- 状态监控：主机状态、运行时间
- 性能监控：性能数据、资源使用情况

### 端到端测试场景示例

```yaml
# 创建桌面池并进行虚拟化层测试
name: "桌面池创建与键盘输入测试"
steps:
  # 1. 通过 VDI API 创建桌面池
  - vdi_action:
      type: create_desk_pool
      name: "测试桌面池"
      count: 2

  # 2. 启用桌面池
  - vdi_action:
      type: enable_desk_pool

  # 3. 启动虚拟机
  - vdi_action:
      type: start_domain

  # 4. 建立虚拟化层连接
  - virtualization_action:
      type: connect

  # 5. 发送键盘输入（虚拟化层）
  - virtualization_action:
      type: send_keyboard
      text: "Hello from ATP!"
```

详细设计见 [VDI 平台测试文档](./VDI_PLATFORM_TESTING.md)

## 3. 协议层 (Protocol Layer)

### 职责
- 提供统一的协议抽象接口
- 支持 QMP、QGA
- 支持自定义 virtio-serial 协议
- 预留 SPICE 协议接口

### 核心接口

#### 3.1 Protocol Trait
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

#### 3.2 ProtocolType
```rust
pub enum ProtocolType {
    QMP,
    QGA,
    VirtioSerial(String),  // 自定义协议名称
    Spice,                  // 预留
}
```

#### 3.3 具体实现
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

## 4. 应用层 (Application Layer)

### 6.1 CLI 接口

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

### 6.2 HTTP API 接口

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

### 6.3 测试场景库

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

## 5. Guest 确认模块 (独立项目)

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

## 6. Web 管理平台

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

## 7. SPICE 协议支持（预留）

### 架构特点
Spice 协议与其他协议的最大区别在于：
- **多通道架构**：每个 VM 有多个独立的 Spice 通道（Main、Display、Inputs 等）
- **独立连接**：每个通道是独立的 TCP 连接，不复用 libvirt 连接
- **动态管理**：通道可以在运行时动态创建和销毁

### 通道类型

```rust
pub enum SpiceChannelType {
    Main,       // 主通道 (必需)
    Display,    // 显示通道 (视频输出)
    Inputs,     // 输入通道 (键盘/鼠标)
    Cursor,     // 光标通道
    Playback,   // Playback 通道 (音频输出)
    Record,     // Record 通道 (音频输入)
    UsbRedir,   // USB 重定向通道
    SmartCard,  // 智能卡通道
    WebDav,     // WebDAV 通道 (文件共享)
    Port,       // 端口通道 (串口/并口重定向)
}
```

### 功能规划
- 输入通道测试（键盘/鼠标）
- 重定向通道测试（USB/串口）
- 分辨率变化测试
- 视频流接收
- 转流媒体（RTSP/HLS）
- 编码能力验证

### 接口定义
```rust
pub struct SpiceVmConnection {
    vm_id: String,
    server_addr: String,
    port: u16,
    channels: Arc<RwLock<HashMap<SpiceChannelType, SpiceChannel>>>,
}

impl SpiceVmConnection {
    async fn connect_main_channel(&mut self) -> Result<()>;
    async fn open_channel(&mut self, channel_type: SpiceChannelType) -> Result<()>;
    async fn close_channel(&mut self, channel_type: &SpiceChannelType) -> Result<()>;
    async fn get_channel(&self, channel_type: &SpiceChannelType) -> Option<SpiceChannel>;
}

#[async_trait]
pub trait Protocol for SpiceProtocol {
    async fn connect(&mut self, domain: &Domain) -> Result<()>;
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn receive(&mut self) -> Result<Vec<u8>>;
    async fn disconnect(&mut self) -> Result<()>;
}
```

详细设计见 [连接模式设计文档](./CONNECTION_MODES.md) 第 2 节。

## 8. 设计原则

### 8.1 易扩展
- 插件化架构
- Protocol Trait 允许添加新协议
- Verifier Trait 允许添加新验证器
- 场景系统支持自定义步骤

### 8.2 分层
- 传输层：底层连接管理
- 协议层：协议抽象
- 应用层：业务逻辑
- 清晰的层级边界

### 8.3 解耦
- Guest 确认模块独立
- Web 前端独立
- 通过接口通信
- 最小依赖

### 8.4 热插拔
- 动态协议注册
- 动态验证器注册
- 运行时配置重载

### 8.5 自动化
- 场景驱动测试
- 批量执行
- 结果自动收集
- 报告自动生成

## 9. 项目结构

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
│   ├── vdiplatform/               # VDI 平台测试模块（新增）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs          # VDI 平台客户端
│   │       ├── api/               # API 模块
│   │       │   ├── domain.rs      # 虚拟机 API
│   │       │   ├── desk_pool.rs   # 桌面池 API
│   │       │   ├── host.rs        # 主机 API
│   │       │   └── model.rs       # 模板 API
│   │       └── models/            # 数据模型
│   ├── orchestrator/              # 场景编排器（新增）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── scenario.rs        # 场景定义
│   │       ├── executor.rs        # 场景执行器
│   │       └── adapter.rs         # 集成适配器
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
│   ├── LAYERED_ARCHITECTURE.md    # 分层架构文档
│   ├── CONNECTION_MODES.md        # 连接模式设计文档
│   ├── VDI_PLATFORM_TESTING.md    # VDI 平台测试文档
│   ├── API.md                     # API 文档
│   └── DEVELOPMENT.md             # 开发指南
└── examples/                      # 示例
    ├── scenarios/                 # 虚拟化层测试场景
    ├── vdi-scenarios/             # VDI 平台测试场景（新增）
    │   ├── basic/                 # 基础场景
    │   ├── integration/           # 集成场景
    │   └── stress/                # 压力测试场景
    └── scripts/                   # 脚本
```

## 10. 开发路线图

### Phase 1: 基础实现（当前）
- [x] 传输层基础实现
- [x] 连接模式设计（Libvirt 复用模式 + Spice 多通道模式）
- [ ] QMP/QGA 协议集成
- [ ] CLI 基础命令
- [ ] Guest 确认模块基础版

### Phase 2: VDI 平台测试层
- [ ] VDI 客户端核心实现
- [ ] 基础 API 封装（Domain、DeskPool、Host）
- [ ] 场景编排器实现
- [ ] VDI 与虚拟化层集成适配器
- [ ] 基础测试场景

### Phase 3: 应用层
- [ ] HTTP API 服务
- [ ] 场景执行引擎
- [ ] Web 控制台基础版
- [ ] 测试报告生成

### Phase 4: 高级功能
- [ ] 并发测试支持
- [ ] 压力测试场景
- [ ] 循环/组合操作
- [ ] 性能优化
- [ ] 监控与报告

### Phase 5: 扩展功能
- [ ] SPICE 协议支持
- [ ] 视频流处理
- [ ] 更多验证器
- [ ] 插件系统
- [ ] 完整的端到端测试场景库

## 11. 性能指标

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

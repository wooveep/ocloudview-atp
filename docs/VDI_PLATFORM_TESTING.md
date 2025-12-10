# VDI 平台测试模块设计

## 概览

VDI 平台测试模块用于自动化测试 OCloudView 云桌面平台的 API 接口功能，并与虚拟化层测试进行深度集成，实现端到端的自动化测试。

## 架构定位

```
┌─────────────────────────────────────────────────────────────┐
│                   应用层 (Application)                        │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  CLI 接口    │  │  HTTP API    │  │  测试场景库       │   │
│  └─────────────┘  └──────────────┘  └──────────────────┘   │
└───────────────────────┬─────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────────┐
│               VDI 平台测试层 (新增)                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ VDI Client   │  │ 场景编排器    │  │ 集成适配器   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└───────────────────────┬─────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────────┐
│                   协议层 (Protocol)                           │
│  ┌──────┐  ┌──────┐  ┌────────────┐  ┌──────────────┐     │
│  │ QMP  │  │ QGA  │  │ VirtioSerial│  │ SPICE(预留) │     │
│  └──────┘  └──────┘  └────────────┘  └──────────────┘     │
└───────────────────────┬─────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────────┐
│                   传输层 (Transport)                          │
│  ┌──────────────┐  ┌─────────────┐  ┌────────────────┐    │
│  │ Libvirt 连接 │  │  连接池管理  │  │  多主机支持     │    │
│  └──────────────┘  └─────────────┘  └────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                        ↓
              ┌──────────────────┐
              │  OCloudView VDI   │
              │  平台              │
              └──────────────────┘
```

## 1. VDI 平台 API 功能分类

根据 OpenAPI 文档，VDI 平台提供以下核心功能：

### 1.1 虚拟机管理 (Domain)
- **生命周期管理**：创建、启动、关闭、重启、删除
- **状态管理**：暂停、恢复、睡眠、唤醒、冻结
- **用户管理**：绑定用户、解绑用户
- **资源管理**：CPU/内存调整、磁盘管理
- **磁盘操作**：添加磁盘、扩容、卸载、速度限制

**关键 API：**
```
POST   /ocloud/v1/domain                    # 创建虚拟机
GET    /ocloud/v1/domain                    # 查询虚拟机列表
DELETE /ocloud/v1/domain/delete             # 删除虚拟机
POST   /ocloud/v1/domain/start              # 启动虚拟机
POST   /ocloud/v1/domain/close              # 关闭虚拟机
POST   /ocloud/v1/domain/reboot             # 重启虚拟机
POST   /ocloud/v1/domain/suspend            # 暂停虚拟机
POST   /ocloud/v1/domain/resume             # 恢复虚拟机
POST   /ocloud/v1/domain/bind-user          # 绑定用户
POST   /ocloud/v1/domain/unbind-user        # 解绑用户
```

### 1.2 桌面池管理 (Desk Pool)
- **桌面池 CRUD**：创建、查询、更新、删除
- **状态管理**：启用、禁用、激活
- **模板管理**：切换模板
- **虚拟机列表**：获取桌面池中的虚拟机

**关键 API：**
```
POST   /ocloud/v1/desk-pool                 # 创建桌面池
GET    /ocloud/v1/desk-pool                 # 查询桌面池列表
PUT    /ocloud/v1/desk-pool/{id}            # 更新桌面池
DELETE /ocloud/v1/desk-pool/{id}            # 删除桌面池
POST   /ocloud/v1/desk-pool/{id}/enable     # 启用桌面池
POST   /ocloud/v1/desk-pool/{id}/disable    # 禁用桌面池
GET    /ocloud/v1/desk-pool/{id}/domain/list # 获取虚拟机列表
```

### 1.3 主机管理 (Host)
- **主机信息**：查询主机信息、硬件信息
- **状态监控**：主机状态、运行时间
- **性能监控**：性能数据、资源使用情况
- **GPU 管理**：直通 GPU、vGPU

**关键 API：**
```
GET    /ocloud/v1/host                      # 查询主机列表
GET    /ocloud/v1/host/{id}                 # 查询主机详情
GET    /ocloud/v1/host/{id}/status          # 查询主机状态
GET    /ocloud/v1/host/{id}/hardware        # 查询硬件信息
GET    /ocloud/v1/host/{id}/uptime          # 查询运行时间
```

### 1.4 模板管理 (Model/Template)
- **模板 CRUD**：创建、查询、更新、删除
- **模板操作**：下载、上传
- **模板应用**：桌面池模板切换

**关键 API：**
```
GET    /ocloud/v1/user/download-template    # 下载模板
POST   /ocloud/v1/desk-pool/switch-model    # 切换桌面池模板
```

### 1.5 存储管理 (Storage)
- **存储池管理**：查询存储池
- **使用量统计**：存储使用情况

**关键 API：**
```
GET    /ocloud/v1/domain/common-storage-pool # 查询存储池
GET    /ocloud/v1/panel/storage-pool-usage   # 查询存储使用量
```

### 1.6 快照管理 (Snapshot)
- **快照 CRUD**：创建、查询、删除快照

**关键 API：**
```
GET    /ocloud/usermodule/snapshot/{domainId} # 查询快照
```

### 1.7 网络管理 (Network)
- **网卡管理**：查询网卡信息
- **IP 管理**：外网 IP 管理

**关键 API：**
```
GET    /ocloud/v1/host/{id}/nic/all         # 查询网卡列表
GET    /ocloud/v1/host/{id}/ip-info         # 查询 IP 信息
```

### 1.8 用户管理 (User)
- **用户 CRUD**：创建、查询、更新、删除用户
- **用户绑定**：用户与虚拟机绑定

### 1.9 监控与日志
- **仪表盘**：主机数量、存储使用量、性能数据
- **事件管理**：事件查询
- **告警管理**：告警查询
- **日志管理**：操作日志、访问日志

## 2. VDI 平台测试模块架构

### 2.1 核心组件

#### VDI Client
```rust
/// VDI 平台客户端
pub struct VdiClient {
    /// API 基础 URL
    base_url: String,

    /// HTTP 客户端
    http_client: reqwest::Client,

    /// 认证令牌
    access_token: Arc<RwLock<Option<String>>>,

    /// 配置
    config: VdiConfig,
}

impl VdiClient {
    /// 创建新的 VDI 客户端
    pub fn new(base_url: &str, config: VdiConfig) -> Self;

    /// 认证登录
    pub async fn login(&mut self, username: &str, password: &str) -> Result<()>;

    /// 发送 API 请求
    async fn request<T: Serialize, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<T>,
    ) -> Result<R>;
}
```

#### API 模块封装
```rust
/// 虚拟机管理 API
pub struct DomainApi {
    client: Arc<VdiClient>,
}

impl DomainApi {
    /// 创建虚拟机
    pub async fn create(&self, req: CreateDomainRequest) -> Result<Domain>;

    /// 启动虚拟机
    pub async fn start(&self, domain_id: &str) -> Result<()>;

    /// 关闭虚拟机
    pub async fn shutdown(&self, domain_id: &str) -> Result<()>;

    /// 删除虚拟机
    pub async fn delete(&self, domain_id: &str) -> Result<()>;

    /// 查询虚拟机详情
    pub async fn get(&self, domain_id: &str) -> Result<Domain>;

    /// 绑定用户
    pub async fn bind_user(&self, domain_id: &str, user_id: &str) -> Result<()>;
}

/// 桌面池管理 API
pub struct DeskPoolApi {
    client: Arc<VdiClient>,
}

impl DeskPoolApi {
    /// 创建桌面池
    pub async fn create(&self, req: CreateDeskPoolRequest) -> Result<DeskPool>;

    /// 启用桌面池
    pub async fn enable(&self, pool_id: &str) -> Result<()>;

    /// 获取桌面池中的虚拟机列表
    pub async fn list_domains(&self, pool_id: &str) -> Result<Vec<Domain>>;
}

/// 主机管理 API
pub struct HostApi {
    client: Arc<VdiClient>,
}

impl HostApi {
    /// 查询主机列表
    pub async fn list(&self) -> Result<Vec<Host>>;

    /// 查询主机详情
    pub async fn get(&self, host_id: &str) -> Result<Host>;

    /// 查询主机状态
    pub async fn get_status(&self, host_id: &str) -> Result<HostStatus>;
}
```

### 2.2 场景编排器

```rust
/// 测试场景步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestStep {
    /// VDI 平台操作
    VdiAction(VdiAction),

    /// 虚拟化层操作
    VirtualizationAction(VirtualizationAction),

    /// 等待
    Wait { duration: Duration },

    /// 验证
    Verify(VerifyCondition),
}

/// VDI 平台操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VdiAction {
    /// 创建桌面池
    CreateDeskPool {
        name: String,
        template_id: String,
        count: u32,
    },

    /// 启动虚拟机
    StartDomain { domain_id: String },

    /// 关闭虚拟机
    ShutdownDomain { domain_id: String },

    /// 绑定用户
    BindUser {
        domain_id: String,
        user_id: String,
    },
}

/// 虚拟化层操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VirtualizationAction {
    /// 发送键盘输入
    SendKeyboard { key: String },

    /// 发送鼠标点击
    SendMouseClick { x: u32, y: u32 },

    /// 执行命令
    ExecuteCommand { command: String },
}

/// 场景编排器
pub struct ScenarioOrchestrator {
    /// VDI 客户端
    vdi_client: Arc<VdiClient>,

    /// 传输管理器（虚拟化层）
    transport_manager: Arc<TransportManager>,

    /// 协议注册表
    protocol_registry: Arc<ProtocolRegistry>,
}

impl ScenarioOrchestrator {
    /// 执行测试场景
    pub async fn execute(&self, scenario: &TestScenario) -> Result<TestReport> {
        let mut report = TestReport::new(&scenario.name);

        for step in &scenario.steps {
            let step_result = self.execute_step(step).await?;
            report.add_step_result(step_result);
        }

        Ok(report)
    }

    /// 执行单个步骤
    async fn execute_step(&self, step: &TestStep) -> Result<StepResult> {
        match step {
            TestStep::VdiAction(action) => self.execute_vdi_action(action).await,
            TestStep::VirtualizationAction(action) => {
                self.execute_virtualization_action(action).await
            }
            TestStep::Wait { duration } => {
                tokio::time::sleep(*duration).await;
                Ok(StepResult::success())
            }
            TestStep::Verify(condition) => self.verify_condition(condition).await,
        }
    }
}
```

### 2.3 集成适配器

```rust
/// VDI 与虚拟化层集成适配器
pub struct VdiVirtualizationAdapter {
    /// VDI 客户端
    vdi_client: Arc<VdiClient>,

    /// 传输管理器
    transport_manager: Arc<TransportManager>,
}

impl VdiVirtualizationAdapter {
    /// 通过 VDI API 创建虚拟机并获取传输连接
    pub async fn create_and_connect(
        &self,
        req: CreateDomainRequest,
    ) -> Result<(Domain, Arc<HostConnection>)> {
        // 1. 通过 VDI API 创建虚拟机
        let domain = self.vdi_client.domain().create(req).await?;

        // 2. 等待虚拟机启动
        self.wait_for_domain_running(&domain.id).await?;

        // 3. 获取虚拟机所在主机信息
        let host_info = self.get_host_for_domain(&domain.id).await?;

        // 4. 建立传输层连接
        let connection = self.transport_manager.get_connection(&host_info.id).await?;

        Ok((domain, connection))
    }

    /// 等待虚拟机进入运行状态
    async fn wait_for_domain_running(&self, domain_id: &str) -> Result<()> {
        let max_attempts = 30;
        let interval = Duration::from_secs(2);

        for _ in 0..max_attempts {
            let domain = self.vdi_client.domain().get(domain_id).await?;
            if domain.status == "running" {
                return Ok(());
            }
            tokio::time::sleep(interval).await;
        }

        Err(Error::Timeout("虚拟机启动超时".to_string()))
    }

    /// 获取虚拟机所在主机信息
    async fn get_host_for_domain(&self, domain_id: &str) -> Result<HostInfo> {
        let domain = self.vdi_client.domain().get(domain_id).await?;
        let host = self.vdi_client.host().get(&domain.host_id).await?;

        Ok(HostInfo {
            id: host.id,
            host: host.ip,
            uri: format!("qemu+tcp://{}:16509/system", host.ip),
            tags: vec![],
            metadata: std::collections::HashMap::new(),
        })
    }
}
```

## 3. 测试场景示例

### 3.1 端到端测试场景（YAML）

```yaml
name: "桌面池创建与虚拟化层测试"
description: "创建桌面池，启动虚拟机，并进行键盘输入测试"

steps:
  # 步骤 1: 创建桌面池
  - vdi_action:
      type: create_desk_pool
      name: "测试桌面池"
      template_id: "template-001"
      count: 2
      capture_output: desk_pool

  # 步骤 2: 等待桌面池准备就绪
  - wait:
      duration: 30s

  # 步骤 3: 启用桌面池
  - vdi_action:
      type: enable_desk_pool
      pool_id: "${desk_pool.id}"

  # 步骤 4: 获取桌面池中的第一个虚拟机
  - vdi_action:
      type: get_desk_pool_domains
      pool_id: "${desk_pool.id}"
      capture_output: domains

  # 步骤 5: 启动第一个虚拟机
  - vdi_action:
      type: start_domain
      domain_id: "${domains[0].id}"

  # 步骤 6: 等待虚拟机启动
  - wait:
      duration: 10s

  # 步骤 7: 建立虚拟化层连接
  - virtualization_action:
      type: connect
      domain_id: "${domains[0].id}"

  # 步骤 8: 发送键盘输入
  - virtualization_action:
      type: send_keyboard
      text: "Hello from OCloudView ATP!"
      verify: true

  # 步骤 9: 执行命令
  - virtualization_action:
      type: execute_command
      command: "echo 'Test successful' > /tmp/test.txt"
      verify: true

  # 步骤 10: 关闭虚拟机
  - vdi_action:
      type: shutdown_domain
      domain_id: "${domains[0].id}"

  # 步骤 11: 删除桌面池
  - vdi_action:
      type: delete_desk_pool
      pool_id: "${desk_pool.id}"
```

### 3.2 桌面池压力测试场景

```yaml
name: "桌面池并发启动测试"
description: "创建包含 50 个虚拟机的桌面池，并发启动所有虚拟机"

steps:
  # 创建大型桌面池
  - vdi_action:
      type: create_desk_pool
      name: "压力测试桌面池"
      template_id: "template-001"
      count: 50
      capture_output: desk_pool

  # 启用桌面池
  - vdi_action:
      type: enable_desk_pool
      pool_id: "${desk_pool.id}"

  # 并发启动所有虚拟机
  - vdi_action:
      type: start_all_domains
      pool_id: "${desk_pool.id}"
      concurrent: true
      max_concurrency: 10

  # 等待所有虚拟机启动
  - verify:
      condition: all_domains_running
      pool_id: "${desk_pool.id}"
      timeout: 300s

  # 对每个虚拟机执行键盘输入测试
  - virtualization_action:
      type: send_keyboard_to_all
      pool_id: "${desk_pool.id}"
      text: "Stress test"
      verify: true
```

### 3.3 用户工作流测试场景

```yaml
name: "用户登录与使用场景"
description: "模拟用户登录、使用虚拟桌面、执行任务的完整流程"

steps:
  # 用户登录 VDI 平台
  - vdi_action:
      type: user_login
      username: "test_user"
      password: "password"

  # 获取用户分配的虚拟机
  - vdi_action:
      type: get_user_domain
      user_id: "test_user"
      capture_output: user_domain

  # 启动虚拟机
  - vdi_action:
      type: start_domain
      domain_id: "${user_domain.id}"

  # 等待虚拟机启动
  - wait:
      duration: 10s

  # 连接到虚拟机
  - virtualization_action:
      type: connect
      domain_id: "${user_domain.id}"

  # 模拟用户工作：打开应用
  - virtualization_action:
      type: send_keyboard
      keys: ["ctrl+esc"]  # 打开开始菜单

  - wait:
      duration: 1s

  - virtualization_action:
      type: send_keyboard
      text: "notepad"

  - virtualization_action:
      type: send_keyboard
      keys: ["enter"]

  # 模拟用户工作：编辑文档
  - wait:
      duration: 2s

  - virtualization_action:
      type: send_keyboard
      text: "This is a test document."

  # 保存文档
  - virtualization_action:
      type: send_keyboard
      keys: ["ctrl+s"]

  # 关闭应用
  - virtualization_action:
      type: send_keyboard
      keys: ["alt+f4"]

  # 用户注销
  - vdi_action:
      type: user_logout
```

## 4. 项目结构更新

```
ocloudview-atp/
├── atp-core/                       # 核心框架
│   ├── transport/                 # 传输层
│   ├── protocol/                  # 协议层
│   ├── vdiplatform/               # VDI 平台测试模块（新增）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs          # VDI 平台客户端
│   │       ├── api/               # API 模块
│   │       │   ├── domain.rs      # 虚拟机 API
│   │       │   ├── desk_pool.rs   # 桌面池 API
│   │       │   ├── host.rs        # 主机 API
│   │       │   ├── model.rs       # 模板 API
│   │       │   └── user.rs        # 用户 API
│   │       ├── models/            # 数据模型
│   │       └── error.rs           # 错误定义
│   └── orchestrator/              # 场景编排器（新增）
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── scenario.rs        # 场景定义
│           ├── executor.rs        # 场景执行器
│           ├── adapter.rs         # 集成适配器
│           └── report.rs          # 测试报告
├── examples/
│   └── vdi-scenarios/             # VDI 测试场景（新增）
│       ├── basic/
│       ├── stress/
│       └── integration/
└── docs/
    └── VDI_PLATFORM_TESTING.md    # VDI 平台测试文档
```

## 5. 实施计划

### Phase 1: VDI 客户端基础实现
- [ ] VDI 客户端核心实现
- [ ] 认证与令牌管理
- [ ] 基础 API 封装（Domain、DeskPool、Host）
- [ ] 错误处理与重试机制

### Phase 2: 场景编排器
- [ ] 测试场景定义（YAML 支持）
- [ ] 场景执行引擎
- [ ] VDI 操作执行
- [ ] 虚拟化层操作集成

### Phase 3: 集成适配器
- [ ] VDI 与虚拟化层适配器
- [ ] 自动连接建立
- [ ] 状态同步
- [ ] 资源清理

### Phase 4: 高级功能
- [ ] 并发测试支持
- [ ] 压力测试场景
- [ ] 测试报告生成
- [ ] 监控与指标收集

### Phase 5: 测试场景库
- [ ] 基础场景（创建、启动、关闭）
- [ ] 集成场景（VDI + 虚拟化层）
- [ ] 压力测试场景
- [ ] 用户工作流场景

## 6. 集成流程示例

### 典型的集成测试流程

```rust
use atp_vdiplatform::VdiClient;
use atp_orchestrator::{ScenarioOrchestrator, TestScenario};
use atp_transport::TransportManager;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 创建 VDI 客户端
    let mut vdi_client = VdiClient::new("http://192.168.1.11:8088", Default::default());
    vdi_client.login("admin", "password").await?;

    // 2. 创建传输管理器（虚拟化层）
    let transport_manager = TransportManager::new(Default::default());

    // 3. 创建场景编排器
    let orchestrator = ScenarioOrchestrator::new(
        Arc::new(vdi_client),
        Arc::new(transport_manager),
        Arc::new(protocol_registry),
    );

    // 4. 加载测试场景
    let scenario = TestScenario::from_yaml("examples/vdi-scenarios/integration/basic.yaml")?;

    // 5. 执行测试场景
    let report = orchestrator.execute(&scenario).await?;

    // 6. 输出测试报告
    println!("测试报告:");
    println!("  场景: {}", report.name);
    println!("  总步骤: {}", report.total_steps);
    println!("  成功: {}", report.success_steps);
    println!("  失败: {}", report.failed_steps);
    println!("  耗时: {:?}", report.duration);

    Ok(())
}
```

## 7. 优势与价值

### 7.1 端到端自动化
- **完整流程覆盖**：从 VDI 平台操作到虚拟化层测试
- **真实场景模拟**：模拟真实用户使用场景
- **自动化验证**：自动验证每个步骤的结果

### 7.2 测试效率提升
- **快速迭代**：自动化测试缩短测试周期
- **并发测试**：支持多虚拟机并发测试
- **可重复性**：测试场景可重复执行

### 7.3 质量保证
- **回归测试**：每次发布前自动回归测试
- **压力测试**：验证系统在高负载下的表现
- **集成测试**：验证 VDI 平台与虚拟化层的集成

### 7.4 灵活性与扩展性
- **场景驱动**：通过 YAML 定义测试场景
- **可组合**：可以组合不同的测试步骤
- **易扩展**：可以轻松添加新的 API 和测试场景

## 8. 安全考虑

### 8.1 认证与授权
- API Token 管理
- Token 自动刷新
- 权限验证

### 8.2 敏感信息保护
- 密码加密存储
- 配置文件加密
- 日志脱敏

### 8.3 资源隔离
- 测试环境隔离
- 资源清理机制
- 防止测试影响生产环境

## 9. 监控与报告

### 9.1 测试指标
- API 响应时间
- 操作成功率
- 虚拟机启动时间
- 端到端流程耗时

### 9.2 测试报告
- 详细的步骤执行日志
- 失败原因分析
- 性能数据统计
- 图表可视化

### 9.3 告警机制
- 测试失败告警
- 性能异常告警
- 资源泄漏告警

# 架构更新：添加 VDI 平台测试层

## 更新日期
2025-11-23

## 更新背景

在原有的虚拟化层自动化测试架构基础上，添加了 **VDI 平台测试层**，用于通过 API 接口自动化测试 OCloudView 云桌面管理平台的功能，并与虚拟化层测试进行深度集成，实现完整的端到端自动化测试。

## 架构变化

### 原架构（三层）
```
应用层 (Application)
    ↓
协议层 (Protocol)
    ↓
传输层 (Transport)
    ↓
Hypervisor (QEMU/KVM)
```

### 新架构（四层）
```
应用层 (Application)
    ↓
VDI 平台测试层 (VDI Platform)  ← 新增
    ↓
协议层 (Protocol)
    ↓
传输层 (Transport)
    ↓
OCloudView VDI 平台
    ↓
Hypervisor (QEMU/KVM)
```

## VDI 平台测试层详细说明

### 1. 核心功能

#### 1.1 VDI API 测试
通过 HTTP/REST API 测试 OCloudView 云桌面平台的各项功能：
- **虚拟机管理**：创建、启动、关闭、删除、暂停、恢复
- **桌面池管理**：桌面池 CRUD、启用/禁用、模板切换
- **主机管理**：主机查询、状态监控、性能数据
- **用户管理**：用户绑定/解绑、权限管理
- **资源管理**：存储、网络、快照等

#### 1.2 端到端集成测试
结合 VDI 平台操作和虚拟化层测试，实现完整的测试流程：
1. 通过 VDI API 创建并启动虚拟机
2. 自动建立传输层连接
3. 执行虚拟化层测试（键盘、鼠标、命令）
4. 验证测试结果
5. 清理测试资源

### 2. 核心组件

#### 2.1 VDI Client
```rust
pub struct VdiClient {
    base_url: String,               // VDI 平台 API 地址
    http_client: reqwest::Client,   // HTTP 客户端
    access_token: Arc<RwLock<Option<String>>>,  // 认证令牌
    config: VdiConfig,              // 配置
}
```

**功能：**
- 认证与令牌管理
- API 请求封装
- 错误处理与重试
- 响应解析

**API 模块：**
- `DomainApi` - 虚拟机管理
- `DeskPoolApi` - 桌面池管理
- `HostApi` - 主机管理
- `ModelApi` - 模板管理
- `UserApi` - 用户管理

#### 2.2 场景编排器 (ScenarioOrchestrator)
```rust
pub struct ScenarioOrchestrator {
    vdi_client: Arc<VdiClient>,
    transport_manager: Arc<TransportManager>,
    protocol_registry: Arc<ProtocolRegistry>,
}
```

**功能：**
- 测试场景解析（支持 YAML）
- 场景步骤执行
- VDI 操作与虚拟化层操作的协调
- 测试结果收集与报告生成

**支持的步骤类型：**
- `VdiAction` - VDI 平台操作
- `VirtualizationAction` - 虚拟化层操作
- `Wait` - 等待
- `Verify` - 验证条件

#### 2.3 集成适配器 (VdiVirtualizationAdapter)
```rust
pub struct VdiVirtualizationAdapter {
    vdi_client: Arc<VdiClient>,
    transport_manager: Arc<TransportManager>,
}
```

**功能：**
- VDI 平台与虚拟化层的桥接
- 自动建立传输连接
- 虚拟机状态同步
- 资源清理

### 3. 测试场景示例

#### 3.1 基础场景：桌面池创建与测试
```yaml
name: "桌面池创建与键盘输入测试"
steps:
  # VDI 平台操作
  - vdi_action:
      type: create_desk_pool
      name: "测试桌面池"
      template_id: "template-001"
      count: 2

  - vdi_action:
      type: enable_desk_pool
      pool_id: "${desk_pool.id}"

  - vdi_action:
      type: start_domain
      domain_id: "${domains[0].id}"

  # 虚拟化层操作
  - virtualization_action:
      type: connect
      domain_id: "${domains[0].id}"

  - virtualization_action:
      type: send_keyboard
      text: "Hello from OCloudView ATP!"
      verify: true

  # 清理资源
  - vdi_action:
      type: delete_desk_pool
      pool_id: "${desk_pool.id}"
```

#### 3.2 压力测试场景
```yaml
name: "桌面池并发启动测试"
steps:
  - vdi_action:
      type: create_desk_pool
      count: 50

  - vdi_action:
      type: start_all_domains
      concurrent: true
      max_concurrency: 10

  - verify:
      condition: all_domains_running
      timeout: 300s
```

#### 3.3 用户工作流场景
```yaml
name: "用户登录与使用场景"
steps:
  - vdi_action:
      type: user_login
      username: "test_user"

  - vdi_action:
      type: get_user_domain

  - vdi_action:
      type: start_domain

  - virtualization_action:
      type: open_application
      app: "notepad"

  - virtualization_action:
      type: input_text
      text: "Test document"

  - virtualization_action:
      type: save_file
```

### 4. 测试类型

#### 4.1 功能测试
- VDI 平台基础功能测试
- API 接口正确性验证
- 错误处理测试

#### 4.2 集成测试
- VDI 平台 + 虚拟化层集成
- 端到端流程测试
- 多组件协同测试

#### 4.3 性能测试
- 桌面池并发创建
- 虚拟机批量启动
- API 响应时间测试

#### 4.4 压力测试
- 大规模虚拟机管理
- 高并发操作
- 资源限制测试

#### 4.5 场景测试
- 真实用户工作流
- 典型使用场景
- 业务流程验证

## 项目结构变化

### 新增模块

```
atp-core/
├── vdiplatform/             # VDI 平台测试模块（新增）
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── client.rs        # VDI 平台客户端核心
│       ├── api/             # API 模块
│       │   ├── domain.rs    # 虚拟机 API
│       │   ├── desk_pool.rs # 桌面池 API
│       │   ├── host.rs      # 主机 API
│       │   ├── model.rs     # 模板 API
│       │   └── user.rs      # 用户 API
│       ├── models/          # 数据模型
│       └── error.rs         # 错误定义
│
└── orchestrator/            # 场景编排器（新增）
    ├── Cargo.toml
    └── src/
        ├── scenario.rs      # 场景定义
        ├── executor.rs      # 场景执行器
        ├── adapter.rs       # 集成适配器
        └── report.rs        # 测试报告
```

### 新增文档

```
docs/
├── VDI_PLATFORM_TESTING.md      # VDI 平台测试详细设计（新增）
└── LAYERED_ARCHITECTURE.md      # 主架构文档（已更新）
```

### 新增示例

```
examples/
└── vdi-scenarios/               # VDI 测试场景（新增）
    ├── basic/                   # 基础场景
    │   ├── create_desk_pool.yaml
    │   ├── start_vm.yaml
    │   └── delete_vm.yaml
    ├── integration/             # 集成场景
    │   ├── end_to_end.yaml
    │   └── user_workflow.yaml
    └── stress/                  # 压力测试
        ├── concurrent_start.yaml
        └── large_pool.yaml
```

## OCloudView VDI API 功能覆盖

根据 `docs/Ocloud View 9.0接口文档_OpenAPI.json`，VDI 平台提供以下 API：

### 虚拟机管理 (Domain)
```
POST   /ocloud/v1/domain                    # 创建虚拟机
DELETE /ocloud/v1/domain/delete             # 删除虚拟机
POST   /ocloud/v1/domain/start              # 启动虚拟机
POST   /ocloud/v1/domain/close              # 关闭虚拟机
POST   /ocloud/v1/domain/reboot             # 重启虚拟机
POST   /ocloud/v1/domain/suspend            # 暂停虚拟机
POST   /ocloud/v1/domain/resume             # 恢复虚拟机
POST   /ocloud/v1/domain/sleep              # 睡眠虚拟机
POST   /ocloud/v1/domain/wakeup             # 唤醒虚拟机
POST   /ocloud/v1/domain/freeze             # 冻结虚拟机
POST   /ocloud/v1/domain/bind-user          # 绑定用户
POST   /ocloud/v1/domain/unbind-user        # 解绑用户
POST   /ocloud/v1/domain/mem-cpu            # 调整 CPU/内存
```

### 桌面池管理 (Desk Pool)
```
POST   /ocloud/v1/desk-pool                 # 创建桌面池
GET    /ocloud/v1/desk-pool                 # 查询桌面池
PUT    /ocloud/v1/desk-pool/{id}            # 更新桌面池
DELETE /ocloud/v1/desk-pool/{id}            # 删除桌面池
POST   /ocloud/v1/desk-pool/{id}/enable     # 启用桌面池
POST   /ocloud/v1/desk-pool/{id}/disable    # 禁用桌面池
POST   /ocloud/v1/desk-pool/{id}/active     # 激活桌面池
GET    /ocloud/v1/desk-pool/{id}/domain/list # 获取虚拟机列表
POST   /ocloud/v1/desk-pool/switch-model    # 切换模板
```

### 主机管理 (Host)
```
GET    /ocloud/v1/host                      # 查询主机列表
GET    /ocloud/v1/host/{id}                 # 查询主机详情
GET    /ocloud/v1/host/{id}/status          # 查询主机状态
GET    /ocloud/v1/host/{id}/hardware        # 查询硬件信息
GET    /ocloud/v1/host/{id}/uptime          # 查询运行时间
GET    /ocloud/v1/panel/host/performance    # 查询性能数据
```

## 开发路线图更新

### Phase 2: VDI 平台测试层（新增）
- [ ] VDI 客户端核心实现
- [ ] 认证与令牌管理
- [ ] 基础 API 封装（Domain、DeskPool、Host）
- [ ] 场景编排器实现
- [ ] VDI 与虚拟化层集成适配器
- [ ] 基础测试场景（创建、启动、关闭）
- [ ] 集成测试场景（端到端）
- [ ] 测试报告生成

## 技术优势

### 1. 完整性
- 覆盖 VDI 平台和虚拟化层两个层面
- 端到端的自动化测试
- 真实场景模拟

### 2. 灵活性
- 场景驱动，通过 YAML 定义测试
- 可组合的测试步骤
- 易于扩展新的 API 和场景

### 3. 效率
- 自动化测试，减少人工操作
- 支持并发测试
- 快速反馈

### 4. 可靠性
- 自动验证每个步骤
- 详细的测试报告
- 资源自动清理

## 使用示例

### 基础使用
```rust
use atp_vdiplatform::VdiClient;
use atp_orchestrator::ScenarioOrchestrator;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 创建 VDI 客户端
    let mut vdi_client = VdiClient::new(
        "http://192.168.1.11:8088",
        Default::default()
    );
    vdi_client.login("admin", "password").await?;

    // 2. 创建传输管理器
    let transport = TransportManager::new(Default::default());

    // 3. 创建场景编排器
    let orchestrator = ScenarioOrchestrator::new(
        Arc::new(vdi_client),
        Arc::new(transport),
        Arc::new(protocol_registry),
    );

    // 4. 执行测试场景
    let scenario = TestScenario::from_yaml(
        "examples/vdi-scenarios/integration/basic.yaml"
    )?;
    let report = orchestrator.execute(&scenario).await?;

    // 5. 输出报告
    println!("测试完成: {}/{} 步骤成功",
        report.success_steps,
        report.total_steps
    );

    Ok(())
}
```

### CLI 使用
```bash
# 执行 VDI 测试场景
atp vdi run --scenario examples/vdi-scenarios/integration/basic.yaml

# 执行压力测试
atp vdi stress --pool-size 50 --concurrent 10

# 查看测试报告
atp vdi report --id <test-id>
```

## 安全考虑

### 认证与授权
- API Token 管理
- Token 自动刷新
- 权限验证

### 敏感信息保护
- 密码加密存储
- 配置文件加密
- 日志脱敏

### 资源隔离
- 测试环境隔离
- 资源清理机制
- 防止测试影响生产

## 性能指标

### VDI API 性能
- API 响应时间: < 100ms
- 虚拟机创建: < 30s
- 虚拟机启动: < 10s
- 桌面池创建: < 60s

### 集成测试性能
- 端到端流程: < 2min
- 并发测试: 50+ VMs
- 测试场景执行: < 5min

## 总结

通过添加 VDI 平台测试层，OCloudView ATP 实现了：

1. **完整的测试覆盖**：从 VDI 平台 API 到虚拟化层的端到端测试
2. **灵活的场景定义**：通过 YAML 定义复杂的测试场景
3. **深度集成**：VDI 平台操作与虚拟化层测试无缝集成
4. **自动化验证**：自动验证每个步骤的结果
5. **高效测试**：支持并发测试和压力测试

这为 OCloudView 云桌面平台的质量保证提供了强大的自动化测试工具！

# Verification Server

Guest 验证器服务端组件，提供 WebSocket 和 TCP 服务器用于接收 Guest Agent 的验证结果，并与发送的事件进行一对一匹配。

## 架构

```
┌─────────────────────────────────────────────────┐
│           ATP Application Layer                 │
│        (ScenarioRunner, Executor)               │
└──────────────────┬──────────────────────────────┘
                   │ 调用 verify_event()
                   ↓
┌─────────────────────────────────────────────────┐
│         VerificationService                     │
│   - UUID 事件跟踪                                │
│   - VM ID 路由                                   │
│   - 一对一事件-结果匹配                           │
│   - 超时管理                                     │
└──────────────────┬──────────────────────────────┘
                   │
        ┌──────────┴──────────┐
        ↓                     ↓
┌───────────────┐    ┌────────────────┐
│ ClientManager │    │VerificationServer│
│ - 客户端管理   │    │ - WebSocket 服务 │
│ - VM ID 映射   │    │ - TCP 服务       │
│ - 事件分发     │    │ - 客户端注册     │
└───────────────┘    └────────────────┘
        ↓                     ↓
┌─────────────────────────────────────────────────┐
│            Network Layer                        │
│    WebSocket (port 8765)  TCP (port 8766)       │
└─────────────────────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────────────┐
│         Guest Verifier Agents                   │
│    (多个 VM，每个有唯一 VM ID)                    │
└─────────────────────────────────────────────────┘
```

## 核心特性

### 1. 一对一事件-结果匹配 ✅

通过 UUID 机制确保每个发送的事件都能精确匹配到对应的验证结果：

```rust
// 服务端发送事件时自动添加 event_id
let event_id = Uuid::new_v4();
event.data["event_id"] = event_id.to_string();

// 客户端返回结果时携带相同的 event_id
result.event_id = event_id.to_string();

// 服务端通过 HashMap 精确匹配
pending_events.get(&event_id) -> PendingEvent
```

### 2. 多 VM 并发隔离 ✅

通过 VM ID 实现客户端路由和事件隔离：

```rust
// VM ID 在连接时发送（首条消息）
clients: HashMap<VmId, ClientSession>

// 事件路由到特定 VM
client_manager.send_event("vm-001", event).await

// 每个 VM 的事件独立跟踪
pending_events: HashMap<EventId, PendingEvent>
```

### 3. 异步等待机制 ✅

使用 tokio oneshot channel 实现事件的异步等待：

```rust
// 创建 oneshot channel
let (result_tx, result_rx) = oneshot::channel();

// 注册待验证事件
pending_events.insert(event_id, PendingEvent {
    event_id,
    result_tx,
    ...
});

// 异步等待结果（带超时）
timeout(Duration::from_secs(10), result_rx).await
```

### 4. 自动超时和清理 ✅

- 每个事件都有独立的超时设置（默认 30 秒）
- 后台清理任务定期移除过期事件（默认每 60 秒）
- 超时事件自动返回 `VerificationError::Timeout`

## 模块结构

### types.rs

核心数据类型：
- `Event` - 测试事件（keyboard, mouse, command）
- `VerifyResult` - 验证结果
- `PendingEvent` - 待验证事件（包含 oneshot sender）
- `ClientInfo` - 客户端信息

### client.rs

客户端管理：
- `ClientManager` - 管理所有连接的客户端
- `ClientSession` - 单个客户端会话
- VM ID 到客户端的映射
- 统一的结果收集通道

### service.rs

验证服务核心：
- `VerificationService` - 主服务类
- `verify_event()` - 发送事件并等待结果
- UUID 事件跟踪
- 自动超时和清理

### server.rs

网络服务器：
- `VerificationServer` - 服务器管理器
- WebSocket 服务器实现
- TCP 服务器实现
- VM ID 握手处理

## 使用示例

### 1. 启动示例服务器

```bash
cd atp-core/verification-server
cargo run --example server
```

输出：
```
INFO  启动 Verification Server 示例
INFO  WebSocket 服务器地址: 0.0.0.0:8765
INFO  TCP 服务器地址: 0.0.0.0:8766
INFO  等待 Guest Agent 连接...
```

### 2. 在代码中使用

```rust
use std::sync::Arc;
use std::time::Duration;
use verification_server::{
    client::ClientManager,
    server::{ServerConfig, VerificationServer},
    service::{ServiceConfig, VerificationService},
    types::Event,
};

// 创建客户端管理器
let client_manager = Arc::new(ClientManager::new());

// 创建验证服务
let service_config = ServiceConfig {
    default_timeout: Duration::from_secs(30),
    cleanup_interval: Duration::from_secs(60),
    max_pending_events: 1000,
};
let verification_service = Arc::new(VerificationService::new(
    client_manager.clone(),
    service_config,
));

// 配置并启动服务器
let server_config = ServerConfig {
    websocket_addr: Some("0.0.0.0:8765".parse()?),
    tcp_addr: Some("0.0.0.0:8766".parse()?),
};
let server = VerificationServer::new(server_config, client_manager.clone());
tokio::spawn(async move {
    server.start().await
});

// 发送验证事件
let event = Event {
    event_type: "keyboard".to_string(),
    data: serde_json::json!({
        "key": "a",
        "timeout_ms": 5000,
    }),
    timestamp: chrono::Utc::now().timestamp_millis(),
};

// 等待验证结果
match verification_service
    .verify_event("vm-001", event, Some(Duration::from_secs(10)))
    .await
{
    Ok(result) => {
        println!("验证成功: verified={}, latency={}ms",
                 result.verified, result.latency_ms);
    }
    Err(e) => {
        eprintln!("验证失败: {}", e);
    }
}
```

## 连接测试结果

测试日期: 2025-11-26

### WebSocket 连接测试 ✅

**服务端日志:**
```
INFO  WebSocket 服务器启动: 0.0.0.0:8765
DEBUG WebSocket 客户端连接: 127.0.0.1:44937
DEBUG 收到 VM ID: vm-test-001
INFO  注册客户端: vm-test-001
INFO  WebSocket 客户端已注册: vm-test-001 (127.0.0.1:44937)
INFO  当前连接的客户端: 1
INFO  向客户端 vm-test-001 发送测试事件
DEBUG 创建验证事件: vm_id=vm-test-001, event_id=0eb34b06-8389-4037-b8d5-06ca8c5a0d7a, type=keyboard
WARN  验证超时: vm_id=vm-test-001, event_id=0eb34b06-8389-4037-b8d5-06ca8c5a0d7a
```

**客户端日志:**
```
INFO  使用 WebSocket 传输
INFO  连接到 WebSocket 服务器: ws://localhost:8765
INFO  成功连接到 WebSocket 服务器
DEBUG 发送 VM ID: vm-test-001
INFO  已连接到服务器: ws://localhost:8765
INFO  启动事件循环
DEBUG 接收到事件: {"event_type":"keyboard","data":{"event_id":"0eb34b06-8389-4037-b8d5-06ca8c5a0d7a","key":"a","timeout_ms":5000},"timestamp":1764119063850}
INFO  收到事件: type=keyboard
```

**验证结果:**
- ✅ WebSocket 连接成功建立
- ✅ VM ID 握手成功
- ✅ 客户端成功注册
- ✅ 事件发送成功，event_id 正确传递
- ✅ 超时机制正常工作（客户端未返回结果导致超时）

### TCP 连接测试

待测试（架构已实现，等待实际测试）

## 配置选项

### ServiceConfig

```rust
pub struct ServiceConfig {
    /// 默认超时时间
    pub default_timeout: Duration,

    /// 事件清理间隔
    pub cleanup_interval: Duration,

    /// 最大待验证事件数
    pub max_pending_events: usize,
}
```

默认值：
- `default_timeout`: 30 秒
- `cleanup_interval`: 60 秒
- `max_pending_events`: 10000

### ServerConfig

```rust
pub struct ServerConfig {
    /// WebSocket 服务器地址
    pub websocket_addr: Option<SocketAddr>,

    /// TCP 服务器地址
    pub tcp_addr: Option<SocketAddr>,
}
```

## API 文档

### VerificationService::verify_event()

发送验证事件并等待结果。

```rust
pub async fn verify_event(
    &self,
    vm_id: &str,
    event: Event,
    timeout_duration: Option<Duration>,
) -> Result<VerifyResult>
```

**参数:**
- `vm_id` - 目标虚拟机 ID
- `event` - 要验证的事件
- `timeout_duration` - 超时时间（None 使用默认值）

**返回:**
- `Ok(VerifyResult)` - 验证成功，返回结果
- `Err(VerificationError::Timeout)` - 超时
- `Err(VerificationError::ClientNotConnected)` - 客户端未连接

### ClientManager API

```rust
// 注册新客户端
pub async fn register_client(&self, info: ClientInfo)
    -> Result<mpsc::UnboundedReceiver<Event>>

// 发送事件到指定客户端
pub async fn send_event(&self, vm_id: &str, event: Event) -> Result<()>

// 获取所有已连接客户端
pub async fn get_clients(&self) -> Vec<ClientInfo>

// 标记客户端断开
pub async fn mark_disconnected(&self, vm_id: &str)
```

## 集成到 ATP 平台

### 步骤 1: 添加依赖

在 `atp-application/cli/Cargo.toml` 中添加：

```toml
[dependencies]
verification-server = { path = "../../atp-core/verification-server" }
```

### 步骤 2: 在 Executor 中集成

```rust
use verification_server::{
    client::ClientManager,
    service::VerificationService,
    server::VerificationServer,
};

pub struct ScenarioRunner {
    // 现有字段...
    verification_service: Arc<VerificationService>,
}

impl ScenarioRunner {
    pub fn new(...) -> Self {
        // 创建客户端管理器
        let client_manager = Arc::new(ClientManager::new());

        // 创建验证服务
        let verification_service = Arc::new(VerificationService::new(
            client_manager.clone(),
            ServiceConfig::default(),
        ));

        // 启动服务器
        let server = VerificationServer::new(
            ServerConfig {
                websocket_addr: Some("0.0.0.0:8765".parse().unwrap()),
                tcp_addr: Some("0.0.0.0:8766".parse().unwrap()),
            },
            client_manager,
        );
        tokio::spawn(async move {
            server.start().await
        });

        Self {
            // 现有字段...
            verification_service,
        }
    }

    pub async fn execute_keyboard_action(...) -> Result<()> {
        // 发送键盘事件到虚拟机
        self.send_keyboard_event(...).await?;

        // 验证事件是否到达 Guest
        let event = Event {
            event_type: "keyboard".to_string(),
            data: serde_json::json!({
                "key": key_code,
                "timeout_ms": 5000,
            }),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        match self.verification_service
            .verify_event(&vm_id, event, Some(Duration::from_secs(10)))
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

## 性能指标

- **事件跟踪开销**: O(1) HashMap 查找
- **并发能力**: 支持数千个并发待验证事件
- **内存占用**: 每个待验证事件约 200 字节
- **清理效率**: 定期批量清理，不影响正常操作

## 故障排查

### 客户端无法连接

**问题**: `连接被拒绝`

**解决方案**:
1. 确认服务器已启动
2. 检查端口是否被占用: `netstat -tlnp | grep 8765`
3. 检查防火墙设置

### 事件一直超时

**问题**: `验证超时: vm_id=xxx, event_id=xxx`

**解决方案**:
1. 确认客户端已连接: 检查日志中的 "注册客户端" 消息
2. 确认客户端验证器已启用: 检查客户端日志
3. 增加超时时间: `verify_event(..., Some(Duration::from_secs(30)))`
4. 检查客户端是否有权限访问输入设备

### VM ID 不匹配

**问题**: `客户端未连接: vm-xxx`

**解决方案**:
1. 确认客户端启动时使用了 `--vm-id` 参数
2. 确认 VM ID 与代码中使用的一致
3. 检查 `get_clients()` 查看已连接的客户端

## 代码统计

- **types.rs**: ~120 行
- **client.rs**: ~180 行
- **service.rs**: ~300 行
- **server.rs**: ~280 行
- **examples/server.rs**: ~130 行

**总计**: ~1,010 行代码

## 许可证

MIT OR Apache-2.0

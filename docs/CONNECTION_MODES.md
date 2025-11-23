# 连接模式设计文档

## 概览

OCloudView ATP 支持两种不同的连接模式，用于与虚拟机进行通信：

1. **Libvirt 复用模式**：用于 QMP、QGA、VirtioSerial 协议
2. **独立多通道模式**：用于 Spice 协议

## 1. Libvirt 复用模式

### 适用协议
- **QMP** (QEMU Monitor Protocol)
- **QGA** (QEMU Guest Agent)
- **VirtioSerial** (自定义协议)

### 连接特性
- **长连接复用**：所有协议共享同一个 libvirt 连接
- **连接管理**：由 libvirt 库负责连接的建立、维护和心跳检测
- **Domain 对象**：每个 VM 对应一个 `virt::domain::Domain` 对象
- **通信方式**：通过 libvirt API 与虚拟机交互

### 架构设计

```rust
┌─────────────────────────────────────────────────────────┐
│                   Protocol Layer                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐     │
│  │QmpProtocol│  │QgaProtocol│  │VirtioSerial...  │     │
│  └────┬─────┘  └────┬─────┘  └────────┬─────────┘     │
│       │             │                  │                │
│       └─────────────┴──────────────────┘                │
└───────────────────────┬─────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────┐
│                  Transport Layer                        │
│         ┌────────────────────────────┐                  │
│         │    HostConnection          │                  │
│         │  - Arc<Mutex<Connect>>     │ ← Libvirt 连接   │
│         │  - ConnectionPool          │                  │
│         └────────────────────────────┘                  │
└───────────────────────┬─────────────────────────────────┘
                        ↓
              ┌──────────────────┐
              │   libvirt API     │
              └──────────────────┘
                        ↓
              ┌──────────────────┐
              │   QEMU/KVM        │
              └──────────────────┘
```

### URI 格式支持

#### SSH 连接 (默认)
```
qemu+ssh://192.168.1.10:22/system
qemu+ssh://user@host:port/system
```

#### TCP 连接 (无加密)
```
qemu+tcp://192.168.1.10/system
qemu+tcp://192.168.1.10:16509/system
```

#### TLS 连接 (加密)
```
qemu+tls://192.168.1.10/system
qemu+tls://192.168.1.10:16514/system
```

#### 本地连接
```
qemu:///system
```

### 配置示例

```rust
use atp_transport::{HostInfo, TransportConfig};

// TCP 连接
let host = HostInfo::new("host1", "192.168.1.10")
    .with_uri("qemu+tcp://192.168.1.10/system");

// TLS 连接
let host_tls = HostInfo::new("host2", "192.168.1.20")
    .with_uri("qemu+tls://192.168.1.20/system");
```

### 连接管理

```rust
pub struct HostConnection {
    /// 主机信息
    host_info: HostInfo,

    /// Libvirt 连接（所有协议共享）
    connection: Arc<Mutex<Option<Connect>>>,

    /// 连接状态
    state: Arc<Mutex<ConnectionState>>,

    /// 配置
    config: Arc<TransportConfig>,
}

impl HostConnection {
    /// 获取连接供协议层使用
    pub async fn get_connection(&self) -> Result<Arc<Mutex<Option<Connect>>>> {
        // QMP/QGA/VirtioSerial 协议都使用这个方法获取连接
        Ok(Arc::clone(&self.connection))
    }
}
```

### 协议实现示例

```rust
#[async_trait]
impl Protocol for QmpProtocol {
    async fn connect(&mut self, domain: &Domain) -> Result<()> {
        // 通过 libvirt Domain 对象建立 QMP 连接
        // 不需要单独的网络连接
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        // 使用 libvirt API 发送数据
    }
}
```

## 2. 独立多通道模式

### 适用协议
- **Spice** (Simple Protocol for Independent Computing Environments)

### 连接特性
- **多通道架构**：每个 VM 有多个独立的 Spice 通道
- **独立连接**：每个通道是独立的 TCP/WebSocket 连接
- **不依赖 libvirt**：直接连接到 Spice 服务器
- **动态通道管理**：通道可以动态创建和销毁

### Spice 通道类型

```rust
pub enum SpiceChannelType {
    /// 主通道 (必需)
    Main,

    /// 显示通道 (视频输出)
    Display,

    /// 输入通道 (键盘/鼠标)
    Inputs,

    /// 光标通道
    Cursor,

    /// Playback 通道 (音频输出)
    Playback,

    /// Record 通道 (音频输入)
    Record,

    /// USB 重定向通道
    UsbRedir,

    /// 智能卡通道
    SmartCard,

    /// WebDAV 通道 (文件共享)
    WebDav,

    /// 端口通道 (串口/并口重定向)
    Port,
}
```

### 架构设计

```rust
┌─────────────────────────────────────────────────────────┐
│                   Protocol Layer                        │
│             ┌────────────────────┐                      │
│             │   SpiceProtocol    │                      │
│             └─────────┬──────────┘                      │
│                       │                                  │
└───────────────────────┼──────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────┐
│              Spice Connection Manager                   │
│   ┌─────────────────────────────────────────────┐      │
│   │        SpiceVmConnection (每个 VM)          │      │
│   │  ┌─────────┐  ┌─────────┐  ┌──────────┐    │      │
│   │  │Main Ch  │  │Display Ch│  │Inputs Ch │    │      │
│   │  └────┬────┘  └────┬─────┘  └────┬─────┘    │      │
│   │       │            │             │           │      │
│   └───────┼────────────┼─────────────┼───────────┘      │
│           │            │             │                  │
└───────────┼────────────┼─────────────┼──────────────────┘
            ↓            ↓             ↓
    ┌────────────┐ ┌────────────┐ ┌────────────┐
    │TCP Socket 1│ │TCP Socket 2│ │TCP Socket 3│
    └────────────┘ └────────────┘ └────────────┘
            ↓            ↓             ↓
    ┌──────────────────────────────────────┐
    │        Spice Server (QEMU)           │
    └──────────────────────────────────────┘
```

### Spice 连接管理器设计

```rust
/// Spice 虚拟机连接
pub struct SpiceVmConnection {
    /// VM 标识
    vm_id: String,

    /// Spice 服务器地址
    server_addr: String,

    /// Spice 端口
    port: u16,

    /// 密码 (可选)
    password: Option<String>,

    /// TLS 配置 (可选)
    tls_config: Option<SpiceTlsConfig>,

    /// 活跃通道
    channels: Arc<RwLock<HashMap<SpiceChannelType, SpiceChannel>>>,

    /// 连接状态
    state: Arc<Mutex<ConnectionState>>,
}

impl SpiceVmConnection {
    /// 创建新的 Spice VM 连接
    pub fn new(vm_id: &str, server_addr: &str, port: u16) -> Self {
        Self {
            vm_id: vm_id.to_string(),
            server_addr: server_addr.to_string(),
            port,
            password: None,
            tls_config: None,
            channels: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
        }
    }

    /// 连接主通道
    pub async fn connect_main_channel(&mut self) -> Result<()> {
        // 连接 Main 通道（必需）
        let channel = SpiceChannel::connect(
            &self.server_addr,
            self.port,
            SpiceChannelType::Main,
            self.password.as_deref(),
        ).await?;

        self.channels.write().await.insert(SpiceChannelType::Main, channel);
        *self.state.lock().await = ConnectionState::Connected;

        Ok(())
    }

    /// 打开额外通道
    pub async fn open_channel(&mut self, channel_type: SpiceChannelType) -> Result<()> {
        // 动态创建新通道
        let channel = SpiceChannel::connect(
            &self.server_addr,
            self.port,
            channel_type,
            self.password.as_deref(),
        ).await?;

        self.channels.write().await.insert(channel_type, channel);

        Ok(())
    }

    /// 关闭通道
    pub async fn close_channel(&mut self, channel_type: &SpiceChannelType) -> Result<()> {
        let mut channels = self.channels.write().await;
        if let Some(mut channel) = channels.remove(channel_type) {
            channel.disconnect().await?;
        }
        Ok(())
    }

    /// 获取通道
    pub async fn get_channel(&self, channel_type: &SpiceChannelType) -> Option<SpiceChannel> {
        self.channels.read().await.get(channel_type).cloned()
    }

    /// 关闭所有通道
    pub async fn disconnect_all(&mut self) -> Result<()> {
        let mut channels = self.channels.write().await;
        for (_, mut channel) in channels.drain() {
            let _ = channel.disconnect().await;
        }
        *self.state.lock().await = ConnectionState::Disconnected;
        Ok(())
    }
}

/// Spice 连接管理器 (管理多个 VM)
pub struct SpiceConnectionManager {
    /// VM 连接映射
    vm_connections: Arc<RwLock<HashMap<String, SpiceVmConnection>>>,
}

impl SpiceConnectionManager {
    pub fn new() -> Self {
        Self {
            vm_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 添加 VM 连接
    pub async fn add_vm(&mut self, vm_id: &str, server_addr: &str, port: u16) -> Result<()> {
        let mut conn = SpiceVmConnection::new(vm_id, server_addr, port);
        conn.connect_main_channel().await?;

        self.vm_connections.write().await.insert(vm_id.to_string(), conn);
        Ok(())
    }

    /// 获取 VM 连接
    pub async fn get_vm_connection(&self, vm_id: &str) -> Option<SpiceVmConnection> {
        self.vm_connections.read().await.get(vm_id).cloned()
    }

    /// 移除 VM 连接
    pub async fn remove_vm(&mut self, vm_id: &str) -> Result<()> {
        if let Some(mut conn) = self.vm_connections.write().await.remove(vm_id) {
            conn.disconnect_all().await?;
        }
        Ok(())
    }
}
```

### Spice 通道实现

```rust
/// Spice 通道
pub struct SpiceChannel {
    /// 通道类型
    channel_type: SpiceChannelType,

    /// TCP 连接
    stream: TcpStream,

    /// 通道 ID
    channel_id: u8,

    /// 状态
    state: ConnectionState,
}

impl SpiceChannel {
    /// 连接到 Spice 服务器
    pub async fn connect(
        server_addr: &str,
        port: u16,
        channel_type: SpiceChannelType,
        password: Option<&str>,
    ) -> Result<Self> {
        // 建立 TCP 连接
        let stream = TcpStream::connect(format!("{}:{}", server_addr, port)).await?;

        // Spice 握手
        // 1. 发送链接头
        // 2. 接收服务器能力
        // 3. 发送客户端能力
        // 4. 认证 (如果需要)

        Ok(Self {
            channel_type,
            stream,
            channel_id: 0,
            state: ConnectionState::Connected,
        })
    }

    /// 发送数据
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write_all(data).await?;
        Ok(())
    }

    /// 接收数据
    pub async fn receive(&mut self) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; 4096];
        let n = self.stream.read(&mut buffer).await?;
        buffer.truncate(n);
        Ok(buffer)
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        self.stream.shutdown().await?;
        self.state = ConnectionState::Disconnected;
        Ok(())
    }
}
```

### 协议实现示例

```rust
#[async_trait]
impl Protocol for SpiceProtocol {
    async fn connect(&mut self, domain: &Domain) -> Result<()> {
        // 从 domain 获取 Spice 配置
        let spice_info = self.get_spice_info(domain)?;

        // 创建 SpiceVmConnection
        let mut conn = SpiceVmConnection::new(
            &domain.get_name()?,
            &spice_info.host,
            spice_info.port,
        );

        // 连接主通道
        conn.connect_main_channel().await?;

        // 根据需要打开其他通道
        conn.open_channel(SpiceChannelType::Display).await?;
        conn.open_channel(SpiceChannelType::Inputs).await?;

        self.connection = Some(conn);
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        // 选择合适的通道发送数据
        if let Some(conn) = &mut self.connection {
            if let Some(mut channel) = conn.get_channel(&SpiceChannelType::Inputs).await {
                channel.send(data).await?;
            }
        }
        Ok(())
    }
}
```

## 3. 对比总结

| 特性 | Libvirt 复用模式 | Spice 独立多通道模式 |
|------|-----------------|-------------------|
| **适用协议** | QMP, QGA, VirtioSerial | Spice |
| **连接数量** | 每个主机一个连接 | 每个 VM 多个通道 |
| **连接管理** | libvirt 库管理 | 应用层管理 |
| **连接类型** | libvirt Connection | TCP/WebSocket |
| **生命周期** | 长连接，持续复用 | 动态创建/销毁 |
| **依赖** | libvirt + QEMU | QEMU Spice Server |
| **URI 格式** | qemu+tcp://, qemu+tls://, qemu+ssh:// | spice://host:port |
| **心跳检测** | libvirt 内置 | 应用层实现 |
| **故障恢复** | libvirt 自动重连 | 应用层重连 |

## 4. 传输层架构更新

### 统一的传输管理器

```rust
pub struct TransportManager {
    /// Libvirt 连接池
    libvirt_pool: ConnectionPool,

    /// Spice 连接管理器
    spice_manager: SpiceConnectionManager,

    /// 配置
    config: Arc<TransportConfig>,
}

impl TransportManager {
    /// 获取 Libvirt 连接 (用于 QMP/QGA/VirtioSerial)
    pub async fn get_libvirt_connection(&self, host_id: &str) -> Result<Arc<Mutex<Option<Connect>>>> {
        self.libvirt_pool.get_connection(host_id).await
    }

    /// 获取 Spice 连接 (用于 Spice 协议)
    pub async fn get_spice_connection(&self, vm_id: &str) -> Result<SpiceVmConnection> {
        self.spice_manager.get_vm_connection(vm_id).await
            .ok_or(TransportError::DomainNotFound(vm_id.to_string()))
    }

    /// 添加 Spice VM
    pub async fn add_spice_vm(&mut self, vm_id: &str, server_addr: &str, port: u16) -> Result<()> {
        self.spice_manager.add_vm(vm_id, server_addr, port).await
    }
}
```

## 5. 配置示例

### YAML 配置

```yaml
# 主机配置
hosts:
  - id: host1
    host: 192.168.1.10
    # TCP 连接 (无加密)
    uri: qemu+tcp://192.168.1.10/system

  - id: host2
    host: 192.168.1.20
    # TLS 连接 (加密)
    uri: qemu+tls://192.168.1.20/system

  - id: host3
    host: 192.168.1.30
    # SSH 连接 (默认)
    uri: qemu+ssh://root@192.168.1.30:22/system

# Spice 配置
spice:
  vms:
    - vm_id: vm1
      server_addr: 192.168.1.10
      port: 5900
      password: optional_password
      tls: false

    - vm_id: vm2
      server_addr: 192.168.1.20
      port: 5901
      tls: true
      tls_verify: true
```

### Rust 配置

```rust
use atp_transport::{HostInfo, TransportManager, TransportConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let mut manager = TransportManager::new(TransportConfig::default());

    // 添加 Libvirt 主机
    manager.add_host(
        HostInfo::new("host1", "192.168.1.10")
            .with_uri("qemu+tcp://192.168.1.10/system")
    ).await?;

    manager.add_host(
        HostInfo::new("host2", "192.168.1.20")
            .with_uri("qemu+tls://192.168.1.20/system")
    ).await?;

    // 添加 Spice VM
    manager.add_spice_vm("vm1", "192.168.1.10", 5900).await?;
    manager.add_spice_vm("vm2", "192.168.1.20", 5901).await?;

    Ok(())
}
```

## 6. 开发路线图

### Phase 1: Libvirt 模式完善 (当前)
- [x] 支持 SSH URI
- [ ] 支持 TCP URI (qemu+tcp://)
- [ ] 支持 TLS URI (qemu+tls://)
- [ ] URI 解析和验证

### Phase 2: Spice 基础支持
- [ ] Spice 通道定义
- [ ] Spice 连接管理器
- [ ] Main 通道实现
- [ ] Inputs 通道实现

### Phase 3: Spice 高级功能
- [ ] Display 通道实现
- [ ] USB 重定向
- [ ] 音频通道
- [ ] 文件共享

### Phase 4: 性能优化
- [ ] 通道池管理
- [ ] 连接复用优化
- [ ] 心跳机制优化
- [ ] 故障恢复优化

## 7. 安全考虑

### Libvirt TLS 连接
- 需要配置 TLS 证书
- 证书验证
- 加密通信

### Spice TLS 连接
- Spice 服务器 TLS 配置
- 客户端证书
- 加密通道

### 认证
- Libvirt: SSH 密钥、用户名密码
- Spice: SASL 认证、密码认证

## 8. 性能指标

### Libvirt 模式
- 连接建立: < 500ms
- 命令执行: < 10ms
- 心跳间隔: 30s
- 重连延迟: 指数退避 (1s - 60s)

### Spice 模式
- 通道建立: < 200ms (每个通道)
- 输入延迟: < 5ms
- 显示刷新: 60 FPS
- 通道数量: 8-12 (典型)

## 9. 故障处理

### Libvirt 连接故障
1. 检测：心跳失败
2. 标记：ConnectionState::Disconnected
3. 重连：指数退避
4. 通知：上层协议

### Spice 通道故障
1. 检测：TCP 连接断开
2. 标记：通道状态
3. 重连：单个通道重连
4. 降级：关键通道优先

## 10. 测试策略

### 单元测试
- 连接建立
- 数据发送/接收
- 故障注入
- 重连逻辑

### 集成测试
- 多主机连接
- 多通道管理
- 并发访问
- 故障恢复

### 性能测试
- 连接池性能
- 通道切换延迟
- 并发连接数
- 资源占用

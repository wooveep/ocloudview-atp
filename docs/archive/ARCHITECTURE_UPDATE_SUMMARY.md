# 架构设计更新总结

## 更新日期
2025-11-23

## 更新背景

根据项目实际需求，对 OCloudView ATP 的架构设计进行了重要更新，主要关注以下几个方面：

1. **协议连接模式的明确区分**
2. **Spice 协议的特殊性**
3. **Libvirt URI 格式的完整支持**

## 核心更新内容

### 1. 两种连接模式的明确定义

#### 1.1 Libvirt 复用模式

**适用协议：** QMP、QGA、VirtioSerial

**核心特性：**
- 所有协议复用同一个 libvirt 长连接
- 连接由 libvirt 库管理（心跳、重连等）
- 通过 libvirt API 与虚拟机交互
- 每个主机一个连接，多个 VM 共享

**示意图：**
```
┌─────────────────────────────────────────┐
│  QMP Protocol    QGA Protocol           │
│       │               │                  │
│       └───────┬───────┘                  │
│               ↓                          │
│      HostConnection                      │
│    (libvirt Connect)                     │
└───────────────┬─────────────────────────┘
                ↓
         Libvirt API
                ↓
          QEMU/KVM
                ↓
          VM1, VM2, VM3...
```

#### 1.2 Spice 独立多通道模式

**适用协议：** Spice

**核心特性：**
- **每个 VM 有多个独立的 Spice 通道**
- **每个通道是独立的 TCP 连接**
- **不依赖 libvirt，直接连接 Spice 服务器**
- 通道可以动态创建和销毁

**示意图：**
```
┌─────────────────────────────────────────┐
│           SpiceProtocol                 │
│                 ↓                        │
│        SpiceVmConnection                │
│  ┌──────┐  ┌──────┐  ┌──────┐          │
│  │Main  │  │Display│  │Inputs│  ...    │
│  │Ch    │  │Ch     │  │Ch    │          │
│  └──┬───┘  └──┬────┘  └──┬───┘          │
└─────┼─────────┼──────────┼──────────────┘
      ↓         ↓          ↓
   TCP Sock  TCP Sock  TCP Sock
      ↓         ↓          ↓
    ┌──────────────────────┐
    │   Spice Server       │
    │   (QEMU)             │
    └──────────────────────┘
```

### 2. Spice 通道类型定义

```rust
pub enum SpiceChannelType {
    Main,       // 主通道 (必需，控制通道)
    Display,    // 显示通道 (视频输出)
    Inputs,     // 输入通道 (键盘/鼠标)
    Cursor,     // 光标通道
    Playback,   // 音频输出通道
    Record,     // 音频输入通道
    UsbRedir,   // USB 重定向通道
    SmartCard,  // 智能卡通道
    WebDav,     // 文件共享通道
    Port,       // 端口重定向通道 (串口/并口)
}
```

**通道特性：**
- **Main 通道必需**：用于控制和协商其他通道
- **其他通道可选**：根据需要动态打开
- **独立生命周期**：每个通道可以独立创建和销毁
- **多实例支持**：某些通道类型可以有多个实例（如 UsbRedir）

### 3. Libvirt URI 格式支持

#### 3.1 SSH 连接（默认）
```
qemu+ssh://192.168.1.10:22/system
qemu+ssh://root@192.168.1.10:22/system
```
- 使用 SSH 协议
- 需要 SSH 密钥或密码认证
- 默认端口 22

#### 3.2 TCP 连接（无加密）
```
qemu+tcp://192.168.1.10/system
qemu+tcp://192.168.1.10:16509/system
```
- 无加密的 TCP 连接
- 性能较好，但不安全
- 默认端口 16509
- 适用于内部网络

#### 3.3 TLS 连接（加密）
```
qemu+tls://192.168.1.10/system
qemu+tls://192.168.1.10:16514/system
```
- 使用 TLS 加密
- 需要证书配置
- 默认端口 16514
- 生产环境推荐

#### 3.4 本地连接
```
qemu:///system
```
- 本地 Unix Socket
- 不需要网络
- 性能最优

### 4. 架构组件更新

#### 4.1 传输层新增组件

**SpiceVmConnection**
```rust
pub struct SpiceVmConnection {
    vm_id: String,
    server_addr: String,
    port: u16,
    password: Option<String>,
    tls_config: Option<SpiceTlsConfig>,
    channels: Arc<RwLock<HashMap<SpiceChannelType, SpiceChannel>>>,
    state: Arc<Mutex<ConnectionState>>,
}
```

**SpiceConnectionManager**
```rust
pub struct SpiceConnectionManager {
    vm_connections: Arc<RwLock<HashMap<String, SpiceVmConnection>>>,
}
```

**TransportManager 更新**
```rust
pub struct TransportManager {
    libvirt_pool: ConnectionPool,           // Libvirt 连接池
    spice_manager: SpiceConnectionManager,  // Spice 连接管理器
    config: Arc<TransportConfig>,
}
```

#### 4.2 协议层更新

**ProtocolType 保持不变**
```rust
pub enum ProtocolType {
    QMP,
    QGA,
    VirtioSerial(String),  // 自定义协议名称
    Spice,                  // 预留
}
```

### 5. 关键设计决策

#### 5.1 为什么 Spice 不复用 libvirt 连接？

1. **协议特性不同**：
   - QMP/QGA 是命令-响应模式，适合通过 libvirt API
   - Spice 是流式多通道协议，需要持续的双向通信

2. **通道独立性**：
   - 每个通道需要独立的连接和状态管理
   - 通道可以动态创建和销毁
   - 不同通道有不同的数据流特性

3. **性能考虑**：
   - 显示通道需要高带宽、低延迟
   - 音频通道需要实时性
   - 直接 TCP 连接性能更好

4. **扩展性**：
   - 可以支持 WebSocket 等其他传输方式
   - 可以实现自定义的流控制
   - 可以针对不同通道优化

#### 5.2 连接管理的挑战

**Libvirt 模式的挑战：**
- 连接池管理
- 并发访问控制
- 连接失效检测和恢复

**Spice 模式的挑战：**
- 多通道同步
- 通道故障隔离
- 资源占用（每个通道一个连接）
- 通道生命周期管理

### 6. 配置示例

#### 6.1 YAML 配置示例

```yaml
# 传输层配置
transport:
  # Libvirt 主机
  hosts:
    - id: host1
      host: 192.168.1.10
      uri: qemu+tcp://192.168.1.10/system
      tags: [production, zone-a]

    - id: host2
      host: 192.168.1.20
      uri: qemu+tls://192.168.1.20/system
      tags: [production, zone-b]

  # Spice 虚拟机
  spice:
    vms:
      - vm_id: vm1
        server_addr: 192.168.1.10
        port: 5900
        password: "optional_password"
        channels:
          - Main      # 必需
          - Display   # 显示
          - Inputs    # 输入
          - Cursor    # 光标

      - vm_id: vm2
        server_addr: 192.168.1.20
        port: 5901
        tls: true
        tls_verify: true
        channels:
          - Main
          - Display
          - Inputs
          - UsbRedir  # USB 重定向
```

#### 6.2 Rust 代码示例

```rust
use atp_transport::{HostInfo, TransportManager, TransportConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let mut manager = TransportManager::new(TransportConfig::default());

    // 添加 Libvirt 主机（支持多种 URI）
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

    // 使用 Libvirt 连接（QMP/QGA/VirtioSerial）
    let conn = manager.get_libvirt_connection("host1").await?;

    // 使用 Spice 连接
    let spice_conn = manager.get_spice_connection("vm1").await?;

    Ok(())
}
```

### 7. 实施计划

#### Phase 1: 完善 Libvirt 模式（当前）
- [x] 基础 HostConnection 实现
- [x] 连接池实现
- [ ] URI 解析器（支持 TCP/TLS/SSH）
- [ ] 配置验证

#### Phase 2: Spice 基础支持
- [ ] SpiceChannel 基础结构
- [ ] SpiceVmConnection 实现
- [ ] SpiceConnectionManager 实现
- [ ] Main 通道实现
- [ ] Inputs 通道实现

#### Phase 3: Spice 高级功能
- [ ] Display 通道实现
- [ ] 音频通道实现
- [ ] USB 重定向实现
- [ ] 文件共享实现

#### Phase 4: 性能优化
- [ ] 通道池化
- [ ] 连接复用优化
- [ ] 故障恢复优化
- [ ] 监控指标完善

### 8. 性能对比

| 指标 | Libvirt 模式 | Spice 模式 |
|------|-------------|-----------|
| **连接建立** | < 500ms | < 200ms (每通道) |
| **命令延迟** | < 10ms | < 5ms (输入) |
| **并发能力** | 50+ VMs/主机 | 取决于通道数 |
| **资源占用** | 低（共享连接） | 中高（多连接） |
| **故障隔离** | 主机级别 | 通道级别 |
| **扩展性** | 受 libvirt 限制 | 高度灵活 |

### 9. 安全考虑

#### Libvirt 连接安全
- **SSH**：密钥认证，加密传输
- **TLS**：证书认证，加密传输
- **TCP**：明文传输，仅限内网

#### Spice 连接安全
- **密码认证**：简单密码保护
- **SASL 认证**：更强的认证机制
- **TLS 加密**：通道加密
- **证书验证**：防止中间人攻击

### 10. 文档索引

- **主架构文档**：[docs/LAYERED_ARCHITECTURE.md](./LAYERED_ARCHITECTURE.md)
- **连接模式详细设计**：[docs/CONNECTION_MODES.md](./CONNECTION_MODES.md)
- **传输层实现**：[atp-core/transport/](../atp-core/transport/)
- **协议层实现**：[atp-core/protocol/](../atp-core/protocol/)

## 总结

此次架构更新明确了以下关键点：

1. **清晰的连接模式划分**：Libvirt 复用模式 vs Spice 多通道模式
2. **Spice 的独特性**：每个 VM 多个独立通道，不复用 libvirt 连接
3. **完整的 URI 支持**：TCP、TLS、SSH 三种 libvirt 连接方式
4. **灵活的架构**：既保证了性能，又保证了可扩展性

这些更新为后续的 Spice 协议实现和多主机管理奠定了坚实的基础。
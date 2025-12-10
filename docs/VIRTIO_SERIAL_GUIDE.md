# VirtioSerial 自定义协议支持

**日期**: 2025-11-24
**作者**: OCloudView ATP Team
**模块**: atp-protocol/virtio

## 概述

VirtioSerial 模块提供了通过 virtio-serial 通道与虚拟机内自定义 agent 通信的能力。与 QGA 不同，VirtioSerial 支持完全自定义的通信协议，允许用户根据具体需求设计和实现协议格式。

## 核心组件

### 1. VirtioChannel - 通道管理

负责管理 virtio-serial Unix Socket 连接。

**功能**:
- 从 libvirt Domain XML 自动发现通道路径
- 支持直接指定 socket 路径
- 提供原始数据收发接口
- 支持字符串和行读取

**示例**:
```rust
// 方式1: 从 domain 发现通道
let channel = VirtioChannel::discover_from_domain(&domain, "com.vmagent.sock").await?;

// 方式2: 直接指定路径
let channel = VirtioChannel::new(
    "com.vmagent.sock",
    PathBuf::from("/var/lib/libvirt/qemu/channel/target/uuid")
);

// 连接
channel.connect().await?;

// 发送数据
channel.send_string("hello").await?;

// 接收数据
let response = channel.receive_line().await?;
```

### 2. ProtocolHandler - 协议处理器

定义协议编码/解码逻辑的 trait，允许用户实现自定义协议。

**内置处理器**:

#### RawProtocolHandler
- 不做任何处理，直接传输原始数据
- 适合自定义二进制协议

```rust
let builder = VirtioSerialBuilder::new("com.vmagent.sock")
    .with_raw_handler();
```

#### JsonProtocolHandler
- 自动将数据包装为 JSON 格式
- 默认格式：`{"data": "your message"}\n`
- 可自定义字段名

```rust
// 使用默认字段
let builder = VirtioSerialBuilder::new("com.vmagent.sock")
    .with_json_handler();

// 自定义字段
let builder = VirtioSerialBuilder::new("com.vmagent.sock")
    .with_custom_json_handler("command", "response");
```

### 3. VirtioSerialProtocol - 协议实现

实现了 `Protocol` trait，提供统一的协议接口。

**功能**:
- 支持可插拔的协议处理器
- 提供请求-响应模式
- 集成到协议注册表

## 使用方法

### 基础使用

```rust
use atp_protocol::{VirtioSerialBuilder, Protocol};

// 1. 创建协议实例
let builder = VirtioSerialBuilder::new("com.vmagent.sock")
    .with_socket_path(PathBuf::from("/path/to/socket"))
    .with_raw_handler();

let mut protocol = builder.build();

// 2. 连接到虚拟机
protocol.connect(&domain).await?;

// 3. 发送数据
protocol.send(b"custom command\n").await?;

// 4. 接收响应
let response = protocol.receive().await?;

// 5. 断开连接
protocol.disconnect().await?;
```

### 使用 JSON 协议

```rust
use atp_protocol::{VirtioSerialBuilder, VirtioSerialProtocol};

// 创建 JSON 协议
let builder = VirtioSerialBuilder::new("com.vmagent.sock")
    .with_json_handler();

let mut protocol = builder.build();

// 发送数据（会自动编码为 JSON）
protocol.send(b"test message").await?;
// 实际发送: {"data":"test message"}\n

// 接收响应（会自动解码）
let response = protocol.receive().await?;
```

### 自定义协议处理器

```rust
use atp_protocol::ProtocolHandler;
use async_trait::async_trait;

struct MyProtocolHandler {
    // 自定义字段
}

#[async_trait]
impl ProtocolHandler for MyProtocolHandler {
    async fn encode_request(&self, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        // 实现自定义编码逻辑
        // 例如：添加消息头、CRC校验等
        let mut encoded = Vec::new();
        encoded.extend_from_slice(b"HEADER:");
        encoded.extend_from_slice(data);
        encoded.push(b'\n');
        Ok(encoded)
    }

    async fn decode_response(&self, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        // 实现自定义解码逻辑
        // 例如：验证消息头、校验CRC等
        if data.starts_with(b"RESPONSE:") {
            Ok(data[9..].to_vec())
        } else {
            Err(ProtocolError::ParseError("无效响应".to_string()))
        }
    }

    fn name(&self) -> &str {
        "my-protocol"
    }
}

// 使用自定义处理器
let builder = VirtioSerialBuilder::new("com.vmagent.sock")
    .with_handler(Arc::new(MyProtocolHandler { /* ... */ }));
```

## 虚拟机配置

### libvirt XML 配置

在虚拟机 XML 中添加 virtio-serial 通道：

```xml
<devices>
  <channel type='unix'>
    <source mode='bind' path='/var/lib/libvirt/qemu/channel/target/YOUR-UUID'/>
    <target type='virtio' name='com.vmagent.sock' state='connected'/>
    <address type='virtio-serial' controller='0' bus='0' port='4'/>
  </channel>
</devices>
```

### Guest Agent 配置

在虚拟机内部，agent 需要连接到 `/dev/virtio-ports/com.vmagent.sock`：

```bash
# 查看可用的 virtio-serial 端口
ls -la /dev/virtio-ports/

# 示例输出
lrwxrwxrwx 1 root root 11 Nov 24 10:00 com.vmagent.sock -> ../vport2p1
```

## Guest Agent 实现示例

### Python Agent 示例

```python
#!/usr/bin/env python3
import json

# 连接到 virtio-serial 端口
with open('/dev/virtio-ports/com.vmagent.sock', 'r+b', buffering=0) as f:
    while True:
        # 读取请求
        line = f.readline()
        if not line:
            break

        try:
            # 解析 JSON 请求
            request = json.loads(line)
            command = request.get('data', '')

            # 处理命令
            if command == 'ping':
                response = {'result': 'pong'}
            elif command.startswith('exec:'):
                # 执行命令
                import subprocess
                cmd = command[5:]
                result = subprocess.run(cmd, shell=True, capture_output=True)
                response = {
                    'result': result.stdout.decode(),
                    'code': result.returncode
                }
            else:
                response = {'result': 'unknown command'}

            # 发送响应
            f.write(json.dumps(response).encode() + b'\n')
            f.flush()

        except Exception as e:
            error_response = {'error': str(e)}
            f.write(json.dumps(error_response).encode() + b'\n')
            f.flush()
```

### Shell Agent 示例

```bash
#!/bin/bash

VIRTIO_PORT="/dev/virtio-ports/com.vmagent.sock"

# 持续读取请求
while IFS= read -r line; do
    # 简单的命令处理
    case "$line" in
        "ping")
            echo "pong" > "$VIRTIO_PORT"
            ;;
        "status")
            echo "{\"status\":\"running\",\"uptime\":$(uptime -s)}" > "$VIRTIO_PORT"
            ;;
        *)
            echo "unknown" > "$VIRTIO_PORT"
            ;;
    esac
done < "$VIRTIO_PORT"
```

## 使用场景

### 1. 自定义监控 Agent

```rust
// 查询系统状态
let mut protocol = create_virtio_protocol("com.monitoring.sock").await?;
protocol.send(b"{\"command\":\"get_metrics\"}").await?;
let metrics = protocol.receive().await?;
```

### 2. 应用控制

```rust
// 启动/停止应用
let mut protocol = create_virtio_protocol("com.appctl.sock").await?;
protocol.send(b"{\"action\":\"start\",\"app\":\"nginx\"}").await?;
let result = protocol.receive().await?;
```

### 3. 文件传输

```rust
// 发送文件块
for chunk in file_chunks {
    protocol.send(&chunk).await?;
}
```

## 性能特性

- **延迟**: < 1ms（本地 Unix Socket）
- **吞吐量**: 取决于 virtio-serial 通道配置
- **并发**: 支持多通道并发通信

## 与 QGA 的对比

| 特性 | QGA | VirtioSerial |
|------|-----|--------------|
| 协议标准化 | 标准 JSON-RPC | 完全自定义 |
| 学习成本 | 低 | 中 |
| 灵活性 | 低 | 高 |
| 功能集 | 固定 | 可扩展 |
| 通道类型 | qemu-ga channel | virtio-serial |
| 最佳用途 | 标准系统操作 | 自定义应用通信 |

## 最佳实践

1. **协议设计**
   - 使用文本协议便于调试
   - 添加版本号支持协议演进
   - 包含错误处理机制

2. **连接管理**
   - 实现重连逻辑
   - 添加超时控制
   - 处理连接断开场景

3. **错误处理**
   - 验证所有输入数据
   - 提供详细的错误信息
   - 记录关键操作日志

4. **安全性**
   - 验证命令权限
   - 限制操作范围
   - 避免执行任意命令

## 调试技巧

### 1. 查看通道状态

```bash
# 主机端
virsh qemu-monitor-command DOMAIN --hmp "info chardev"

# 或查看 XML
virsh dumpxml DOMAIN | grep -A 5 channel
```

### 2. 手动测试通道

```bash
# 在主机端直接写入 socket
echo "test" | nc -U /var/lib/libvirt/qemu/channel/target/UUID

# 在虚拟机内读取
cat /dev/virtio-ports/com.vmagent.sock
```

### 3. 启用日志

```rust
// 设置 RUST_LOG 环境变量
RUST_LOG=atp_protocol=debug cargo run
```

## 故障排除

### 问题：无法连接到通道

**可能原因**:
- Socket 文件不存在
- 权限不足
- 虚拟机未运行

**解决方法**:
```bash
# 检查 socket 存在
ls -la /var/lib/libvirt/qemu/channel/target/

# 检查权限
sudo chown your-user:libvirt /var/lib/libvirt/qemu/channel/target/*

# 检查虚拟机状态
virsh list --all
```

### 问题：虚拟机内无法看到端口

**可能原因**:
- virtio-serial 模块未加载
- XML 配置错误

**解决方法**:
```bash
# 检查模块
lsmod | grep virtio

# 加载模块
modprobe virtio_console

# 检查设备
ls -la /dev/virtio-ports/
```

## 代码统计

- **channel.rs**: ~280 行
- **protocol.rs**: ~330 行
- **mod.rs**: ~20 行
- **示例代码**: ~150 行

**总计**: ~780 行代码

## 参考资料

- [virtio-serial 规范](https://www.linux-kvm.org/page/Virtio)
- [libvirt Domain XML 格式](https://libvirt.org/formatdomain.html#serial-port)
- [QEMU 文档](https://www.qemu.org/docs/master/specs/index.html)

## 更新日志

### 2025-11-24
- ✅ 实现 VirtioChannel 通道管理
- ✅ 实现 ProtocolHandler trait 和内置处理器
- ✅ 实现 VirtioSerialProtocol 和构建器
- ✅ 创建使用示例和文档
- ✅ 集成到 Protocol trait 系统

---

**维护者**: OCloudView ATP Team
**许可证**: MIT OR Apache-2.0

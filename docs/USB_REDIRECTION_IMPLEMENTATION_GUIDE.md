# USB 重定向实现详细指南

## 概述

本文档基于对 usbredir 官方仓库和 SPICE 源代码的深入研究，提供了在 ATP 项目中实现完整 USB 重定向功能的详细路径。

**研究仓库**:
- `https://gitlab.freedesktop.org/spice/usbredir` (usbredir 协议实现)
- `https://gitlab.freedesktop.org/spice/spice` (SPICE 服务器实现)
- `https://gitlab.freedesktop.org/spice/spice-protocol` (SPICE 协议定义)

**当前状态**: 框架已完成，需要实现核心协议解析和 libusb 集成

---

## 架构分析

### usbredir 协议层次

```
┌─────────────────────────────────────────────┐
│         Application Layer                   │
│   (ATP USB Redirection Logic)               │
├─────────────────────────────────────────────┤
│      usbredirparser (协议解析器)             │
│  - Packet serialization/deserialization     │
│  - Callback-based event handling            │
├─────────────────────────────────────────────┤
│      usbredirhost (主机端库)                │
│  - libusb device interaction                │
│  - USB traffic redirection                  │
├─────────────────────────────────────────────┤
│         Transport Layer                     │
│  (SPICE Channel / TCP Socket)               │
└─────────────────────────────────────────────┘
```

### 协议消息类型

从 `usbredirproto.h` 分析得出：

**控制消息** (0-99):
1. `usb_redir_hello` - 握手和能力协商
2. `usb_redir_device_connect` - 设备连接通知
3. `usb_redir_device_disconnect` - 设备断开
4. `usb_redir_reset` - 重置设备
5. `usb_redir_interface_info` - 接口信息
6. `usb_redir_ep_info` - 端点信息
7. `usb_redir_set_configuration` - 设置配置
8. `usb_redir_get_configuration` - 获取配置
9. `usb_redir_start_iso_stream` - 启动 ISO 流
10. `usb_redir_start_interrupt_receiving` - 启动中断接收
11. `usb_redir_alloc_bulk_streams` - 分配批量流 (USB 3.0)
12. `usb_redir_filter_filter` - 设备过滤规则

**数据消息** (100+):
1. `usb_redir_control_packet` (100) - 控制传输
2. `usb_redir_bulk_packet` (101) - 批量传输
3. `usb_redir_iso_packet` (102) - 同步传输
4. `usb_redir_interrupt_packet` (113) - 中断传输
5. `usb_redir_buffered_bulk_packet` (114) - 缓冲批量传输

---

## 实现路径

### 阶段 1: usbredir 协议解析器 (高优先级)

**目标**: 实现 usbredir 消息的序列化和反序列化

#### 文件: `atp-core/protocol/src/spice/usbredir/proto.rs`

```rust
//! usbredir 协议消息定义
//!
//! 基于 usbredirproto.h v0.7.1

/// usbredir 版本
pub const USBREDIR_VERSION: u32 = 0x000701;

/// 传输状态
#[repr(u8)]
pub enum UsbRedirStatus {
    Success = 0,
    Cancelled = 1,    // 传输被取消
    Invalid = 2,      // 无效的包类型/长度/端点
    IoError = 3,      // IO 错误
    Stall = 4,        // 管道停止
    Timeout = 5,      // 超时
    Babble = 6,       // 设备 "babble"
}

/// USB 传输类型
#[repr(u8)]
pub enum UsbRedirType {
    Control = 0,
    Iso = 1,
    Bulk = 2,
    Interrupt = 3,
    Invalid = 255,
}

/// USB 速度
#[repr(u8)]
pub enum UsbRedirSpeed {
    Low = 0,
    Full = 1,
    High = 2,
    Super = 3,
    Unknown = 255,
}

/// usbredir 消息头部
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct UsbRedirHeader {
    /// 消息类型
    pub msg_type: u32,
    /// 消息长度
    pub length: u32,
    /// 消息 ID (用于匹配请求-响应)
    pub id: u64,
}

impl UsbRedirHeader {
    pub const SIZE: usize = 16;

    pub fn new(msg_type: u32, length: u32, id: u64) -> Self {
        Self { msg_type, length, id }
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.msg_type.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.length.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.id.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }
        Some(Self {
            msg_type: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            length: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            id: u64::from_le_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11],
                bytes[12], bytes[13], bytes[14], bytes[15]
            ]),
        })
    }
}

/// Hello 握手消息
#[derive(Debug, Clone)]
pub struct UsbRedirHello {
    /// 版本字符串 (64 字节)
    pub version: String,
    /// 能力位掩码
    pub capabilities: Vec<u32>,
}

impl UsbRedirHello {
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
            capabilities: vec![
                // 支持的能力
                (1 << 0) |  // bulk_streams
                (1 << 1) |  // connect_device_version
                (1 << 2) |  // filter
                (1 << 3) |  // device_disconnect_ack
                (1 << 4) |  // ep_info_max_packet_size
                (1 << 5) |  // 64bits_ids
                (1 << 6) |  // 32bits_bulk_length
                (1 << 7),   // bulk_receiving
            ],
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 64]; // 版本字符串
        let version_bytes = self.version.as_bytes();
        let copy_len = version_bytes.len().min(63);
        bytes[..copy_len].copy_from_slice(&version_bytes[..copy_len]);

        // 添加能力
        for cap in &self.capabilities {
            bytes.extend_from_slice(&cap.to_le_bytes());
        }

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 64 {
            return None;
        }

        // 解析版本字符串
        let version_end = bytes[..64].iter().position(|&b| b == 0).unwrap_or(64);
        let version = String::from_utf8_lossy(&bytes[..version_end]).to_string();

        // 解析能力
        let mut capabilities = Vec::new();
        let mut offset = 64;
        while offset + 4 <= bytes.len() {
            let cap = u32::from_le_bytes([
                bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]
            ]);
            capabilities.push(cap);
            offset += 4;
        }

        Some(Self { version, capabilities })
    }
}

/// 设备连接消息
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct UsbRedirDeviceConnect {
    pub speed: u8,
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_version_bcd: u16,
}

impl UsbRedirDeviceConnect {
    pub const SIZE: usize = 10;

    pub fn to_bytes(&self) -> [u8; 10] {
        let mut bytes = [0u8; 10];
        bytes[0] = self.speed;
        bytes[1] = self.device_class;
        bytes[2] = self.device_subclass;
        bytes[3] = self.device_protocol;
        bytes[4..6].copy_from_slice(&self.vendor_id.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.product_id.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.device_version_bcd.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 10 {
            return None;
        }
        Some(Self {
            speed: bytes[0],
            device_class: bytes[1],
            device_subclass: bytes[2],
            device_protocol: bytes[3],
            vendor_id: u16::from_le_bytes([bytes[4], bytes[5]]),
            product_id: u16::from_le_bytes([bytes[6], bytes[7]]),
            device_version_bcd: u16::from_le_bytes([bytes[8], bytes[9]]),
        })
    }
}

/// 控制传输包头部
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct UsbRedirControlPacketHeader {
    pub endpoint: u8,
    pub request: u8,
    pub requesttype: u8,
    pub status: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16,
}

impl UsbRedirControlPacketHeader {
    pub const SIZE: usize = 10;

    pub fn to_bytes(&self) -> [u8; 10] {
        let mut bytes = [0u8; 10];
        bytes[0] = self.endpoint;
        bytes[1] = self.request;
        bytes[2] = self.requesttype;
        bytes[3] = self.status;
        bytes[4..6].copy_from_slice(&self.value.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.index.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.length.to_le_bytes());
        bytes
    }
}

/// 批量传输包头部
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct UsbRedirBulkPacketHeader {
    pub endpoint: u8,
    pub status: u8,
    pub length: u32,      // 32位长度 (cap_32bits_bulk_length)
    pub stream_id: u32,   // USB 3.0 流 ID
    pub length_high: u16, // 高16位长度
}

// 更多消息类型定义...
// TODO: 实现所有 usbredir 消息类型
```

**实现步骤**:

1. ✅ 定义基础类型 (Status, Type, Speed)
2. ✅ 实现 UsbRedirHeader 序列化/反序列化
3. ✅ 实现 Hello 握手消息
4. ✅ 实现 DeviceConnect 消息
5. 🔲 实现所有控制消息头部
6. 🔲 实现所有数据消息头部
7. 🔲 添加单元测试

**难度**: ⭐⭐☆☆☆ (中等)
**工作量**: ~500 行代码

---

### 阶段 2: usbredir 解析器 (高优先级)

**目标**: 实现消息解析和回调机制

#### 文件: `atp-core/protocol/src/spice/usbredir/parser.rs`

```rust
//! usbredir 协议解析器
//!
//! 提供消息解析和基于回调的事件处理

use super::proto::*;
use crate::Result;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// 解析器回调 trait
pub trait UsbRedirParserCallbacks: Send + Sync {
    /// Hello 消息
    fn on_hello(&mut self, hello: UsbRedirHello);

    /// 设备连接
    fn on_device_connect(&mut self, device: UsbRedirDeviceConnect);

    /// 设备断开
    fn on_device_disconnect(&mut self);

    /// 控制传输
    fn on_control_packet(&mut self, id: u64, header: UsbRedirControlPacketHeader, data: Vec<u8>);

    /// 批量传输
    fn on_bulk_packet(&mut self, id: u64, header: UsbRedirBulkPacketHeader, data: Vec<u8>);

    /// 中断传输
    fn on_interrupt_packet(&mut self, id: u64, data: Vec<u8>);

    // TODO: 添加所有回调方法
}

/// usbredir 协议解析器
pub struct UsbRedirParser<T: AsyncReadExt + AsyncWriteExt + Unpin> {
    /// 传输层 (TCP/SPICE Channel)
    transport: T,
    /// 下一个消息 ID
    next_id: u64,
    /// 待处理的请求
    pending_requests: HashMap<u64, PendingRequest>,
}

struct PendingRequest {
    msg_type: u32,
    timestamp: std::time::Instant,
}

impl<T: AsyncReadExt + AsyncWriteExt + Unpin> UsbRedirParser<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_id: 1,
            pending_requests: HashMap::new(),
        }
    }

    /// 发送 Hello 握手
    pub async fn send_hello(&mut self, version: &str) -> Result<()> {
        let hello = UsbRedirHello::new(version);
        let data = hello.to_bytes();

        let header = UsbRedirHeader::new(
            0, // usb_redir_hello
            data.len() as u32,
            0, // Hello 不需要 ID
        );

        self.transport.write_all(&header.to_bytes()).await?;
        self.transport.write_all(&data).await?;
        self.transport.flush().await?;

        Ok(())
    }

    /// 发送设备连接通知
    pub async fn send_device_connect(&mut self, device: UsbRedirDeviceConnect) -> Result<()> {
        let data = device.to_bytes();

        let header = UsbRedirHeader::new(
            1, // usb_redir_device_connect
            data.len() as u32,
            0,
        );

        self.transport.write_all(&header.to_bytes()).await?;
        self.transport.write_all(&data).await?;
        self.transport.flush().await?;

        Ok(())
    }

    /// 发送控制传输
    pub async fn send_control_packet(
        &mut self,
        header: UsbRedirControlPacketHeader,
        data: &[u8],
    ) -> Result<u64> {
        let id = self.next_id;
        self.next_id += 1;

        let msg_header = UsbRedirHeader::new(
            100, // usb_redir_control_packet
            (UsbRedirControlPacketHeader::SIZE + data.len()) as u32,
            id,
        );

        self.transport.write_all(&msg_header.to_bytes()).await?;
        self.transport.write_all(&header.to_bytes()).await?;
        self.transport.write_all(data).await?;
        self.transport.flush().await?;

        self.pending_requests.insert(id, PendingRequest {
            msg_type: 100,
            timestamp: std::time::Instant::now(),
        });

        Ok(id)
    }

    /// 接收并处理消息
    pub async fn process_message<C: UsbRedirParserCallbacks>(
        &mut self,
        callbacks: &mut C,
    ) -> Result<()> {
        // 读取消息头部
        let mut header_buf = [0u8; UsbRedirHeader::SIZE];
        self.transport.read_exact(&mut header_buf).await?;

        let header = UsbRedirHeader::from_bytes(&header_buf)
            .ok_or_else(|| crate::ProtocolError::ParseError("Invalid header".to_string()))?;

        // 读取消息数据
        let mut data = vec![0u8; header.length as usize];
        if header.length > 0 {
            self.transport.read_exact(&mut data).await?;
        }

        // 根据消息类型分发
        match header.msg_type {
            0 => {
                // usb_redir_hello
                if let Some(hello) = UsbRedirHello::from_bytes(&data) {
                    callbacks.on_hello(hello);
                }
            }
            1 => {
                // usb_redir_device_connect
                if let Some(device) = UsbRedirDeviceConnect::from_bytes(&data) {
                    callbacks.on_device_connect(device);
                }
            }
            2 => {
                // usb_redir_device_disconnect
                callbacks.on_device_disconnect();
            }
            100 => {
                // usb_redir_control_packet
                if data.len() >= UsbRedirControlPacketHeader::SIZE {
                    let ctrl_header = UsbRedirControlPacketHeader::from_bytes(
                        &data[..UsbRedirControlPacketHeader::SIZE]
                    ).unwrap();
                    let packet_data = data[UsbRedirControlPacketHeader::SIZE..].to_vec();
                    callbacks.on_control_packet(header.id, ctrl_header, packet_data);
                }
            }
            // TODO: 处理所有消息类型
            _ => {
                tracing::debug!("Unknown usbredir message type: {}", header.msg_type);
            }
        }

        // 清理已完成的请求
        self.pending_requests.remove(&header.id);

        Ok(())
    }

    /// 获取待处理请求数量
    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }
}

// TODO: 实现更多协议方法
```

**实现步骤**:

1. ✅ 定义回调 trait
2. ✅ 实现解析器基础结构
3. ✅ 实现 Hello 握手
4. ✅ 实现设备连接通知
5. ✅ 实现控制传输
6. 🔲 实现批量/中断/ISO 传输
7. 🔲 实现所有控制消息
8. 🔲 添加错误处理和超时
9. 🔲 添加单元测试

**难度**: ⭐⭐⭐☆☆ (中等偏难)
**工作量**: ~700 行代码

---

### 阶段 3: libusb 集成 (中优先级)

**目标**: 使用 Rust libusb 绑定与 USB 设备交互

#### 依赖库

在 `atp-core/protocol/Cargo.toml` 添加:

```toml
[dependencies]
rusb = "0.9"  # Rust libusb 绑定
```

#### 文件: `atp-core/protocol/src/spice/usbredir/host.rs`

```rust
//! USB 主机端实现
//!
//! 使用 libusb (rusb) 与本地 USB 设备交互

use rusb::{Context, DeviceHandle, Device, UsbContext};
use super::proto::*;
use super::parser::*;
use crate::Result;
use std::time::Duration;

/// USB 设备包装器
pub struct UsbDeviceHost {
    /// libusb 上下文
    context: Context,
    /// 设备句柄
    handle: Option<DeviceHandle<Context>>,
    /// 设备信息
    device_info: UsbRedirDeviceConnect,
}

impl UsbDeviceHost {
    /// 打开 USB 设备
    pub fn open(vendor_id: u16, product_id: u16) -> Result<Self> {
        let context = Context::new()
            .map_err(|e| crate::ProtocolError::ConnectionFailed(
                format!("Failed to create libusb context: {}", e)
            ))?;

        // 查找设备
        let device = find_device(&context, vendor_id, product_id)?;
        let handle = device.open()
            .map_err(|e| crate::ProtocolError::ConnectionFailed(
                format!("Failed to open device: {}", e)
            ))?;

        // 获取设备描述符
        let desc = device.device_descriptor()
            .map_err(|e| crate::ProtocolError::ParseError(
                format!("Failed to get device descriptor: {}", e)
            ))?;

        let device_info = UsbRedirDeviceConnect {
            speed: speed_to_usbredir(device.speed()),
            device_class: desc.class_code(),
            device_subclass: desc.sub_class_code(),
            device_protocol: desc.protocol_code(),
            vendor_id: desc.vendor_id(),
            product_id: desc.product_id(),
            device_version_bcd: desc.device_version().into(),
        };

        Ok(Self {
            context,
            handle: Some(handle),
            device_info,
        })
    }

    /// 获取设备信息
    pub fn device_info(&self) -> &UsbRedirDeviceConnect {
        &self.device_info
    }

    /// 声明接口
    pub fn claim_interface(&mut self, interface: u8) -> Result<()> {
        let handle = self.handle.as_mut()
            .ok_or_else(|| crate::ProtocolError::ConnectionFailed(
                "Device not opened".to_string()
            ))?;

        handle.claim_interface(interface)
            .map_err(|e| crate::ProtocolError::CommandFailed(
                format!("Failed to claim interface: {}", e)
            ))?;

        Ok(())
    }

    /// 控制传输
    pub fn control_transfer(
        &mut self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &mut [u8],
        timeout: Duration,
    ) -> Result<usize> {
        let handle = self.handle.as_mut()
            .ok_or_else(|| crate::ProtocolError::ConnectionFailed(
                "Device not opened".to_string()
            ))?;

        // TODO: 根据 request_type 判断读/写方向
        let direction = request_type & 0x80;

        let len = if direction == 0x80 {
            // IN (设备到主机)
            handle.read_control(request_type, request, value, index, data, timeout)
        } else {
            // OUT (主机到设备)
            handle.write_control(request_type, request, value, index, data, timeout)
        }.map_err(|e| crate::ProtocolError::IoError(
            std::io::Error::new(std::io::ErrorKind::Other, e)
        ))?;

        Ok(len)
    }

    /// 批量传输 (IN)
    pub fn bulk_read(
        &mut self,
        endpoint: u8,
        data: &mut [u8],
        timeout: Duration,
    ) -> Result<usize> {
        let handle = self.handle.as_mut()
            .ok_or_else(|| crate::ProtocolError::ConnectionFailed(
                "Device not opened".to_string()
            ))?;

        handle.read_bulk(endpoint, data, timeout)
            .map_err(|e| crate::ProtocolError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e)
            ))
    }

    /// 批量传输 (OUT)
    pub fn bulk_write(
        &mut self,
        endpoint: u8,
        data: &[u8],
        timeout: Duration,
    ) -> Result<usize> {
        let handle = self.handle.as_mut()
            .ok_or_else(|| crate::ProtocolError::ConnectionFailed(
                "Device not opened".to_string()
            ))?;

        handle.write_bulk(endpoint, data, timeout)
            .map_err(|e| crate::ProtocolError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e)
            ))
    }

    /// 中断传输 (IN)
    pub fn interrupt_read(
        &mut self,
        endpoint: u8,
        data: &mut [u8],
        timeout: Duration,
    ) -> Result<usize> {
        let handle = self.handle.as_mut()
            .ok_or_else(|| crate::ProtocolError::ConnectionFailed(
                "Device not opened".to_string()
            ))?;

        handle.read_interrupt(endpoint, data, timeout)
            .map_err(|e| crate::ProtocolError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e)
            ))
    }

    /// 中断传输 (OUT)
    pub fn interrupt_write(
        &mut self,
        endpoint: u8,
        data: &[u8],
        timeout: Duration,
    ) -> Result<usize> {
        let handle = self.handle.as_mut()
            .ok_or_else(|| crate::ProtocolError::ConnectionFailed(
                "Device not opened".to_string()
            ))?;

        handle.write_interrupt(endpoint, data, timeout)
            .map_err(|e| crate::ProtocolError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e)
            ))
    }

    /// 释放接口
    pub fn release_interface(&mut self, interface: u8) -> Result<()> {
        let handle = self.handle.as_mut()
            .ok_or_else(|| crate::ProtocolError::ConnectionFailed(
                "Device not opened".to_string()
            ))?;

        handle.release_interface(interface)
            .map_err(|e| crate::ProtocolError::CommandFailed(
                format!("Failed to release interface: {}", e)
            ))?;

        Ok(())
    }

    /// 关闭设备
    pub fn close(&mut self) {
        self.handle = None;
    }
}

/// 查找 USB 设备
fn find_device(
    context: &Context,
    vendor_id: u16,
    product_id: u16,
) -> Result<Device<Context>> {
    let devices = context.devices()
        .map_err(|e| crate::ProtocolError::ConnectionFailed(
            format!("Failed to get device list: {}", e)
        ))?;

    for device in devices.iter() {
        let desc = device.device_descriptor()
            .map_err(|e| crate::ProtocolError::ParseError(
                format!("Failed to get descriptor: {}", e)
            ))?;

        if desc.vendor_id() == vendor_id && desc.product_id() == product_id {
            return Ok(device);
        }
    }

    Err(crate::ProtocolError::ConnectionFailed(
        format!("Device {}:{} not found", vendor_id, product_id)
    ))
}

/// 转换 USB 速度
fn speed_to_usbredir(speed: rusb::Speed) -> u8 {
    match speed {
        rusb::Speed::Low => UsbRedirSpeed::Low as u8,
        rusb::Speed::Full => UsbRedirSpeed::Full as u8,
        rusb::Speed::High => UsbRedirSpeed::High as u8,
        rusb::Speed::Super => UsbRedirSpeed::Super as u8,
        _ => UsbRedirSpeed::Unknown as u8,
    }
}

/// 枚举所有 USB 设备
pub fn enumerate_devices() -> Result<Vec<(u16, u16, String)>> {
    let context = Context::new()
        .map_err(|e| crate::ProtocolError::ConnectionFailed(
            format!("Failed to create context: {}", e)
        ))?;

    let devices = context.devices()
        .map_err(|e| crate::ProtocolError::ConnectionFailed(
            format!("Failed to get devices: {}", e)
        ))?;

    let mut result = Vec::new();

    for device in devices.iter() {
        if let Ok(desc) = device.device_descriptor() {
            let vendor_id = desc.vendor_id();
            let product_id = desc.product_id();

            // 尝试获取产品字符串
            let product_str = if let Ok(handle) = device.open() {
                handle.read_product_string_ascii(&desc)
                    .unwrap_or_else(|_| format!("Unknown Device"))
            } else {
                format!("Device {:04x}:{:04x}", vendor_id, product_id)
            };

            result.push((vendor_id, product_id, product_str));
        }
    }

    Ok(result)
}

// TODO: 实现异步 USB 传输
// TODO: 实现 ISO 传输
// TODO: 实现流式批量传输 (USB 3.0)
```

**实现步骤**:

1. ✅ 集成 rusb 库
2. ✅ 实现设备打开/关闭
3. ✅ 实现控制传输
4. ✅ 实现批量传输
5. ✅ 实现中断传输
6. 🔲 实现 ISO 传输
7. 🔲 实现异步传输
8. 🔲 实现设备枚举
9. 🔲 添加错误处理
10. 🔲 添加单元测试

**难度**: ⭐⭐⭐⭐☆ (难)
**工作量**: ~800 行代码
**依赖**: rusb crate (libusb 绑定)

---

### 阶段 4: USB 重定向桥接 (中优先级)

**目标**: 连接 usbredir 协议和 libusb，实现完整的重定向流程

#### 文件: `atp-core/protocol/src/spice/usbredir/bridge.rs`

```rust
//! USB 重定向桥接
//!
//! 连接 libusb 设备和 usbredir 协议

use super::host::UsbDeviceHost;
use super::parser::{UsbRedirParser, UsbRedirParserCallbacks};
use super::proto::*;
use crate::Result;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// USB 重定向桥接
pub struct UsbRedirBridge {
    /// USB 设备主机端
    device: UsbDeviceHost,
    /// usbredir 解析器
    parser: UsbRedirParser<Box<dyn tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send>>,
    /// 已声明的接口
    claimed_interfaces: Vec<u8>,
    /// 传输请求通道
    tx: mpsc::UnboundedSender<TransferRequest>,
    /// 传输响应通道
    rx: mpsc::UnboundedReceiver<TransferResponse>,
}

struct TransferRequest {
    id: u64,
    transfer_type: TransferType,
}

enum TransferType {
    Control {
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: Vec<u8>,
    },
    Bulk {
        endpoint: u8,
        data: Vec<u8>,
    },
    Interrupt {
        endpoint: u8,
        data: Vec<u8>,
    },
}

struct TransferResponse {
    id: u64,
    status: UsbRedirStatus,
    data: Vec<u8>,
}

impl UsbRedirBridge {
    pub fn new(
        device: UsbDeviceHost,
        transport: Box<dyn tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            device,
            parser: UsbRedirParser::new(transport),
            claimed_interfaces: Vec::new(),
            tx,
            rx,
        }
    }

    /// 启动桥接
    pub async fn start(&mut self) -> Result<()> {
        info!("启动 USB 重定向桥接");

        // 发送 Hello 握手
        self.parser.send_hello(&format!("ATP-usbredir-{}", env!("CARGO_PKG_VERSION"))).await?;

        // 发送设备连接通知
        self.parser.send_device_connect(*self.device.device_info()).await?;

        info!("USB 重定向桥接已启动");
        Ok(())
    }

    /// 处理消息循环
    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                // 处理来自远程的消息
                result = self.parser.process_message(&mut *self) => {
                    result?;
                }

                // 处理本地传输请求
                Some(request) = self.rx.recv() => {
                    self.handle_transfer_request(request).await?;
                }
            }
        }
    }

    /// 处理传输请求
    async fn handle_transfer_request(&mut self, request: TransferRequest) -> Result<()> {
        let response = match request.transfer_type {
            TransferType::Control { request_type, request, value, index, mut data } => {
                match self.device.control_transfer(
                    request_type,
                    request,
                    value,
                    index,
                    &mut data,
                    Duration::from_secs(5),
                ) {
                    Ok(len) => TransferResponse {
                        id: request.id,
                        status: UsbRedirStatus::Success,
                        data: data[..len].to_vec(),
                    },
                    Err(e) => {
                        warn!("Control transfer failed: {}", e);
                        TransferResponse {
                            id: request.id,
                            status: UsbRedirStatus::IoError,
                            data: Vec::new(),
                        }
                    }
                }
            }
            TransferType::Bulk { endpoint, data } => {
                // 根据端点方向决定读/写
                let direction = endpoint & 0x80;
                let result = if direction == 0x80 {
                    // IN
                    let mut buf = vec![0u8; 8192];
                    self.device.bulk_read(endpoint, &mut buf, Duration::from_secs(5))
                        .map(|len| buf[..len].to_vec())
                } else {
                    // OUT
                    self.device.bulk_write(endpoint, &data, Duration::from_secs(5))
                        .map(|_| Vec::new())
                };

                match result {
                    Ok(data) => TransferResponse {
                        id: request.id,
                        status: UsbRedirStatus::Success,
                        data,
                    },
                    Err(e) => {
                        warn!("Bulk transfer failed: {}", e);
                        TransferResponse {
                            id: request.id,
                            status: UsbRedirStatus::IoError,
                            data: Vec::new(),
                        }
                    }
                }
            }
            TransferType::Interrupt { endpoint, data } => {
                let direction = endpoint & 0x80;
                let result = if direction == 0x80 {
                    let mut buf = vec![0u8; 1024];
                    self.device.interrupt_read(endpoint, &mut buf, Duration::from_secs(1))
                        .map(|len| buf[..len].to_vec())
                } else {
                    self.device.interrupt_write(endpoint, &data, Duration::from_secs(1))
                        .map(|_| Vec::new())
                };

                match result {
                    Ok(data) => TransferResponse {
                        id: request.id,
                        status: UsbRedirStatus::Success,
                        data,
                    },
                    Err(e) => {
                        warn!("Interrupt transfer failed: {}", e);
                        TransferResponse {
                        id: request.id,
                            status: UsbRedirStatus::IoError,
                            data: Vec::new(),
                        }
                    }
                }
            }
        };

        // TODO: 发送响应回远程

        Ok(())
    }
}

impl UsbRedirParserCallbacks for UsbRedirBridge {
    fn on_hello(&mut self, hello: UsbRedirHello) {
        info!("收到 Hello: version={}", hello.version);
    }

    fn on_device_connect(&mut self, device: UsbRedirDeviceConnect) {
        info!("远程请求连接设备: {:04x}:{:04x}",
              device.vendor_id, device.product_id);
    }

    fn on_device_disconnect(&mut self) {
        info!("远程断开设备");
        // 释放所有接口
        for interface in self.claimed_interfaces.clone() {
            let _ = self.device.release_interface(interface);
        }
        self.claimed_interfaces.clear();
    }

    fn on_control_packet(&mut self, id: u64, header: UsbRedirControlPacketHeader, data: Vec<u8>) {
        debug!("收到控制传输请求: id={}", id);

        let request = TransferRequest {
            id,
            transfer_type: TransferType::Control {
                request_type: header.requesttype,
                request: header.request,
                value: header.value,
                index: header.index,
                data,
            },
        };

        let _ = self.tx.send(request);
    }

    fn on_bulk_packet(&mut self, id: u64, header: UsbRedirBulkPacketHeader, data: Vec<u8>) {
        debug!("收到批量传输请求: id={}, endpoint={:02x}", id, header.endpoint);

        let request = TransferRequest {
            id,
            transfer_type: TransferType::Bulk {
                endpoint: header.endpoint,
                data,
            },
        };

        let _ = self.tx.send(request);
    }

    fn on_interrupt_packet(&mut self, id: u64, data: Vec<u8>) {
        debug!("收到中断传输请求: id={}", id);
        // TODO: 从某处获取端点信息
    }
}

// TODO: 实现完整的双向桥接
// TODO: 实现接口管理
// TODO: 实现端点管理
// TODO: 添加错误恢复
```

**实现步骤**:

1. ✅ 定义桥接结构
2. ✅ 实现消息分发
3. ✅ 实现控制传输处理
4. ✅ 实现批量传输处理
5. 🔲 实现中断传输处理
6. 🔲 实现 ISO 传输处理
7. 🔲 实现接口管理
8. 🔲 实现配置管理
9. 🔲 添加错误处理和恢复
10. 🔲 添加集成测试

**难度**: ⭐⭐⭐⭐⭐ (非常难)
**工作量**: ~1000 行代码

---

## 完整实现路线图

### 短期目标 (1-2 周)

1. ✅ 完成 usbredir 协议消息定义 (`proto.rs`)
2. ✅ 实现基础的协议解析器 (`parser.rs`)
3. 🔲 添加单元测试

### 中期目标 (2-4 周)

1. 🔲 集成 rusb 库
2. 🔲 实现 USB 设备主机端 (`host.rs`)
3. 🔲 实现设备枚举功能
4. 🔲 添加集成测试

### 长期目标 (1-2 月)

1. 🔲 实现完整的桥接逻辑 (`bridge.rs`)
2. 🔲 实现所有传输类型
3. 🔲 优化性能和稳定性
4. 🔲 添加端到端测试
5. 🔲 编写用户文档

---

## 技术挑战和解决方案

### 挑战 1: 异步 USB 传输

**问题**: rusb 是同步 API，需要与 tokio 异步运行时集成

**解决方案**:
```rust
// 使用 tokio::task::spawn_blocking 封装同步调用
pub async fn bulk_read_async(
    &self,
    endpoint: u8,
    size: usize,
) -> Result<Vec<u8>> {
    let device = self.device.clone();
    tokio::task::spawn_blocking(move || {
        let mut buf = vec![0u8; size];
        let len = device.bulk_read(endpoint, &mut buf, Duration::from_secs(5))?;
        buf.truncate(len);
        Ok(buf)
    }).await??
}
```

### 挑战 2: USB 设备权限

**问题**: Linux 下访问 USB 设备需要 root 权限或 udev 规则

**解决方案**:
1. 创建 udev 规则文件 `/etc/udev/rules.d/99-usbredir.rules`:
   ```
   SUBSYSTEM=="usb", ATTR{idVendor}=="1234", ATTR{idProduct}=="5678", MODE="0666"
   ```
2. 重新加载 udev: `sudo udevadm control --reload-rules`
3. 或使用 `sudo` 运行程序

### 挑战 3: ISO 传输

**问题**: ISO (同步) 传输需要精确的时序控制

**解决方案**:
- 使用 rusb 的 ISO 传输 API
- 实现缓冲和时间戳管理
- 参考 usbredirhost 的实现

---

## 测试策略

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_serialization() {
        let header = UsbRedirHeader::new(100, 256, 12345);
        let bytes = header.to_bytes();
        let parsed = UsbRedirHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.msg_type, 100);
        assert_eq!(parsed.length, 256);
        assert_eq!(parsed.id, 12345);
    }

    #[test]
    fn test_hello_message() {
        let hello = UsbRedirHello::new("test-version");
        let bytes = hello.to_bytes();
        let parsed = UsbRedirHello::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.version, "test-version");
    }
}
```

### 集成测试

使用虚拟 USB 设备 (如 USB/IP) 进行测试：

```bash
# 在一台机器上启动 USB/IP 服务器
modprobe vhci-hcd
usbip attach -r <remote_host> -b <bus_id>

# 在另一台机器上运行测试
cargo test --test usbredir_integration
```

---

## 参考资源

### 官方文档

1. [usbredir 协议文档](https://www.spice-space.org/usbredir.html)
2. [libusb 文档](https://libusb.info/)
3. [USB 2.0 规范](https://www.usb.org/document-library/usb-20-specification)

### 代码参考

1. `/tmp/spice-research/usbredir/usbredirparser/` - 协议解析器
2. `/tmp/spice-research/usbredir/usbredirhost/` - 主机端实现
3. `/tmp/spice-research/spice/server/red-stream-device.c` - SPICE 集成

### Rust Crates

1. `rusb` - libusb Rust 绑定
2. `tokio` - 异步运行时
3. `bytes` - 字节缓冲管理

---

## 总结

USB 重定向是 SPICE 协议中最复杂的部分，需要：

1. **协议层**: 实现 usbredir 消息的序列化/反序列化
2. **设备层**: 使用 libusb 与 USB 设备交互
3. **桥接层**: 连接协议和设备，处理双向数据流
4. **传输层**: 集成到 SPICE 通道

**总工作量估算**: ~3000 行代码，2-3 个月开发时间

**关键依赖**: rusb (libusb), tokio, SPICE 通道

**建议**: 分阶段实现，先完成基础的控制和批量传输，再扩展到 ISO 和流式传输。

---

**维护者**: OCloudView ATP Team
**最后更新**: 2025-11-25

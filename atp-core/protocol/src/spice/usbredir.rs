//! SPICE USB 重定向通道
//!
//! 实现 USB 设备重定向功能，允许将本地 USB 设备重定向到虚拟机。

use crate::{ProtocolError, Result};
use std::collections::HashMap;
use tracing::{debug, trace, warn};

use super::channel::{ChannelConnection, ChannelType};
use super::constants::*;
use super::messages::MsgUsbredirData;

/// USB 设备信息
#[derive(Debug, Clone)]
pub struct UsbDevice {
    /// 设备 ID
    pub id: u32,
    /// 厂商 ID
    pub vendor_id: u16,
    /// 产品 ID
    pub product_id: u16,
    /// 设备类
    pub device_class: u8,
    /// 设备子类
    pub device_subclass: u8,
    /// 设备协议
    pub device_protocol: u8,
    /// 设备描述
    pub description: String,
    /// 是否已重定向
    pub redirected: bool,
}

/// USB 重定向过滤规则
#[derive(Debug, Clone)]
pub struct UsbFilter {
    /// 允许的设备（vendor_id, product_id, -1 表示任意）
    pub allow: Vec<(i32, i32)>,
    /// 阻止的设备
    pub block: Vec<(i32, i32)>,
}

impl UsbFilter {
    pub fn new() -> Self {
        Self {
            allow: Vec::new(),
            block: Vec::new(),
        }
    }

    /// 允许特定设备
    pub fn allow_device(mut self, vendor_id: u16, product_id: u16) -> Self {
        self.allow.push((vendor_id as i32, product_id as i32));
        self
    }

    /// 允许所有某厂商的设备
    pub fn allow_vendor(mut self, vendor_id: u16) -> Self {
        self.allow.push((vendor_id as i32, -1));
        self
    }

    /// 允许所有设备
    pub fn allow_all(mut self) -> Self {
        self.allow.push((-1, -1));
        self
    }

    /// 阻止特定设备
    pub fn block_device(mut self, vendor_id: u16, product_id: u16) -> Self {
        self.block.push((vendor_id as i32, product_id as i32));
        self
    }

    /// 检查设备是否允许
    pub fn is_allowed(&self, vendor_id: u16, product_id: u16) -> bool {
        // 先检查阻止列表
        for (v, p) in &self.block {
            if (*v == -1 || *v == vendor_id as i32) && (*p == -1 || *p == product_id as i32) {
                return false;
            }
        }

        // 再检查允许列表
        for (v, p) in &self.allow {
            if (*v == -1 || *v == vendor_id as i32) && (*p == -1 || *p == product_id as i32) {
                return true;
            }
        }

        // 默认不允许
        false
    }
}

impl Default for UsbFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// USB 重定向事件
#[derive(Debug, Clone)]
pub enum UsbRedirEvent {
    /// 设备连接
    DeviceConnected(UsbDevice),
    /// 设备断开
    DeviceDisconnected(u32),
    /// 重定向成功
    RedirectSuccess(u32),
    /// 重定向失败
    RedirectFailed { device_id: u32, error: String },
    /// 数据接收
    DataReceived { device_id: u32, data: Vec<u8> },
}

/// SPICE USB 重定向通道
///
/// 管理 USB 设备的重定向
pub struct UsbRedirChannel {
    /// 底层通道连接
    connection: ChannelConnection,
    /// 设备过滤器
    filter: UsbFilter,
    /// 已重定向的设备
    devices: HashMap<u32, UsbDevice>,
    /// 下一个设备 ID
    next_device_id: u32,
}

impl UsbRedirChannel {
    /// 创建新的 USB 重定向通道
    pub fn new(channel_id: u8) -> Self {
        Self {
            connection: ChannelConnection::new(ChannelType::Usbredir, channel_id),
            filter: UsbFilter::new(),
            devices: HashMap::new(),
            next_device_id: 1,
        }
    }

    /// 设置设备过滤器
    pub fn set_filter(&mut self, filter: UsbFilter) {
        self.filter = filter;
    }

    /// 连接到服务器
    pub async fn connect(
        &mut self,
        host: &str,
        port: u16,
        connection_id: u32,
        password: Option<&str>,
    ) -> Result<()> {
        self.connection
            .connect(host, port, connection_id, password)
            .await?;
        debug!("USB 重定向通道已连接");
        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        // 停止所有设备重定向
        for (id, _) in self.devices.drain() {
            debug!("停止设备重定向: {}", id);
        }
        self.connection.disconnect().await
    }

    /// 是否已连接
    pub fn is_connected(&self) -> bool {
        self.connection.is_connected()
    }

    /// 重定向设备
    ///
    /// # Arguments
    /// * `device` - 要重定向的设备信息
    pub async fn redirect_device(&mut self, mut device: UsbDevice) -> Result<u32> {
        if !self.connection.is_connected() {
            return Err(ProtocolError::ConnectionFailed(
                "USB 重定向通道未连接".to_string(),
            ));
        }

        // 检查过滤器
        if !self.filter.is_allowed(device.vendor_id, device.product_id) {
            return Err(ProtocolError::CommandFailed(format!(
                "设备 {:04x}:{:04x} 不在允许列表中",
                device.vendor_id, device.product_id
            )));
        }

        let device_id = self.next_device_id;
        self.next_device_id += 1;

        device.id = device_id;
        device.redirected = true;

        // TODO: 发送 USB 重定向协议消息
        //
        // 实现步骤（基于 usbredir 协议分析）:
        //
        // 1. 发送 usbredir Hello 握手消息
        //    - 创建 usb_redir_header (16 字节): type=0, length, id=0
        //    - 创建 usb_redir_hello_header:
        //      - version: "ATP-usbredir-0.1.0" (64字节字符串)
        //      - capabilities: [USB_REDIR_CAPS_SIZE] 能力位掩码
        //        - cap_bulk_streams (bit 0): USB 3.0 批量流支持
        //        - cap_connect_device_version (bit 1): 设备版本字段
        //        - cap_filter (bit 2): 过滤器支持
        //        - cap_64bits_ids (bit 5): 64位消息ID
        //        - cap_32bits_bulk_length (bit 6): 32位批量长度
        //        - cap_bulk_receiving (bit 7): 批量接收/缓冲输入
        //    示例代码:
        //    ```rust
        //    let hello_header = [0u32; 4]; // type=0, length=68, id=0
        //    let hello_data = b"ATP-usbredir-0.1.0\0".to_vec();
        //    hello_data.resize(64, 0);
        //    let caps = 0b11100111u32; // 启用所需能力
        //    hello_data.extend_from_slice(&caps.to_le_bytes());
        //    ```
        //
        // 2. 等待服务器 Hello 响应
        //    - 读取服务器版本和能力
        //    - 协商通用能力集
        //
        // 3. 发送 usb_redir_device_connect 消息
        //    - 创建 usb_redir_header: type=1, length=10, id=0
        //    - 创建 usb_redir_device_connect_header (10字节):
        //      ```rust
        //      struct DeviceConnect {
        //          speed: u8,           // 0=low, 1=full, 2=high, 3=super
        //          device_class: u8,    // USB 设备类
        //          device_subclass: u8, // USB 设备子类
        //          device_protocol: u8, // USB 设备协议
        //          vendor_id: u16,      // 厂商ID (little-endian)
        //          product_id: u16,     // 产品ID (little-endian)
        //          device_version_bcd: u16, // 设备版本 BCD格式
        //      }
        //      ```
        //
        // 4. 发送 usb_redir_interface_info 消息
        //    - type=4, 包含所有接口信息
        //    - interface_count: u32
        //    - interface[32]: u8 数组，每个接口号
        //    - interface_class[32]: u8 数组
        //    - interface_subclass[32]: u8 数组
        //    - interface_protocol[32]: u8 数组
        //
        // 5. 发送 usb_redir_ep_info 消息
        //    - type=5, 包含所有端点信息
        //    - 对每个端点 (最多32个):
        //      - type[32]: u8 (0=control, 1=iso, 2=bulk, 3=interrupt)
        //      - interval[32]: u8 (中断/ISO传输间隔)
        //      - interface[32]: u8 (所属接口号)
        //      - max_packet_size[32]: u16 (最大包大小)
        //      - max_streams[32]: u32 (USB 3.0 流数量)
        //
        // 参考实现 (来自 usbredir/usbredirhost/usbredirhost.c):
        // ```c
        // usbredirparser_send_device_connect(host->parser, &device_connect);
        // usbredirparser_send_interface_info(host->parser, &interface_info);
        // usbredirparser_send_ep_info(host->parser, &ep_info);
        // ```

        debug!(
            "重定向设备: {} ({:04x}:{:04x})",
            device.description, device.vendor_id, device.product_id
        );

        self.devices.insert(device_id, device);
        Ok(device_id)
    }

    /// 停止设备重定向
    pub async fn stop_redirect(&mut self, device_id: u32) -> Result<()> {
        if let Some(device) = self.devices.remove(&device_id) {
            // TODO: 发送断开设备消息
            //
            // 实现步骤:
            // 1. 发送 usb_redir_device_disconnect 消息
            //    - 创建 usb_redir_header: type=2, length=0, id=0
            //    - 无需额外数据，仅发送头部
            //
            // 2. 等待 usb_redir_device_disconnect_ack 确认 (可选)
            //    - 如果服务器支持 cap_device_disconnect_ack
            //    - type=24, 无数据负载
            //
            // 示例代码:
            // ```rust
            // let disconnect_header = UsbRedirHeader {
            //     msg_type: 2, // usb_redir_device_disconnect
            //     length: 0,
            //     id: 0,
            // };
            // self.connection.send_message(2, &disconnect_header.to_bytes()).await?;
            // ```
            debug!("停止设备重定向: {} ({})", device_id, device.description);
            Ok(())
        } else {
            Err(ProtocolError::CommandFailed(format!(
                "设备 {} 未在重定向中",
                device_id
            )))
        }
    }

    /// 发送数据到设备
    pub async fn send_data(&mut self, device_id: u32, data: &[u8]) -> Result<()> {
        if !self.devices.contains_key(&device_id) {
            return Err(ProtocolError::CommandFailed(format!(
                "设备 {} 未在重定向中",
                device_id
            )));
        }

        let msg = MsgUsbredirData::new(data.to_vec());
        self.connection
            .send_message(msg_usbredir::DATA, &msg.to_bytes())
            .await?;

        trace!("发送 USB 数据: device={}, {} bytes", device_id, data.len());
        Ok(())
    }

    /// 获取已重定向的设备列表
    pub fn devices(&self) -> &HashMap<u32, UsbDevice> {
        &self.devices
    }

    /// 处理服务器消息
    pub async fn process_events(&mut self) -> Result<Option<UsbRedirEvent>> {
        let (msg_type, data) = self.connection.receive_message().await?;

        match msg_type {
            // SPICE_MSG_SPICEVMC_DATA (USB 数据)
            101 => {
                trace!("收到 USB 数据: {} bytes", data.len());
                // TODO: 解析 usbredir 协议数据并分发到对应设备
                //
                // 实现步骤:
                // 1. 解析 usbredir 消息头部 (16字节)
                //    ```rust
                //    struct UsbRedirHeader {
                //        msg_type: u32,  // 消息类型
                //        length: u32,    // 数据长度
                //        id: u64,        // 消息ID (用于匹配请求-响应)
                //    }
                //    ```
                //
                // 2. 根据 msg_type 分发处理:
                //    - 100: usb_redir_control_packet (控制传输)
                //      处理 USB 控制传输响应
                //    - 101: usb_redir_bulk_packet (批量传输)
                //      处理批量数据传输
                //    - 102: usb_redir_iso_packet (同步传输)
                //      处理音视频等同步数据
                //    - 103: usb_redir_interrupt_packet (中断传输)
                //      处理键鼠等中断设备数据
                //    - 114: usb_redir_buffered_bulk_packet (缓冲批量)
                //      处理带缓冲的批量传输 (v0.6+)
                //
                // 3. 解析具体消息内容:
                //    控制传输响应示例:
                //    ```rust
                //    struct ControlPacketHeader {
                //        endpoint: u8,
                //        request: u8,
                //        requesttype: u8,
                //        status: u8,     // 0=成功
                //        value: u16,
                //        index: u16,
                //        length: u16,
                //    }
                //    // 后面跟随 length 字节的数据
                //    ```
                //
                // 4. 触发事件或回调:
                //    ```rust
                //    return Ok(Some(UsbRedirEvent::DataReceived {
                //        device_id: self.get_device_by_endpoint(endpoint),
                //        data: packet_data,
                //    }));
                //    ```
                //
                // 参考 usbredir/usbredirparser/usbredirparser.c:
                // - usbredirparser_do_read() 读取和解析
                // - usbredirparser_handle_msg() 消息分发
                Ok(None)
            }
            _ => {
                debug!("USB 重定向通道消息: type={}", msg_type);
                Ok(None)
            }
        }
    }
}

/// USB 设备列表枚举器（模拟）
///
/// 在实际实现中，这需要使用系统 API 来列举 USB 设备
pub struct UsbDeviceEnumerator;

impl UsbDeviceEnumerator {
    /// 列举本地 USB 设备
    ///
    /// # Note
    /// 这是一个占位实现，实际需要使用 libusb 或系统 API
    pub fn enumerate() -> Vec<UsbDevice> {
        // TODO: 使用 libusb 或系统 API 枚举设备
        warn!("USB 设备枚举尚未实现");
        Vec::new()
    }

    /// 根据 vendor/product ID 查找设备
    pub fn find_device(vendor_id: u16, product_id: u16) -> Option<UsbDevice> {
        Self::enumerate()
            .into_iter()
            .find(|d| d.vendor_id == vendor_id && d.product_id == product_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usb_filter() {
        let filter = UsbFilter::new()
            .allow_vendor(0x1234)
            .block_device(0x1234, 0x0001);

        assert!(filter.is_allowed(0x1234, 0x5678));
        assert!(!filter.is_allowed(0x1234, 0x0001));
        assert!(!filter.is_allowed(0xAAAA, 0xBBBB));
    }

    #[test]
    fn test_usb_filter_allow_all() {
        let filter = UsbFilter::new().allow_all().block_device(0x1234, 0x5678);

        assert!(filter.is_allowed(0xAAAA, 0xBBBB));
        assert!(!filter.is_allowed(0x1234, 0x5678));
    }

    #[test]
    fn test_usb_device() {
        let device = UsbDevice {
            id: 1,
            vendor_id: 0x1234,
            product_id: 0x5678,
            device_class: 0,
            device_subclass: 0,
            device_protocol: 0,
            description: "Test Device".to_string(),
            redirected: false,
        };

        assert_eq!(device.vendor_id, 0x1234);
        assert!(!device.redirected);
    }
}

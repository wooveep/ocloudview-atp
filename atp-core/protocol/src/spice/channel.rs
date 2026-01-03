//! SPICE 通道抽象
//!
//! 定义 SPICE 通道的基础结构和通用功能。

use crate::{ProtocolError, Result};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::{debug, trace};

use super::constants::*;
use super::types::*;

/// 通道类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelType {
    Main,
    Display,
    Inputs,
    Cursor,
    Playback,
    Record,
    Smartcard,
    Usbredir,
    Port,
    Webdav,
}

impl ChannelType {
    pub fn to_u8(self) -> u8 {
        match self {
            Self::Main => channel_type::MAIN,
            Self::Display => channel_type::DISPLAY,
            Self::Inputs => channel_type::INPUTS,
            Self::Cursor => channel_type::CURSOR,
            Self::Playback => channel_type::PLAYBACK,
            Self::Record => channel_type::RECORD,
            Self::Smartcard => channel_type::SMARTCARD,
            Self::Usbredir => channel_type::USBREDIR,
            Self::Port => channel_type::PORT,
            Self::Webdav => channel_type::WEBDAV,
        }
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            channel_type::MAIN => Some(Self::Main),
            channel_type::DISPLAY => Some(Self::Display),
            channel_type::INPUTS => Some(Self::Inputs),
            channel_type::CURSOR => Some(Self::Cursor),
            channel_type::PLAYBACK => Some(Self::Playback),
            channel_type::RECORD => Some(Self::Record),
            channel_type::SMARTCARD => Some(Self::Smartcard),
            channel_type::USBREDIR => Some(Self::Usbredir),
            channel_type::PORT => Some(Self::Port),
            channel_type::WEBDAV => Some(Self::Webdav),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Display => "display",
            Self::Inputs => "inputs",
            Self::Cursor => "cursor",
            Self::Playback => "playback",
            Self::Record => "record",
            Self::Smartcard => "smartcard",
            Self::Usbredir => "usbredir",
            Self::Port => "port",
            Self::Webdav => "webdav",
        }
    }
}

/// SPICE 通道 trait
#[async_trait]
pub trait SpiceChannel: Send + Sync {
    /// 获取通道类型
    fn channel_type(&self) -> ChannelType;

    /// 获取通道 ID
    fn channel_id(&self) -> u8;

    /// 连接通道
    async fn connect(&mut self, host: &str, port: u16, connection_id: u32) -> Result<()>;

    /// 断开连接
    async fn disconnect(&mut self) -> Result<()>;

    /// 是否已连接
    fn is_connected(&self) -> bool;

    /// 发送消息
    async fn send_message(&mut self, msg_type: u16, data: &[u8]) -> Result<()>;

    /// 接收消息
    async fn receive_message(&mut self) -> Result<(u16, Vec<u8>)>;

    /// 处理接收到的消息
    async fn handle_message(&mut self, msg_type: u16, data: &[u8]) -> Result<()>;
}

/// 基础通道连接
pub struct ChannelConnection {
    /// 通道类型
    channel_type: ChannelType,
    /// 通道 ID
    channel_id: u8,
    /// TCP 写入端
    writer: Option<Arc<Mutex<WriteHalf<TcpStream>>>>,
    /// TCP 读取端
    reader: Option<Arc<Mutex<BufReader<ReadHalf<TcpStream>>>>>,
    /// 消息序列号
    serial: AtomicU64,
    /// 连接状态
    connected: bool,
    /// 使用迷你头部
    use_mini_header: bool,
    /// 连接 ID
    connection_id: u32,
}

impl ChannelConnection {
    pub fn new(channel_type: ChannelType, channel_id: u8) -> Self {
        Self {
            channel_type,
            channel_id,
            writer: None,
            reader: None,
            serial: AtomicU64::new(1),
            connected: false,
            use_mini_header: false,
            connection_id: 0,
        }
    }

    /// 连接到 SPICE 服务器
    pub async fn connect(
        &mut self,
        host: &str,
        port: u16,
        connection_id: u32,
        password: Option<&str>,
    ) -> Result<()> {
        let addr = format!("{}:{}", host, port);
        debug!(
            "连接到 SPICE 服务器: {} (通道: {:?})",
            addr, self.channel_type
        );

        // 建立 TCP 连接
        let stream = TcpStream::connect(&addr).await.map_err(|e| {
            ProtocolError::ConnectionFailed(format!("无法连接到 SPICE 服务器 {}: {}", addr, e))
        })?;

        let (read_half, write_half) = tokio::io::split(stream);
        let reader = BufReader::new(read_half);

        self.writer = Some(Arc::new(Mutex::new(write_half)));
        self.reader = Some(Arc::new(Mutex::new(reader)));
        self.connection_id = connection_id;

        // 执行 SPICE 握手
        self.perform_handshake(password).await?;

        self.connected = true;
        debug!("通道 {:?} 连接成功", self.channel_type);

        Ok(())
    }

    /// 执行 SPICE 握手
    async fn perform_handshake(&mut self, password: Option<&str>) -> Result<()> {
        // 1. 发送链接消息
        let link_msg = SpiceLinkMessage::new(self.channel_type.to_u8(), self.channel_id)
            .with_connection_id(self.connection_id);

        let link_data = link_msg.to_bytes();
        let header = SpiceLinkHeader::new(link_data.len() as u32);

        // 发送头部和消息
        let writer = self.writer.as_ref().unwrap();
        let mut writer_guard = writer.lock().await;

        writer_guard
            .write_all(&header.to_bytes())
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        writer_guard
            .write_all(&link_data)
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        writer_guard
            .flush()
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        drop(writer_guard);

        trace!("发送 SPICE Link 消息");

        // 2. 接收服务器回复
        let reader = self.reader.as_ref().unwrap();
        let mut reader_guard = reader.lock().await;

        // 读取链接头部
        let mut header_buf = [0u8; 16];
        reader_guard
            .read_exact(&mut header_buf)
            .await
            .map_err(|e| ProtocolError::ReceiveFailed(format!("读取链接头部失败: {}", e)))?;

        let reply_header = SpiceLinkHeader::from_bytes(&header_buf)
            .ok_or_else(|| ProtocolError::ParseError("无效的链接头部".to_string()))?;

        if !reply_header.is_valid() {
            return Err(ProtocolError::ParseError("无效的 SPICE 魔数".to_string()));
        }

        // 复制值避免引用 packed 字段
        let major = reply_header.major_version;
        let minor = reply_header.minor_version;
        debug!("SPICE 服务器版本: {}.{}", major, minor);

        // 读取回复内容
        let mut reply_buf = vec![0u8; reply_header.size as usize];
        reader_guard
            .read_exact(&mut reply_buf)
            .await
            .map_err(|e| ProtocolError::ReceiveFailed(format!("读取链接回复失败: {}", e)))?;

        let reply = SpiceLinkReply::from_bytes(&reply_buf)
            .ok_or_else(|| ProtocolError::ParseError("无效的链接回复".to_string()))?;

        if !reply.is_ok() {
            return Err(ProtocolError::ConnectionFailed(format!(
                "SPICE 链接错误: {}",
                reply.error
            )));
        }

        drop(reader_guard);

        // 3. 发送认证信息（如果需要密码）
        if let Some(pwd) = password {
            self.send_auth(pwd, &reply.pub_key).await?;
        } else {
            // 发送空认证
            self.send_empty_auth().await?;
        }

        // 检查是否支持 mini header
        // TODO: 从能力协商中确定
        self.use_mini_header = false;

        Ok(())
    }

    /// 发送认证信息
    async fn send_auth(&mut self, _password: &str, _pub_key: &[u8]) -> Result<()> {
        // TODO: 实现 RSA 加密密码
        //
        // 参考实现：spice-gtk/src/channel-main.c 中的 spice_channel_send_link_ack()
        //
        // 1. 解析服务器提供的 RSA 公钥（DER 格式）
        //    - pub_key 字段包含 SPICE_TICKET_PUBKEY_BYTES (162 字节) 的 RSA 公钥
        //    - 使用 rsa crate: use rsa::RsaPublicKey;
        //    - 解析方法：
        //      ```rust
        //      use rsa::pkcs1::DecodeRsaPublicKey;
        //      let public_key = RsaPublicKey::from_pkcs1_der(_pub_key)
        //          .map_err(|e| ProtocolError::ParseError(format!("解析 RSA 公钥失败: {}", e)))?;
        //      ```
        //
        // 2. 准备密码票据（128 字节缓冲区）
        //    - 创建 [u8; 128] 数组，初始化为 0
        //    - 将密码字符串拷贝到缓冲区前面（最多 127 字节，保留一个 \0 结尾）
        //    - 剩余部分用随机数填充以增强安全性
        //      ```rust
        //      use rand::RngCore;
        //      let mut ticket = [0u8; 128];
        //      let pwd_bytes = _password.as_bytes();
        //      let copy_len = std::cmp::min(pwd_bytes.len(), 127);
        //      ticket[..copy_len].copy_from_slice(&pwd_bytes[..copy_len]);
        //      // ticket[copy_len] = 0; // null terminator (already 0)
        //      if copy_len < 127 {
        //          rand::thread_rng().fill_bytes(&mut ticket[copy_len+1..]);
        //      }
        //      ```
        //
        // 3. 使用 RSA-OAEP 加密票据
        //    - 使用 SHA-1 作为哈希函数（SPICE 协议规范）
        //    - 使用 rsa crate 的 Oaep 填充：
        //      ```rust
        //      use rsa::{Oaep, sha1::Sha1};
        //      use rand::thread_rng;
        //
        //      let padding = Oaep::new::<Sha1>();
        //      let mut rng = thread_rng();
        //      let encrypted_ticket = public_key.encrypt(&mut rng, padding, &ticket)
        //          .map_err(|e| ProtocolError::ConnectionFailed(format!("RSA 加密失败: {}", e)))?;
        //      ```
        //
        // 4. 发送加密后的票据（128 字节）
        //    - 加密后的数据大小应该等于 RSA 密钥大小（通常 128 字节）
        //    - 如果加密结果不足 128 字节，需要左侧填充 0
        //      ```rust
        //      let writer = self.writer.as_ref().unwrap();
        //      let mut writer_guard = writer.lock().await;
        //
        //      // 确保是 128 字节
        //      let mut final_ticket = [0u8; 128];
        //      let offset = 128 - encrypted_ticket.len();
        //      final_ticket[offset..].copy_from_slice(&encrypted_ticket);
        //
        //      writer_guard.write_all(&final_ticket).await
        //          .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        //      writer_guard.flush().await
        //          .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        //      drop(writer_guard);
        //      ```
        //
        // 5. 读取认证结果（与 send_empty_auth 相同）
        //    - 接收 4 字节的认证结果
        //    - 0 表示成功，非 0 表示失败
        //      ```rust
        //      let reader = self.reader.as_ref().unwrap();
        //      let mut reader_guard = reader.lock().await;
        //      let mut result = [0u8; 4];
        //      reader_guard.read_exact(&mut result).await
        //          .map_err(|e| ProtocolError::ReceiveFailed(format!("读取认证结果失败: {}", e)))?;
        //      let auth_result = u32::from_le_bytes(result);
        //      if auth_result != 0 {
        //          return Err(ProtocolError::ConnectionFailed(
        //              format!("SPICE 认证失败，错误码: {}", auth_result)
        //          ));
        //      }
        //      debug!("SPICE RSA 认证成功");
        //      ```
        //
        // 需要添加的依赖到 Cargo.toml:
        //   rsa = "0.9"
        //   rand = "0.8"
        //   sha1 = "0.10"
        //
        // 参考资料：
        //   - spice-protocol/spice/protocol.h: SPICE_TICKET_PUBKEY_BYTES 定义
        //   - spice-gtk/src/channel-main.c: spice_channel_send_link_ack() 函数

        // 目前发送空认证（假设服务器不需要密码）
        self.send_empty_auth().await
    }

    /// 发送空认证
    async fn send_empty_auth(&mut self) -> Result<()> {
        let writer = self.writer.as_ref().unwrap();
        let mut writer_guard = writer.lock().await;

        // 发送 128 字节的空票据
        let ticket = [0u8; 128];
        writer_guard
            .write_all(&ticket)
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        writer_guard
            .flush()
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;

        // 读取认证结果
        drop(writer_guard);

        let reader = self.reader.as_ref().unwrap();
        let mut reader_guard = reader.lock().await;

        let mut result = [0u8; 4];
        reader_guard
            .read_exact(&mut result)
            .await
            .map_err(|e| ProtocolError::ReceiveFailed(format!("读取认证结果失败: {}", e)))?;

        let auth_result = u32::from_le_bytes(result);
        if auth_result != 0 {
            return Err(ProtocolError::ConnectionFailed(format!(
                "SPICE 认证失败: {}",
                auth_result
            )));
        }

        debug!("SPICE 认证成功");
        Ok(())
    }

    /// 发送消息
    pub async fn send_message(&mut self, msg_type: u16, data: &[u8]) -> Result<()> {
        let writer = self
            .writer
            .as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("通道未连接".to_string()))?;

        let serial = self.serial.fetch_add(1, Ordering::SeqCst);

        let mut writer_guard = writer.lock().await;

        if self.use_mini_header {
            let header = SpiceMiniDataHeader::new(msg_type, data.len() as u32);
            writer_guard
                .write_all(&header.to_bytes())
                .await
                .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        } else {
            let header = SpiceDataHeader::new(msg_type, data.len() as u32).with_serial(serial);
            writer_guard
                .write_all(&header.to_bytes())
                .await
                .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        }

        if !data.is_empty() {
            writer_guard
                .write_all(data)
                .await
                .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;
        }

        writer_guard
            .flush()
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;

        trace!("发送消息: type={}, size={}", msg_type, data.len());
        Ok(())
    }

    /// 接收消息
    pub async fn receive_message(&mut self) -> Result<(u16, Vec<u8>)> {
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("通道未连接".to_string()))?;

        let mut reader_guard = reader.lock().await;

        let (msg_type, size) = if self.use_mini_header {
            let mut buf = [0u8; SpiceMiniDataHeader::SIZE];
            reader_guard
                .read_exact(&mut buf)
                .await
                .map_err(|e| ProtocolError::ReceiveFailed(e.to_string()))?;
            let header = SpiceMiniDataHeader::from_bytes(&buf)
                .ok_or_else(|| ProtocolError::ParseError("无效的消息头".to_string()))?;
            (header.msg_type, header.size)
        } else {
            let mut buf = [0u8; SpiceDataHeader::SIZE];
            reader_guard
                .read_exact(&mut buf)
                .await
                .map_err(|e| ProtocolError::ReceiveFailed(e.to_string()))?;
            let header = SpiceDataHeader::from_bytes(&buf)
                .ok_or_else(|| ProtocolError::ParseError("无效的消息头".to_string()))?;
            (header.msg_type, header.size)
        };

        let mut data = vec![0u8; size as usize];
        if size > 0 {
            reader_guard
                .read_exact(&mut data)
                .await
                .map_err(|e| ProtocolError::ReceiveFailed(e.to_string()))?;
        }

        trace!("接收消息: type={}, size={}", msg_type, size);
        Ok((msg_type, data))
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        self.writer = None;
        self.reader = None;
        self.connected = false;
        debug!("通道 {:?} 已断开", self.channel_type);
        Ok(())
    }

    /// 是否已连接
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// 获取通道类型
    pub fn channel_type(&self) -> ChannelType {
        self.channel_type
    }

    /// 获取通道 ID
    pub fn channel_id(&self) -> u8 {
        self.channel_id
    }

    /// 设置使用迷你头部
    pub fn set_use_mini_header(&mut self, use_mini: bool) {
        self.use_mini_header = use_mini;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type_conversion() {
        assert_eq!(ChannelType::Main.to_u8(), 1);
        assert_eq!(ChannelType::Display.to_u8(), 2);
        assert_eq!(ChannelType::Inputs.to_u8(), 3);

        assert_eq!(ChannelType::from_u8(1), Some(ChannelType::Main));
        assert_eq!(ChannelType::from_u8(2), Some(ChannelType::Display));
        assert_eq!(ChannelType::from_u8(255), None);
    }

    #[test]
    fn test_channel_connection_new() {
        let conn = ChannelConnection::new(ChannelType::Inputs, 0);
        assert_eq!(conn.channel_type(), ChannelType::Inputs);
        assert_eq!(conn.channel_id(), 0);
        assert!(!conn.is_connected());
    }
}

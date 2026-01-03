//! SPICE 协议类型定义
//!
//! 定义 SPICE 协议中使用的各种数据类型和结构。

use serde::{Deserialize, Serialize};

/// SPICE 链接头部 (RedLinkHeader)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SpiceLinkHeader {
    /// 魔数 (REDQ)
    pub magic: u32,
    /// 主版本号
    pub major_version: u32,
    /// 次版本号
    pub minor_version: u32,
    /// 消息大小
    pub size: u32,
}

impl SpiceLinkHeader {
    pub const MAGIC: u32 = 0x51444552; // "REDQ" in little-endian
    pub const MAJOR_VERSION: u32 = 2;
    pub const MINOR_VERSION: u32 = 2;

    pub fn new(size: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            major_version: Self::MAJOR_VERSION,
            minor_version: Self::MINOR_VERSION,
            size,
        }
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.magic.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.major_version.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.minor_version.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.size.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }
        Some(Self {
            magic: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            major_version: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            minor_version: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            size: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        })
    }

    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

/// SPICE 链接消息 (RedLinkMess)
#[derive(Debug, Clone)]
pub struct SpiceLinkMessage {
    /// 连接ID (用于重连)
    pub connection_id: u32,
    /// 通道类型
    pub channel_type: u8,
    /// 通道ID
    pub channel_id: u8,
    /// 公共能力数量
    pub num_common_caps: u32,
    /// 通道能力数量
    pub num_channel_caps: u32,
    /// 能力偏移
    pub caps_offset: u32,
    /// 公共能力
    pub common_caps: Vec<u32>,
    /// 通道能力
    pub channel_caps: Vec<u32>,
}

impl SpiceLinkMessage {
    pub fn new(channel_type: u8, channel_id: u8) -> Self {
        Self {
            connection_id: 0,
            channel_type,
            channel_id,
            num_common_caps: 0,
            num_channel_caps: 0,
            caps_offset: 18, // 头部大小
            common_caps: Vec::new(),
            channel_caps: Vec::new(),
        }
    }

    pub fn with_connection_id(mut self, id: u32) -> Self {
        self.connection_id = id;
        self
    }

    pub fn with_common_caps(mut self, caps: Vec<u32>) -> Self {
        self.num_common_caps = caps.len() as u32;
        self.common_caps = caps;
        self
    }

    pub fn with_channel_caps(mut self, caps: Vec<u32>) -> Self {
        self.num_channel_caps = caps.len() as u32;
        self.channel_caps = caps;
        self
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.connection_id.to_le_bytes());
        bytes.push(self.channel_type);
        bytes.push(self.channel_id);
        bytes.extend_from_slice(&self.num_common_caps.to_le_bytes());
        bytes.extend_from_slice(&self.num_channel_caps.to_le_bytes());
        bytes.extend_from_slice(&self.caps_offset.to_le_bytes());

        // 写入能力
        for cap in &self.common_caps {
            bytes.extend_from_slice(&cap.to_le_bytes());
        }
        for cap in &self.channel_caps {
            bytes.extend_from_slice(&cap.to_le_bytes());
        }

        bytes
    }
}

/// SPICE 链接回复 (RedLinkReply)
#[derive(Debug, Clone)]
pub struct SpiceLinkReply {
    /// 错误码
    pub error: u32,
    /// RSA 公钥长度
    pub pub_key_len: u32,
    /// RSA 公钥数据
    pub pub_key: Vec<u8>,
    /// 公共能力数量
    pub num_common_caps: u32,
    /// 通道能力数量
    pub num_channel_caps: u32,
    /// 能力偏移
    pub caps_offset: u32,
    /// 公共能力
    pub common_caps: Vec<u32>,
    /// 通道能力
    pub channel_caps: Vec<u32>,
}

impl SpiceLinkReply {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 20 {
            return None;
        }

        let error = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let pub_key_len = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        let pub_key_end = 8 + pub_key_len as usize;
        if bytes.len() < pub_key_end + 12 {
            return None;
        }

        let pub_key = bytes[8..pub_key_end].to_vec();

        let offset = pub_key_end;
        let num_common_caps = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        let num_channel_caps = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        let caps_offset = u32::from_le_bytes([
            bytes[offset + 8],
            bytes[offset + 9],
            bytes[offset + 10],
            bytes[offset + 11],
        ]);

        // TODO: 解析能力列表

        Some(Self {
            error,
            pub_key_len,
            pub_key,
            num_common_caps,
            num_channel_caps,
            caps_offset,
            common_caps: Vec::new(),
            channel_caps: Vec::new(),
        })
    }

    pub fn is_ok(&self) -> bool {
        self.error == 0
    }
}

/// SPICE 数据头部 (RedDataHeader)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SpiceDataHeader {
    /// 序列号
    pub serial: u64,
    /// 消息类型
    pub msg_type: u16,
    /// 消息大小
    pub size: u32,
    /// 子消息列表偏移
    pub sub_list: u32,
}

impl SpiceDataHeader {
    pub const SIZE: usize = 18;

    pub fn new(msg_type: u16, size: u32) -> Self {
        Self {
            serial: 0,
            msg_type,
            size,
            sub_list: 0,
        }
    }

    pub fn with_serial(mut self, serial: u64) -> Self {
        self.serial = serial;
        self
    }

    pub fn to_bytes(&self) -> [u8; 18] {
        let mut bytes = [0u8; 18];
        bytes[0..8].copy_from_slice(&self.serial.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.msg_type.to_le_bytes());
        bytes[10..14].copy_from_slice(&self.size.to_le_bytes());
        bytes[14..18].copy_from_slice(&self.sub_list.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            serial: u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
            msg_type: u16::from_le_bytes([bytes[8], bytes[9]]),
            size: u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]),
            sub_list: u32::from_le_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]),
        })
    }
}

/// SPICE 迷你数据头部 (用于某些通道)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SpiceMiniDataHeader {
    /// 消息类型
    pub msg_type: u16,
    /// 消息大小
    pub size: u32,
}

impl SpiceMiniDataHeader {
    pub const SIZE: usize = 6;

    pub fn new(msg_type: u16, size: u32) -> Self {
        Self { msg_type, size }
    }

    pub fn to_bytes(&self) -> [u8; 6] {
        let mut bytes = [0u8; 6];
        bytes[0..2].copy_from_slice(&self.msg_type.to_le_bytes());
        bytes[2..6].copy_from_slice(&self.size.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            msg_type: u16::from_le_bytes([bytes[0], bytes[1]]),
            size: u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        })
    }
}

/// 键盘修饰键状态
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct KeyModifiersState {
    /// Scroll Lock
    pub scroll_lock: bool,
    /// Num Lock
    pub num_lock: bool,
    /// Caps Lock
    pub caps_lock: bool,
}

impl KeyModifiersState {
    pub fn to_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.scroll_lock {
            flags |= 1;
        }
        if self.num_lock {
            flags |= 2;
        }
        if self.caps_lock {
            flags |= 4;
        }
        flags
    }

    pub fn from_flags(flags: u32) -> Self {
        Self {
            scroll_lock: (flags & 1) != 0,
            num_lock: (flags & 2) != 0,
            caps_lock: (flags & 4) != 0,
        }
    }
}

/// 鼠标按钮状态掩码
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseButtonsMask {
    pub left: bool,
    pub middle: bool,
    pub right: bool,
    pub scroll_up: bool,
    pub scroll_down: bool,
    pub side: bool,
    pub extra: bool,
}

impl MouseButtonsMask {
    pub fn to_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.left {
            flags |= 1;
        }
        if self.middle {
            flags |= 2;
        }
        if self.right {
            flags |= 4;
        }
        if self.scroll_up {
            flags |= 8;
        }
        if self.scroll_down {
            flags |= 16;
        }
        if self.side {
            flags |= 32;
        }
        if self.extra {
            flags |= 64;
        }
        flags
    }

    pub fn from_flags(flags: u32) -> Self {
        Self {
            left: (flags & 1) != 0,
            middle: (flags & 2) != 0,
            right: (flags & 4) != 0,
            scroll_up: (flags & 8) != 0,
            scroll_down: (flags & 16) != 0,
            side: (flags & 32) != 0,
            extra: (flags & 64) != 0,
        }
    }
}

/// 分辨率信息
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// 显示器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// 显示器ID
    pub id: u8,
    /// 分辨率
    pub resolution: Resolution,
    /// X 偏移
    pub x: i32,
    /// Y 偏移
    pub y: i32,
    /// 是否启用
    pub enabled: bool,
}

/// SPICE 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceSessionInfo {
    /// 连接ID
    pub connection_id: u32,
    /// 服务器版本
    pub server_version: String,
    /// 已连接的通道
    pub channels: Vec<ChannelInfo>,
    /// 显示器列表
    pub monitors: Vec<MonitorInfo>,
    /// 当前鼠标模式
    pub mouse_mode: u32,
}

/// 通道信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    /// 通道类型
    pub channel_type: u8,
    /// 通道ID
    pub channel_id: u8,
    /// 是否已连接
    pub connected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_header_serialization() {
        let header = SpiceLinkHeader::new(100);
        let bytes = header.to_bytes();
        let parsed = SpiceLinkHeader::from_bytes(&bytes).unwrap();

        // 使用局部变量避免 packed struct 字段引用对齐问题
        let magic = parsed.magic;
        let major_version = parsed.major_version;
        let minor_version = parsed.minor_version;
        let size = parsed.size;

        assert_eq!(magic, SpiceLinkHeader::MAGIC);
        assert_eq!(major_version, 2);
        assert_eq!(minor_version, 2);
        assert_eq!(size, 100);
    }

    #[test]
    fn test_data_header_serialization() {
        let header = SpiceDataHeader::new(1, 256).with_serial(12345);
        let bytes = header.to_bytes();
        let parsed = SpiceDataHeader::from_bytes(&bytes).unwrap();

        // 使用局部变量避免 packed struct 字段引用对齐问题
        let serial = parsed.serial;
        let msg_type = parsed.msg_type;
        let size = parsed.size;

        assert_eq!(serial, 12345);
        assert_eq!(msg_type, 1);
        assert_eq!(size, 256);
    }

    #[test]
    fn test_key_modifiers() {
        let mods = KeyModifiersState {
            scroll_lock: false,
            num_lock: true,
            caps_lock: true,
        };
        let flags = mods.to_flags();
        let parsed = KeyModifiersState::from_flags(flags);

        assert_eq!(parsed.scroll_lock, false);
        assert_eq!(parsed.num_lock, true);
        assert_eq!(parsed.caps_lock, true);
    }
}

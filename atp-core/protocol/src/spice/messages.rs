//! SPICE 协议消息定义
//!
//! 定义 SPICE 协议中各通道使用的消息结构。

use super::constants::*;
use super::types::*;

/// Main 通道初始化消息 (服务器 -> 客户端)
#[derive(Debug, Clone)]
pub struct MsgMainInit {
    /// 会话ID
    pub session_id: u32,
    /// 显示通道数量
    pub display_channels_hint: u32,
    /// 支持的鼠标模式
    pub supported_mouse_modes: u32,
    /// 当前鼠标模式
    pub current_mouse_mode: u32,
    /// Agent 是否已连接
    pub agent_connected: u32,
    /// Agent token 数量
    pub agent_tokens: u32,
    /// 多媒体时间
    pub multi_media_time: u32,
    /// RAM 提示
    pub ram_hint: u32,
}

impl MsgMainInit {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 32 {
            return None;
        }

        Some(Self {
            session_id: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            display_channels_hint: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            supported_mouse_modes: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            current_mouse_mode: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            agent_connected: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            agent_tokens: u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            multi_media_time: u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            ram_hint: u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
        })
    }
}

/// 通道列表消息
#[derive(Debug, Clone)]
pub struct MsgMainChannelsList {
    pub channels: Vec<ChannelInfo>,
}

impl MsgMainChannelsList {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        let num_channels = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let mut channels = Vec::with_capacity(num_channels);

        let mut offset = 4;
        for _ in 0..num_channels {
            if offset + 2 > bytes.len() {
                break;
            }
            channels.push(ChannelInfo {
                channel_type: bytes[offset],
                channel_id: bytes[offset + 1],
                connected: false,
            });
            offset += 2;
        }

        Some(Self { channels })
    }
}

/// 鼠标模式消息
#[derive(Debug, Clone, Copy)]
pub struct MsgMainMouseMode {
    /// 支持的模式
    pub supported_modes: u32,
    /// 当前模式
    pub current_mode: u32,
}

impl MsgMainMouseMode {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None;
        }

        Some(Self {
            supported_modes: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            current_mode: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        })
    }
}

/// 鼠标模式请求消息 (客户端 -> 服务器)
#[derive(Debug, Clone, Copy)]
pub struct MsgcMainMouseModeRequest {
    pub mode: u32,
}

impl MsgcMainMouseModeRequest {
    pub fn new(mode: u32) -> Self {
        Self { mode }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.mode.to_le_bytes()
    }
}

/// Inputs 通道初始化消息 (服务器 -> 客户端)
#[derive(Debug, Clone, Copy)]
pub struct MsgInputsInit {
    /// 键盘修饰键状态
    pub keyboard_modifiers: u32,
}

impl MsgInputsInit {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        Some(Self {
            keyboard_modifiers: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
    }
}

/// 键盘按键消息 (客户端 -> 服务器)
#[derive(Debug, Clone, Copy)]
pub struct MsgcInputsKeyDown {
    /// PC AT 扫描码
    pub code: u32,
}

impl MsgcInputsKeyDown {
    pub fn new(code: u32) -> Self {
        Self { code }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.code.to_le_bytes()
    }
}

/// 键盘释放消息 (客户端 -> 服务器)
#[derive(Debug, Clone, Copy)]
pub struct MsgcInputsKeyUp {
    /// PC AT 扫描码
    pub code: u32,
}

impl MsgcInputsKeyUp {
    pub fn new(code: u32) -> Self {
        Self { code }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.code.to_le_bytes()
    }
}

/// 键盘修饰键消息
#[derive(Debug, Clone, Copy)]
pub struct MsgInputsKeyModifiers {
    pub modifiers: u32,
}

impl MsgInputsKeyModifiers {
    pub fn new(modifiers: u32) -> Self {
        Self { modifiers }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        Some(Self {
            modifiers: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.modifiers.to_le_bytes()
    }
}

/// 鼠标运动消息 (服务器模式: 相对坐标)
#[derive(Debug, Clone, Copy)]
pub struct MsgcInputsMouseMotion {
    /// X 方向移动量
    pub dx: i32,
    /// Y 方向移动量
    pub dy: i32,
    /// 按钮状态掩码
    pub buttons_state: u32,
}

impl MsgcInputsMouseMotion {
    pub fn new(dx: i32, dy: i32, buttons_state: u32) -> Self {
        Self {
            dx,
            dy,
            buttons_state,
        }
    }

    pub fn to_bytes(&self) -> [u8; 12] {
        let mut bytes = [0u8; 12];
        bytes[0..4].copy_from_slice(&self.dx.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.dy.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.buttons_state.to_le_bytes());
        bytes
    }
}

/// 鼠标位置消息 (客户端模式: 绝对坐标)
#[derive(Debug, Clone, Copy)]
pub struct MsgcInputsMousePosition {
    /// X 坐标
    pub x: u32,
    /// Y 坐标
    pub y: u32,
    /// 按钮状态掩码
    pub buttons_state: u32,
    /// 显示器 ID
    pub display_id: u8,
}

impl MsgcInputsMousePosition {
    pub fn new(x: u32, y: u32, buttons_state: u32, display_id: u8) -> Self {
        Self {
            x,
            y,
            buttons_state,
            display_id,
        }
    }

    pub fn to_bytes(&self) -> [u8; 13] {
        let mut bytes = [0u8; 13];
        bytes[0..4].copy_from_slice(&self.x.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.y.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.buttons_state.to_le_bytes());
        bytes[12] = self.display_id;
        bytes
    }
}

/// 鼠标按下消息
#[derive(Debug, Clone, Copy)]
pub struct MsgcInputsMousePress {
    /// 按钮 ID
    pub button: u8,
    /// 按钮状态掩码
    pub buttons_state: u32,
}

impl MsgcInputsMousePress {
    pub fn new(button: u8, buttons_state: u32) -> Self {
        Self {
            button,
            buttons_state,
        }
    }

    pub fn to_bytes(&self) -> [u8; 5] {
        let mut bytes = [0u8; 5];
        bytes[0] = self.button;
        bytes[1..5].copy_from_slice(&self.buttons_state.to_le_bytes());
        bytes
    }
}

/// 鼠标释放消息
#[derive(Debug, Clone, Copy)]
pub struct MsgcInputsMouseRelease {
    /// 按钮 ID
    pub button: u8,
    /// 按钮状态掩码
    pub buttons_state: u32,
}

impl MsgcInputsMouseRelease {
    pub fn new(button: u8, buttons_state: u32) -> Self {
        Self {
            button,
            buttons_state,
        }
    }

    pub fn to_bytes(&self) -> [u8; 5] {
        let mut bytes = [0u8; 5];
        bytes[0] = self.button;
        bytes[1..5].copy_from_slice(&self.buttons_state.to_le_bytes());
        bytes
    }
}

/// Display 通道模式消息
#[derive(Debug, Clone, Copy)]
pub struct MsgDisplayMode {
    /// X 分辨率
    pub x_res: u32,
    /// Y 分辨率
    pub y_res: u32,
    /// 色深
    pub bits: u32,
}

impl MsgDisplayMode {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 {
            return None;
        }

        Some(Self {
            x_res: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            y_res: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            bits: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        })
    }
}

/// 显示器配置消息
#[derive(Debug, Clone)]
pub struct MsgDisplayMonitorsConfig {
    /// 数量
    pub count: u16,
    /// 最大允许
    pub max_allowed: u16,
    /// 监视器列表
    pub monitors: Vec<MonitorConfig>,
}

#[derive(Debug, Clone, Copy)]
pub struct MonitorConfig {
    pub id: u32,
    pub surface_id: u32,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub flags: u32,
}

impl MsgDisplayMonitorsConfig {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        let count = u16::from_le_bytes([bytes[0], bytes[1]]);
        let max_allowed = u16::from_le_bytes([bytes[2], bytes[3]]);

        let mut monitors = Vec::with_capacity(count as usize);
        let mut offset = 4;

        for _ in 0..count {
            if offset + 28 > bytes.len() {
                break;
            }
            monitors.push(MonitorConfig {
                id: u32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]),
                surface_id: u32::from_le_bytes([
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ]),
                width: u32::from_le_bytes([
                    bytes[offset + 8],
                    bytes[offset + 9],
                    bytes[offset + 10],
                    bytes[offset + 11],
                ]),
                height: u32::from_le_bytes([
                    bytes[offset + 12],
                    bytes[offset + 13],
                    bytes[offset + 14],
                    bytes[offset + 15],
                ]),
                x: i32::from_le_bytes([
                    bytes[offset + 16],
                    bytes[offset + 17],
                    bytes[offset + 18],
                    bytes[offset + 19],
                ]),
                y: i32::from_le_bytes([
                    bytes[offset + 20],
                    bytes[offset + 21],
                    bytes[offset + 22],
                    bytes[offset + 23],
                ]),
                flags: u32::from_le_bytes([
                    bytes[offset + 24],
                    bytes[offset + 25],
                    bytes[offset + 26],
                    bytes[offset + 27],
                ]),
            });
            offset += 28;
        }

        Some(Self {
            count,
            max_allowed,
            monitors,
        })
    }
}

/// Surface 创建消息
#[derive(Debug, Clone, Copy)]
pub struct MsgDisplaySurfaceCreate {
    /// Surface ID
    pub surface_id: u32,
    /// 宽度
    pub width: u32,
    /// 高度
    pub height: u32,
    /// 格式
    pub format: u32,
    /// 标志
    pub flags: u32,
}

impl MsgDisplaySurfaceCreate {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 20 {
            return None;
        }

        Some(Self {
            surface_id: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            width: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            height: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            format: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            flags: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
        })
    }
}

/// 视频流创建消息
#[derive(Debug, Clone)]
pub struct MsgDisplayStreamCreate {
    /// 流 ID
    pub id: u32,
    /// Surface ID
    pub surface_id: u32,
    /// 标志
    pub flags: u8,
    /// 编解码器类型
    pub codec_type: u8,
    /// 时间戳
    pub stamp: u64,
    /// 流宽度
    pub stream_width: u32,
    /// 流高度
    pub stream_height: u32,
    /// 源宽度
    pub src_width: u32,
    /// 源高度
    pub src_height: u32,
    /// 目标位置
    pub dest: Rect,
    /// 裁剪区域
    pub clip: Clip,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}

#[derive(Debug, Clone)]
pub struct Clip {
    pub clip_type: u8,
    pub rects: Vec<Rect>,
}

/// Display 初始化消息 (客户端 -> 服务器)
#[derive(Debug, Clone, Copy)]
pub struct MsgcDisplayInit {
    /// pixmap 缓存 ID
    pub pixmap_cache_id: u8,
    /// pixmap 缓存大小
    pub pixmap_cache_size: u64,
    /// GLZ 字典窗口大小
    pub glz_dictionary_window_size: u32,
}

impl MsgcDisplayInit {
    pub fn new() -> Self {
        Self {
            pixmap_cache_id: 0,
            pixmap_cache_size: 0,
            glz_dictionary_window_size: 0,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(13);
        bytes.push(self.pixmap_cache_id);
        bytes.extend_from_slice(&self.pixmap_cache_size.to_le_bytes());
        bytes.extend_from_slice(&self.glz_dictionary_window_size.to_le_bytes());
        bytes
    }
}

impl Default for MsgcDisplayInit {
    fn default() -> Self {
        Self::new()
    }
}

/// USB 重定向数据消息
#[derive(Debug, Clone)]
pub struct MsgUsbredirData {
    pub data: Vec<u8>,
}

impl MsgUsbredirData {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            data: bytes.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_down_message() {
        let msg = MsgcInputsKeyDown::new(0x1E); // 'A' key
        let bytes = msg.to_bytes();
        assert_eq!(bytes.len(), 4);
        assert_eq!(u32::from_le_bytes(bytes), 0x1E);
    }

    #[test]
    fn test_mouse_position_message() {
        let msg = MsgcInputsMousePosition::new(100, 200, 0, 0);
        let bytes = msg.to_bytes();
        assert_eq!(bytes.len(), 13);
    }
}

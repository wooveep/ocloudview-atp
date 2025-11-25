//! SPICE 协议常量定义
//!
//! 包含 SPICE 协议中使用的所有常量值。

/// SPICE 通道类型
pub mod channel_type {
    pub const MAIN: u8 = 1;
    pub const DISPLAY: u8 = 2;
    pub const INPUTS: u8 = 3;
    pub const CURSOR: u8 = 4;
    pub const PLAYBACK: u8 = 5;
    pub const RECORD: u8 = 6;
    pub const TUNNEL: u8 = 7; // 已废弃
    pub const SMARTCARD: u8 = 8;
    pub const USBREDIR: u8 = 9;
    pub const PORT: u8 = 10;
    pub const WEBDAV: u8 = 11;
}

/// 链接错误码
pub mod link_error {
    pub const OK: u32 = 0;
    pub const ERROR: u32 = 1;
    pub const INVALID_MAGIC: u32 = 2;
    pub const INVALID_DATA: u32 = 3;
    pub const VERSION_MISMATCH: u32 = 4;
    pub const NEED_SECURED: u32 = 5;
    pub const NEED_UNSECURED: u32 = 6;
    pub const PERMISSION_DENIED: u32 = 7;
    pub const BAD_CONNECTION_ID: u32 = 8;
    pub const CHANNEL_NOT_AVAILABLE: u32 = 9;
}

/// Main 通道消息类型 (服务器 -> 客户端)
pub mod msg_main {
    pub const MIGRATE_BEGIN: u16 = 101;
    pub const MIGRATE_CANCEL: u16 = 102;
    pub const INIT: u16 = 103;
    pub const CHANNELS_LIST: u16 = 104;
    pub const MOUSE_MODE: u16 = 105;
    pub const MULTI_MEDIA_TIME: u16 = 106;
    pub const AGENT_CONNECTED: u16 = 107;
    pub const AGENT_DISCONNECTED: u16 = 108;
    pub const AGENT_DATA: u16 = 109;
    pub const AGENT_TOKEN: u16 = 110;
    pub const MIGRATE_SWITCH_HOST: u16 = 111;
    pub const MIGRATE_END: u16 = 112;
    pub const NAME: u16 = 113;
    pub const UUID: u16 = 114;
    pub const MIGRATE_BEGIN_SEAMLESS: u16 = 115;
    pub const MIGRATE_DST_SEAMLESS_ACK: u16 = 116;
    pub const MIGRATE_DST_SEAMLESS_NACK: u16 = 117;
    pub const PING: u16 = 118;
}

/// Main 通道消息类型 (客户端 -> 服务器)
pub mod msgc_main {
    pub const CLIENT_INFO: u16 = 101;
    pub const MIGRATE_CONNECTED: u16 = 102;
    pub const MIGRATE_CONNECT_ERROR: u16 = 103;
    pub const ATTACH_CHANNELS: u16 = 104;
    pub const MOUSE_MODE_REQUEST: u16 = 105;
    pub const AGENT_START: u16 = 106;
    pub const AGENT_DATA: u16 = 107;
    pub const AGENT_TOKEN: u16 = 108;
    pub const MIGRATE_END: u16 = 109;
    pub const MIGRATE_DST_DO_SEAMLESS: u16 = 110;
    pub const MIGRATE_SEAMLESS_DST_ACK: u16 = 111;
    pub const MIGRATE_SEAMLESS_DST_NACK: u16 = 112;
    pub const PONG: u16 = 113;
}

/// Inputs 通道消息类型 (服务器 -> 客户端)
pub mod msg_inputs {
    pub const INIT: u16 = 101;
    pub const KEY_MODIFIERS: u16 = 102;
    pub const MOUSE_MOTION_ACK: u16 = 111;
}

/// Inputs 通道消息类型 (客户端 -> 服务器)
pub mod msgc_inputs {
    pub const KEY_DOWN: u16 = 101;
    pub const KEY_UP: u16 = 102;
    pub const KEY_MODIFIERS: u16 = 103;
    pub const MOUSE_MOTION: u16 = 111;
    pub const MOUSE_POSITION: u16 = 112;
    pub const MOUSE_PRESS: u16 = 113;
    pub const MOUSE_RELEASE: u16 = 114;
}

/// Display 通道消息类型 (服务器 -> 客户端)
pub mod msg_display {
    pub const MODE: u16 = 101;
    pub const MARK: u16 = 102;
    pub const RESET: u16 = 103;
    pub const COPY_BITS: u16 = 104;
    pub const INVAL_LIST: u16 = 105;
    pub const INVAL_ALL_PIXMAPS: u16 = 106;
    pub const INVAL_PALETTE: u16 = 107;
    pub const INVAL_ALL_PALETTES: u16 = 108;
    pub const STREAM_CREATE: u16 = 122;
    pub const STREAM_DATA: u16 = 123;
    pub const STREAM_CLIP: u16 = 124;
    pub const STREAM_DESTROY: u16 = 125;
    pub const STREAM_DESTROY_ALL: u16 = 126;
    pub const DRAW_FILL: u16 = 302;
    pub const DRAW_OPAQUE: u16 = 303;
    pub const DRAW_COPY: u16 = 304;
    pub const DRAW_BLEND: u16 = 305;
    pub const DRAW_BLACKNESS: u16 = 306;
    pub const DRAW_WHITENESS: u16 = 307;
    pub const DRAW_INVERS: u16 = 308;
    pub const DRAW_ROP3: u16 = 309;
    pub const DRAW_STROKE: u16 = 310;
    pub const DRAW_TEXT: u16 = 311;
    pub const DRAW_TRANSPARENT: u16 = 312;
    pub const DRAW_ALPHA_BLEND: u16 = 313;
    pub const DRAW_COMPOSITE: u16 = 314;
    pub const SURFACE_CREATE: u16 = 315;
    pub const SURFACE_DESTROY: u16 = 316;
    pub const MONITORS_CONFIG: u16 = 317;
    pub const GL_SCANOUT_UNIX: u16 = 318;
    pub const GL_DRAW: u16 = 319;
}

/// Display 通道消息类型 (客户端 -> 服务器)
pub mod msgc_display {
    pub const INIT: u16 = 101;
    pub const STREAM_REPORT: u16 = 102;
    pub const PREFERRED_COMPRESSION: u16 = 103;
    pub const GL_DRAW_DONE: u16 = 104;
    pub const PREFERRED_VIDEO_CODEC_TYPE: u16 = 105;
}

/// Cursor 通道消息类型
pub mod msg_cursor {
    pub const INIT: u16 = 101;
    pub const RESET: u16 = 102;
    pub const SET: u16 = 103;
    pub const MOVE: u16 = 104;
    pub const HIDE: u16 = 105;
    pub const TRAIL: u16 = 106;
    pub const INVAL_ONE: u16 = 107;
    pub const INVAL_ALL: u16 = 108;
}

/// USB 重定向消息类型
pub mod msg_usbredir {
    pub const DATA: u16 = 101;
}

pub mod msgc_usbredir {
    pub const DATA: u16 = 101;
}

/// 鼠标模式
pub mod mouse_mode {
    /// 服务器模式 (相对移动)
    pub const SERVER: u32 = 1;
    /// 客户端模式 (绝对位置)
    pub const CLIENT: u32 = 2;
}

/// 鼠标按钮 ID
pub mod mouse_button {
    pub const LEFT: u8 = 1;
    pub const MIDDLE: u8 = 2;
    pub const RIGHT: u8 = 3;
    pub const SCROLL_UP: u8 = 4;
    pub const SCROLL_DOWN: u8 = 5;
    pub const SIDE: u8 = 6;
    pub const EXTRA: u8 = 7;
}

/// 鼠标按钮掩码
pub mod mouse_button_mask {
    pub const LEFT: u32 = 1 << 0;
    pub const MIDDLE: u32 = 1 << 1;
    pub const RIGHT: u32 = 1 << 2;
    pub const SCROLL_UP: u32 = 1 << 3;
    pub const SCROLL_DOWN: u32 = 1 << 4;
    pub const SIDE: u32 = 1 << 5;
    pub const EXTRA: u32 = 1 << 6;
}

/// 键盘修饰键
pub mod key_modifier {
    pub const SCROLL_LOCK: u32 = 1 << 0;
    pub const NUM_LOCK: u32 = 1 << 1;
    pub const CAPS_LOCK: u32 = 1 << 2;
}

/// 公共能力
pub mod common_caps {
    pub const AUTH_SELECT: u32 = 1 << 0;
    pub const AUTH_SPICE: u32 = 1 << 1;
    pub const AUTH_SASL: u32 = 1 << 2;
    pub const MINI_HEADER: u32 = 1 << 3;
}

/// Inputs 通道能力
pub mod inputs_caps {
    pub const KEY_SCANCODE: u32 = 1 << 0;
}

/// Display 通道能力
pub mod display_caps {
    pub const SIZED_STREAM: u32 = 1 << 0;
    pub const MONITORS_CONFIG: u32 = 1 << 1;
    pub const COMPOSITE: u32 = 1 << 2;
    pub const A8_SURFACE: u32 = 1 << 3;
    pub const STREAM_REPORT: u32 = 1 << 4;
    pub const LZ4_COMPRESSION: u32 = 1 << 5;
    pub const PREF_COMPRESSION: u32 = 1 << 6;
    pub const GL_SCANOUT: u32 = 1 << 7;
    pub const MULTI_CODEC: u32 = 1 << 8;
    pub const CODEC_MJPEG: u32 = 1 << 9;
    pub const CODEC_VP8: u32 = 1 << 10;
    pub const CODEC_H264: u32 = 1 << 11;
    pub const CODEC_VP9: u32 = 1 << 12;
    pub const CODEC_H265: u32 = 1 << 13;
}

/// Main 通道能力
pub mod main_caps {
    pub const SEMI_SEAMLESS_MIGRATE: u32 = 1 << 0;
    pub const NAME_AND_UUID: u32 = 1 << 1;
    pub const AGENT_CONNECTED_TOKENS: u32 = 1 << 2;
    pub const SEAMLESS_MIGRATE: u32 = 1 << 3;
}

/// 图像压缩类型
pub mod image_compression {
    pub const INVALID: u8 = 0;
    pub const OFF: u8 = 1;
    pub const AUTO_GLZ: u8 = 2;
    pub const AUTO_LZ: u8 = 3;
    pub const QUIC: u8 = 4;
    pub const GLZ: u8 = 5;
    pub const LZ: u8 = 6;
    pub const LZ4: u8 = 7;
}

/// 视频编解码器类型
pub mod video_codec {
    pub const MJPEG: u8 = 1;
    pub const VP8: u8 = 2;
    pub const H264: u8 = 3;
    pub const VP9: u8 = 4;
    pub const H265: u8 = 5;
}

/// 默认端口
pub const DEFAULT_PORT: u16 = 5900;
pub const DEFAULT_TLS_PORT: u16 = 5901;

/// 消息运动确认阈值
pub const MOTION_ACK_BUNCH: u32 = 8;

/// RSA 公钥大小 (位)
pub const RSA_KEY_SIZE: usize = 1024;

/// 密码最大长度
pub const PASSWORD_MAX_LEN: usize = 60;

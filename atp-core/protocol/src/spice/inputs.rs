//! SPICE Inputs 通道
//!
//! 实现键盘和鼠标输入事件的发送。

use crate::{ProtocolError, Result};
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::{debug, trace};

use super::channel::{ChannelConnection, ChannelType};
use super::constants::*;
use super::messages::*;
use super::types::*;

/// 鼠标按钮
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    ScrollUp,
    ScrollDown,
    Side,
    Extra,
}

impl MouseButton {
    pub fn to_id(self) -> u8 {
        match self {
            Self::Left => mouse_button::LEFT,
            Self::Middle => mouse_button::MIDDLE,
            Self::Right => mouse_button::RIGHT,
            Self::ScrollUp => mouse_button::SCROLL_UP,
            Self::ScrollDown => mouse_button::SCROLL_DOWN,
            Self::Side => mouse_button::SIDE,
            Self::Extra => mouse_button::EXTRA,
        }
    }

    pub fn to_mask(self) -> u32 {
        match self {
            Self::Left => mouse_button_mask::LEFT,
            Self::Middle => mouse_button_mask::MIDDLE,
            Self::Right => mouse_button_mask::RIGHT,
            Self::ScrollUp => mouse_button_mask::SCROLL_UP,
            Self::ScrollDown => mouse_button_mask::SCROLL_DOWN,
            Self::Side => mouse_button_mask::SIDE,
            Self::Extra => mouse_button_mask::EXTRA,
        }
    }
}

/// 鼠标模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseMode {
    /// 服务器模式（相对坐标）
    Server,
    /// 客户端模式（绝对坐标）
    Client,
}

impl MouseMode {
    pub fn from_u32(value: u32) -> Self {
        if (value & mouse_mode::CLIENT) != 0 {
            Self::Client
        } else {
            Self::Server
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            Self::Server => mouse_mode::SERVER,
            Self::Client => mouse_mode::CLIENT,
        }
    }
}

/// 键盘修饰键
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyModifiers {
    pub scroll_lock: bool,
    pub num_lock: bool,
    pub caps_lock: bool,
}

impl KeyModifiers {
    pub fn to_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.scroll_lock {
            flags |= key_modifier::SCROLL_LOCK;
        }
        if self.num_lock {
            flags |= key_modifier::NUM_LOCK;
        }
        if self.caps_lock {
            flags |= key_modifier::CAPS_LOCK;
        }
        flags
    }

    pub fn from_flags(flags: u32) -> Self {
        Self {
            scroll_lock: (flags & key_modifier::SCROLL_LOCK) != 0,
            num_lock: (flags & key_modifier::NUM_LOCK) != 0,
            caps_lock: (flags & key_modifier::CAPS_LOCK) != 0,
        }
    }
}

/// SPICE Inputs 通道
///
/// 用于发送键盘和鼠标事件到虚拟机
pub struct InputsChannel {
    /// 底层通道连接 (使用 Mutex 实现内部可变性)
    connection: tokio::sync::Mutex<ChannelConnection>,
    /// 当前鼠标按钮状态
    buttons_state: AtomicU32,
    /// 键盘修饰键状态
    key_modifiers: KeyModifiers,
    /// 待确认的鼠标移动消息数
    motion_ack_pending: AtomicU32,
}

impl InputsChannel {
    /// 创建新的 Inputs 通道
    pub fn new(channel_id: u8) -> Self {
        Self {
            connection: tokio::sync::Mutex::new(ChannelConnection::new(
                ChannelType::Inputs,
                channel_id,
            )),
            buttons_state: AtomicU32::new(0),
            key_modifiers: KeyModifiers::default(),
            motion_ack_pending: AtomicU32::new(0),
        }
    }

    /// 连接到服务器
    pub async fn connect(
        &mut self,
        host: &str,
        port: u16,
        connection_id: u32,
        password: Option<&str>,
    ) -> Result<()> {
        let mut connection = self.connection.lock().await;
        connection
            .connect(host, port, connection_id, password)
            .await?;
        drop(connection);

        // 处理 Inputs 初始化消息
        self.handle_init().await?;

        debug!("Inputs 通道已连接");
        Ok(())
    }

    /// 处理初始化消息
    async fn handle_init(&mut self) -> Result<()> {
        let mut connection = self.connection.lock().await;
        let (msg_type, data) = connection.receive_message().await?;

        // SPICE_MSG_INPUTS_INIT = 101
        if msg_type == 101 {
            if let Some(init) = MsgInputsInit::from_bytes(&data) {
                self.key_modifiers = KeyModifiers::from_flags(init.keyboard_modifiers);
                debug!("Inputs 初始化: modifiers={:?}", self.key_modifiers);
            }
        }

        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        self.connection.lock().await.disconnect().await
    }

    /// 是否已连接
    pub async fn is_connected(&self) -> bool {
        self.connection.lock().await.is_connected()
    }

    // ========================================================================
    // 键盘操作
    // ========================================================================

    /// 发送键盘按下事件
    ///
    /// # Arguments
    /// * `scancode` - PC AT 扫描码
    pub async fn send_key_down(&self, scancode: u32) -> Result<()> {
        let msg = MsgcInputsKeyDown::new(scancode);
        let data = msg.to_bytes();

        let mut connection = self.connection.lock().await;
        if !connection.is_connected() {
            return Err(ProtocolError::ConnectionFailed(
                "Inputs 通道未连接".to_string(),
            ));
        }

        connection.send_message(101, &data).await?; // SPICE_MSGC_INPUTS_KEY_DOWN

        trace!("发送键盘按下: scancode=0x{:X}", scancode);
        Ok(())
    }

    /// 发送键盘释放事件
    pub async fn send_key_up(&self, scancode: u32) -> Result<()> {
        let msg = MsgcInputsKeyUp::new(scancode);
        let data = msg.to_bytes();

        let mut connection = self.connection.lock().await;
        if !connection.is_connected() {
            return Err(ProtocolError::ConnectionFailed(
                "Inputs 通道未连接".to_string(),
            ));
        }

        connection.send_message(102, &data).await?; // SPICE_MSGC_INPUTS_KEY_UP

        trace!("发送键盘释放: scancode=0x{:X}", scancode);
        Ok(())
    }

    /// 发送完整的按键操作（按下 + 释放）
    pub async fn send_key_press(&self, scancode: u32) -> Result<()> {
        self.send_key_down(scancode).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        self.send_key_up(scancode).await?;
        Ok(())
    }

    /// 发送键盘修饰键状态
    pub async fn send_key_modifiers(&self, modifiers: KeyModifiers) -> Result<()> {
        let msg = MsgInputsKeyModifiers::new(modifiers.to_flags());
        let data = msg.to_bytes();

        let mut connection = self.connection.lock().await;
        if !connection.is_connected() {
            return Err(ProtocolError::ConnectionFailed(
                "Inputs 通道未连接".to_string(),
            ));
        }

        connection.send_message(103, &data).await?; // SPICE_MSGC_INPUTS_KEY_MODIFIERS

        trace!("发送键盘修饰键: {:?}", modifiers);
        Ok(())
    }

    /// 发送文本（转换为扫描码序列）
    pub async fn send_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            if let Some(scancode) = char_to_scancode(ch) {
                // 检查是否需要 Shift
                let needs_shift = ch.is_ascii_uppercase() || "!@#$%^&*()_+{}|:\"<>?~".contains(ch);

                if needs_shift {
                    self.send_key_down(0x2A).await?; // Left Shift
                }

                self.send_key_press(scancode).await?;

                if needs_shift {
                    self.send_key_up(0x2A).await?;
                }

                // 按键之间的延迟
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            }
        }
        Ok(())
    }

    // ========================================================================
    // 鼠标操作
    // ========================================================================

    /// 发送鼠标位置（客户端模式，绝对坐标）
    pub async fn send_mouse_position(&self, x: u32, y: u32, display_id: u8) -> Result<()> {
        let buttons = self.buttons_state.load(Ordering::Relaxed);

        let mut data = Vec::with_capacity(13);
        data.extend_from_slice(&x.to_le_bytes());
        data.extend_from_slice(&y.to_le_bytes());
        data.extend_from_slice(&buttons.to_le_bytes());
        data.push(display_id);

        let mut connection = self.connection.lock().await;
        if !connection.is_connected() {
            return Err(ProtocolError::ConnectionFailed(
                "Inputs 通道未连接".to_string(),
            ));
        }

        connection.send_message(112, &data).await?; // SPICE_MSGC_INPUTS_MOUSE_POSITION

        self.motion_ack_pending.fetch_add(1, Ordering::Relaxed);
        trace!("发送鼠标位置: ({}, {}) display={}", x, y, display_id);
        Ok(())
    }

    /// 发送鼠标相对移动（服务器模式）
    pub async fn send_mouse_motion(&self, dx: i32, dy: i32) -> Result<()> {
        let buttons = self.buttons_state.load(Ordering::Relaxed);

        let mut data = Vec::with_capacity(12);
        data.extend_from_slice(&dx.to_le_bytes());
        data.extend_from_slice(&dy.to_le_bytes());
        data.extend_from_slice(&buttons.to_le_bytes());

        let mut connection = self.connection.lock().await;
        if !connection.is_connected() {
            return Err(ProtocolError::ConnectionFailed(
                "Inputs 通道未连接".to_string(),
            ));
        }

        connection.send_message(111, &data).await?; // SPICE_MSGC_INPUTS_MOUSE_MOTION

        self.motion_ack_pending.fetch_add(1, Ordering::Relaxed);
        trace!("发送鼠标移动: ({}, {})", dx, dy);
        Ok(())
    }

    /// 发送鼠标按下事件
    pub async fn send_mouse_press(&self, button: MouseButton) -> Result<()> {
        // 更新按钮状态
        let mask = button.to_mask();
        let new_state = self.buttons_state.fetch_or(mask, Ordering::Relaxed) | mask;

        let mut data = Vec::with_capacity(5);
        data.push(button.to_id());
        data.extend_from_slice(&new_state.to_le_bytes());

        let mut connection = self.connection.lock().await;
        if !connection.is_connected() {
            return Err(ProtocolError::ConnectionFailed(
                "Inputs 通道未连接".to_string(),
            ));
        }

        connection.send_message(113, &data).await?; // SPICE_MSGC_INPUTS_MOUSE_PRESS

        trace!("发送鼠标按下: {:?}", button);
        Ok(())
    }

    /// 发送鼠标释放事件
    pub async fn send_mouse_release(&self, button: MouseButton) -> Result<()> {
        // 更新按钮状态
        let mask = button.to_mask();
        let new_state = self.buttons_state.fetch_and(!mask, Ordering::Relaxed) & !mask;

        let mut data = Vec::with_capacity(5);
        data.push(button.to_id());
        data.extend_from_slice(&new_state.to_le_bytes());

        let mut connection = self.connection.lock().await;
        if !connection.is_connected() {
            return Err(ProtocolError::ConnectionFailed(
                "Inputs 通道未连接".to_string(),
            ));
        }

        connection.send_message(114, &data).await?; // SPICE_MSGC_INPUTS_MOUSE_RELEASE

        trace!("发送鼠标释放: {:?}", button);
        Ok(())
    }

    /// 发送鼠标点击（按下 + 释放）
    pub async fn send_mouse_click(&self, button: MouseButton) -> Result<()> {
        self.send_mouse_press(button).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        self.send_mouse_release(button).await?;
        Ok(())
    }

    /// 发送鼠标双击
    pub async fn send_mouse_double_click(&self, button: MouseButton) -> Result<()> {
        self.send_mouse_click(button).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.send_mouse_click(button).await?;
        Ok(())
    }

    /// 发送鼠标滚轮事件
    pub async fn send_mouse_scroll(&self, up: bool, count: u32) -> Result<()> {
        let button = if up {
            MouseButton::ScrollUp
        } else {
            MouseButton::ScrollDown
        };

        for _ in 0..count {
            self.send_mouse_press(button).await?;
            self.send_mouse_release(button).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        }

        Ok(())
    }

    // ========================================================================
    // 辅助方法
    // ========================================================================

    /// 获取当前按钮状态
    pub fn buttons_state(&self) -> u32 {
        self.buttons_state.load(Ordering::Relaxed)
    }

    /// 获取键盘修饰键状态
    pub fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }

    /// 处理服务器消息
    pub async fn process_events(&mut self) -> Result<()> {
        let mut connection = self.connection.lock().await;
        let (msg_type, data) = connection.receive_message().await?;

        match msg_type {
            // SPICE_MSG_INPUTS_KEY_MODIFIERS
            102 => {
                if let Some(mods) = MsgInputsKeyModifiers::from_bytes(&data) {
                    self.key_modifiers = KeyModifiers::from_flags(mods.modifiers);
                    debug!("键盘修饰键更新: {:?}", self.key_modifiers);
                }
            }
            // SPICE_MSG_INPUTS_MOUSE_MOTION_ACK
            111 => {
                self.motion_ack_pending.fetch_sub(
                    MOTION_ACK_BUNCH.min(self.motion_ack_pending.load(Ordering::Relaxed)),
                    Ordering::Relaxed,
                );
                trace!("收到鼠标移动确认");
            }
            _ => {
                debug!("Inputs 通道消息: type={}", msg_type);
            }
        }

        Ok(())
    }
}

/// 字符到扫描码的映射
fn char_to_scancode(ch: char) -> Option<u32> {
    // PC AT 扫描码集 1
    let scancode = match ch.to_ascii_lowercase() {
        'a' => 0x1E,
        'b' => 0x30,
        'c' => 0x2E,
        'd' => 0x20,
        'e' => 0x12,
        'f' => 0x21,
        'g' => 0x22,
        'h' => 0x23,
        'i' => 0x17,
        'j' => 0x24,
        'k' => 0x25,
        'l' => 0x26,
        'm' => 0x32,
        'n' => 0x31,
        'o' => 0x18,
        'p' => 0x19,
        'q' => 0x10,
        'r' => 0x13,
        's' => 0x1F,
        't' => 0x14,
        'u' => 0x16,
        'v' => 0x2F,
        'w' => 0x11,
        'x' => 0x2D,
        'y' => 0x15,
        'z' => 0x2C,
        '0' | ')' => 0x0B,
        '1' | '!' => 0x02,
        '2' | '@' => 0x03,
        '3' | '#' => 0x04,
        '4' | '$' => 0x05,
        '5' | '%' => 0x06,
        '6' | '^' => 0x07,
        '7' | '&' => 0x08,
        '8' | '*' => 0x09,
        '9' | '(' => 0x0A,
        ' ' => 0x39,
        '\n' | '\r' => 0x1C,
        '\t' => 0x0F,
        '-' | '_' => 0x0C,
        '=' | '+' => 0x0D,
        '[' | '{' => 0x1A,
        ']' | '}' => 0x1B,
        '\\' | '|' => 0x2B,
        ';' | ':' => 0x27,
        '\'' | '"' => 0x28,
        '`' | '~' => 0x29,
        ',' | '<' => 0x33,
        '.' | '>' => 0x34,
        '/' | '?' => 0x35,
        _ => return None,
    };

    Some(scancode)
}

/// 常用扫描码常量
pub mod scancode {
    // 功能键
    pub const ESCAPE: u32 = 0x01;
    pub const BACKSPACE: u32 = 0x0E;
    pub const TAB: u32 = 0x0F;
    pub const ENTER: u32 = 0x1C;
    pub const LEFT_CTRL: u32 = 0x1D;
    pub const LEFT_SHIFT: u32 = 0x2A;
    pub const RIGHT_SHIFT: u32 = 0x36;
    pub const LEFT_ALT: u32 = 0x38;
    pub const SPACE: u32 = 0x39;
    pub const CAPS_LOCK: u32 = 0x3A;
    pub const NUM_LOCK: u32 = 0x45;
    pub const SCROLL_LOCK: u32 = 0x46;

    // F 键
    pub const F1: u32 = 0x3B;
    pub const F2: u32 = 0x3C;
    pub const F3: u32 = 0x3D;
    pub const F4: u32 = 0x3E;
    pub const F5: u32 = 0x3F;
    pub const F6: u32 = 0x40;
    pub const F7: u32 = 0x41;
    pub const F8: u32 = 0x42;
    pub const F9: u32 = 0x43;
    pub const F10: u32 = 0x44;
    pub const F11: u32 = 0x57;
    pub const F12: u32 = 0x58;

    // 扩展键 (需要 0xE0 前缀)
    pub const INSERT: u32 = 0xE052;
    pub const DELETE: u32 = 0xE053;
    pub const HOME: u32 = 0xE047;
    pub const END: u32 = 0xE04F;
    pub const PAGE_UP: u32 = 0xE049;
    pub const PAGE_DOWN: u32 = 0xE051;
    pub const UP: u32 = 0xE048;
    pub const DOWN: u32 = 0xE050;
    pub const LEFT: u32 = 0xE04B;
    pub const RIGHT: u32 = 0xE04D;

    // Windows/Super 键
    pub const LEFT_WIN: u32 = 0xE05B;
    pub const RIGHT_WIN: u32 = 0xE05C;
    pub const MENU: u32 = 0xE05D;

    // 右侧修饰键
    pub const RIGHT_CTRL: u32 = 0xE01D;
    pub const RIGHT_ALT: u32 = 0xE038;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_button_conversion() {
        assert_eq!(MouseButton::Left.to_id(), 1);
        assert_eq!(MouseButton::Right.to_id(), 3);
        assert_eq!(MouseButton::Left.to_mask(), 1);
        assert_eq!(MouseButton::Right.to_mask(), 4);
    }

    #[test]
    fn test_key_modifiers() {
        let mods = KeyModifiers {
            scroll_lock: false,
            num_lock: true,
            caps_lock: true,
        };
        let flags = mods.to_flags();
        let parsed = KeyModifiers::from_flags(flags);

        assert_eq!(parsed.num_lock, true);
        assert_eq!(parsed.caps_lock, true);
        assert_eq!(parsed.scroll_lock, false);
    }

    #[test]
    fn test_char_to_scancode() {
        assert_eq!(char_to_scancode('a'), Some(0x1E));
        assert_eq!(char_to_scancode('A'), Some(0x1E));
        assert_eq!(char_to_scancode('1'), Some(0x02));
        assert_eq!(char_to_scancode(' '), Some(0x39));
        assert_eq!(char_to_scancode('\n'), Some(0x1C));
    }
}

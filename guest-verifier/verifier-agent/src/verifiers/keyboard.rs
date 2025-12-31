//! 键盘验证器实现

use async_trait::async_trait;
#[allow(unused_imports)]
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
#[allow(unused_imports)]
use verifier_core::{Event, RawInputEvent, Result, Verifier, VerifierError, VerifierTransport, VerifierType, VerifyResult};

/// 键盘验证器 trait (保留用于兼容)
#[allow(dead_code)]
#[async_trait]
pub trait KeyboardVerifier: Verifier {
    /// 验证键盘事件
    async fn verify_keyboard(&self, event: &Event) -> Result<VerifyResult>;
}

// ===== Linux 实现 (evdev) =====

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use evdev::{Device, InputEventKind, Key};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// Linux 键盘验证器（使用 evdev）
    pub struct LinuxKeyboardVerifier {
        devices: Arc<Mutex<Vec<Device>>>,
    }

    impl LinuxKeyboardVerifier {
        /// 创建新的 Linux 键盘验证器
        pub fn new() -> Result<Self> {
            info!("初始化 Linux 键盘验证器 (evdev)");

            // 查找所有键盘设备
            let devices = Self::find_keyboard_devices()?;

            if devices.is_empty() {
                error!("未找到键盘设备");
                return Err(VerifierError::VerificationFailed(
                    "未找到键盘设备".to_string(),
                ));
            }

            info!("找到 {} 个键盘设备", devices.len());
            Ok(Self {
                devices: Arc::new(Mutex::new(devices)),
            })
        }

        /// 査找所有键盘设备
        fn find_keyboard_devices() -> Result<Vec<Device>> {
            let mut keyboards = Vec::new();

            // 遍历 /dev/input/event* 设备
            for entry in std::fs::read_dir("/dev/input")
                .map_err(|e| VerifierError::IoError(e))?
            {
                let entry = entry.map_err(|e| VerifierError::IoError(e))?;
                let path = entry.path();

                // 只处理 eventX 设备
                if let Some(filename) = path.file_name() {
                    if let Some(name) = filename.to_str() {
                        if name.starts_with("event") {
                            // 尝试打开设备
                            if let Ok(device) = Device::open(&path) {
                                // 检查是否支持键盘事件
                                if let Some(keys) = device.supported_keys() {
                                    // 检查是否有键盘按键（至少有字母键或数字键）
                                    if keys.contains(Key::KEY_A) || keys.contains(Key::KEY_1) {
                                        debug!("找到键盘设备: {:?} ({})", path, device.name().unwrap_or("未知"));

                                        // 设置为非阻塞模式以确保 fetch_events() 正常工作
                                        use std::os::unix::io::AsRawFd;
                                        let fd = device.as_raw_fd();
                                        unsafe {
                                            let flags = libc::fcntl(fd, libc::F_GETFL, 0);
                                            if flags >= 0 {
                                                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                                            }
                                        }

                                        keyboards.push(device);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok(keyboards)
        }

        /// 获取按键名称
        fn key_to_name(key: Key) -> String {
            let key_name = format!("{:?}", key);
            key_name.strip_prefix("KEY_").unwrap_or(&key_name).to_string()
        }

        /// 启动输入事件监听并转发到服务端
        ///
        /// 此方法会持续监听键盘事件，并将每个事件实时发送到服务端。
        pub async fn start(self, transport: Arc<RwLock<Box<dyn VerifierTransport>>>) -> Result<()> {
            info!("启动键盘输入上报模式");

            loop {
                // 检查所有设备
                let mut devices = self.devices.lock().await;
                for device in devices.iter_mut() {
                    // 尝试读取事件（非阻塞）
                    match device.fetch_events() {
                        Ok(events) => {
                            for event in events {
                                if let InputEventKind::Key(key) = event.kind() {
                                    let key_code = key.code();
                                    let key_name = Self::key_to_name(key);
                                    let value = event.value();

                                    debug!("检测到按键: {} (code: {}, value: {})", key_name, key_code, value);

                                    // 构造原始输入事件
                                    let timestamp = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as i64;

                                    let raw_event = RawInputEvent {
                                        message_type: "raw_input_event".to_string(),
                                        event_type: "keyboard".to_string(),
                                        code: key_code,
                                        value,
                                        name: key_name.clone(),
                                        timestamp,
                                    };

                                    // 发送到服务端
                                    let mut transport = transport.write().await;
                                    if let Err(e) = transport.send_raw_input_event(&raw_event).await {
                                        warn!("发送键盘事件失败: {}", e);
                                    } else {
                                        debug!("已发送键盘事件: {} (value: {})", key_name, value);
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // 非阻塞模式下无事件时会返回错误，忽略
                        }
                    }
                }

                // 短暂休眠避免 CPU 占用过高
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
        }

        /// 匹配按键名称 (保留用于兼容)
        #[allow(dead_code)]
        fn match_key(&self, detected: &str, expected: &str) -> bool {
            // 移除 KEY_ 前缀
            let detected = detected.strip_prefix("KEY_").unwrap_or(detected);
            let expected = expected.strip_prefix("KEY_").unwrap_or(expected);

            // 不区分大小写比较
            detected.eq_ignore_ascii_case(expected)
        }
    }

    #[async_trait]
    impl Verifier for LinuxKeyboardVerifier {
        async fn verify(&self, _event: Event) -> Result<VerifyResult> {
            // 输入上报模式下不再使用 verify 方法
            Err(VerifierError::VerificationFailed(
                "输入上报模式下不支持 verify 方法".to_string(),
            ))
        }

        fn verifier_type(&self) -> VerifierType {
            VerifierType::Keyboard
        }
    }

    #[async_trait]
    impl KeyboardVerifier for LinuxKeyboardVerifier {
        async fn verify_keyboard(&self, _event: &Event) -> Result<VerifyResult> {
            // 输入上报模式下不再使用 verify 方法
            Err(VerifierError::VerificationFailed(
                "输入上报模式下不支持 verify_keyboard 方法".to_string(),
            ))
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::LinuxKeyboardVerifier;

// ===== Windows 实现 (Hook API) =====

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use ::windows::Win32::UI::WindowsAndMessaging::*;

    /// 键盘事件
    #[derive(Debug, Clone)]
    struct KeyEvent {
        key: String,
        vk_code: u32,
        value: i32,  // 1=按下, 0=释放
        timestamp: Instant,
    }

    // 全局事件队列（用于 Hook 回调）
    lazy_static::lazy_static! {
        static ref KEYBOARD_EVENTS: Arc<Mutex<VecDeque<KeyEvent>>> =
            Arc::new(Mutex::new(VecDeque::new()));
    }

    /// Windows 键盘验证器（使用 Hook API）
    pub struct WindowsKeyboardVerifier {
        event_queue: Arc<Mutex<VecDeque<KeyEvent>>>,
        _hook_thread: std::thread::JoinHandle<()>,
    }

    impl WindowsKeyboardVerifier {
        /// 创建新的 Windows 键盘验证器
        pub fn new() -> Result<Self> {
            info!("初始化 Windows 键盘验证器 (Hook API)");

            let event_queue = KEYBOARD_EVENTS.clone();

            // 启动 Hook 线程
            let hook_thread = std::thread::spawn(|| {
                Self::hook_thread();
            });

            // 等待 Hook 安装完成
            std::thread::sleep(std::time::Duration::from_millis(100));

            info!("Windows 键盘验证器初始化成功");
            Ok(Self {
                event_queue,
                _hook_thread: hook_thread,
            })
        }

        /// Hook 线程主函数
        fn hook_thread() {
            unsafe {
                // 设置键盘钩子
                let hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(Self::keyboard_proc),
                    None,
                    0,
                )
                .expect("Failed to set keyboard hook");

                debug!("键盘钩子已安装: {:?}", hook);

                // Windows 消息循环
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                // 清理钩子
                let _ = UnhookWindowsHookEx(hook);
                debug!("键盘钩子已卸载");
            }
        }

        /// 键盘钩子回调函数
        unsafe extern "system" fn keyboard_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                // 获取键盘信息
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                let vk_code = kb.vkCode;
                let msg = wparam.0 as u32;

                // 判断是按下还是释放
                let value = match msg {
                    WM_KEYDOWN | WM_SYSKEYDOWN => 1,
                    WM_KEYUP | WM_SYSKEYUP => 0,
                    _ => return CallNextHookEx(None, code, wparam, lparam),
                };

                // 转换为按键名称
                if let Some(key_name) = Self::vk_code_to_key_name(vk_code) {
                    let event = KeyEvent {
                        key: key_name.clone(),
                        vk_code,
                        value,
                        timestamp: Instant::now(),
                    };

                    // 推送到事件队列
                    if let Ok(mut queue) = KEYBOARD_EVENTS.lock() {
                        queue.push_back(event);
                        // 限制队列大小
                        if queue.len() > 100 {
                            queue.pop_front();
                        }
                    }

                    debug!("检测到按键: {} (VK: 0x{:X}, value: {})", key_name, vk_code, value);
                }
            }

            CallNextHookEx(None, code, wparam, lparam)
        }

        /// 虚拟键码转换为按键名称
        fn vk_code_to_key_name(vk_code: u32) -> Option<String> {
            let vk = vk_code as u16;

            // 字母键 A-Z (0x41-0x5A)
            if vk >= 0x41 && vk <= 0x5A {
                return Some(format!("{}", (vk_code as u8) as char));
            }

            // 数字键 0-9 (0x30-0x39)
            if vk >= 0x30 && vk <= 0x39 {
                return Some(format!("{}", vk_code - 0x30));
            }

            // F1-F12 (0x70-0x7B)
            if vk >= 0x70 && vk <= 0x7B {
                return Some(format!("F{}", vk - 0x70 + 1));
            }

            // 数字键盘 0-9 (0x60-0x69)
            if vk >= 0x60 && vk <= 0x69 {
                return Some(format!("NUMPAD{}", vk - 0x60));
            }

            // 其他功能键使用显式匹配
            match vk {
                // 功能键
                0x0D => Some("ENTER".to_string()),     // VK_RETURN
                0x20 => Some("SPACE".to_string()),     // VK_SPACE
                0x1B => Some("ESC".to_string()),       // VK_ESCAPE
                0x09 => Some("TAB".to_string()),       // VK_TAB
                0x08 => Some("BACKSPACE".to_string()), // VK_BACK
                0x2E => Some("DELETE".to_string()),    // VK_DELETE
                0x2D => Some("INSERT".to_string()),    // VK_INSERT
                0x24 => Some("HOME".to_string()),      // VK_HOME
                0x23 => Some("END".to_string()),       // VK_END
                0x21 => Some("PAGEUP".to_string()),    // VK_PRIOR
                0x22 => Some("PAGEDOWN".to_string()),  // VK_NEXT
                // 方向键
                0x25 => Some("LEFT".to_string()),      // VK_LEFT
                0x27 => Some("RIGHT".to_string()),     // VK_RIGHT
                0x26 => Some("UP".to_string()),        // VK_UP
                0x28 => Some("DOWN".to_string()),      // VK_DOWN
                // 修饰键
                0x10 => Some("SHIFT".to_string()),     // VK_SHIFT
                0x11 => Some("CTRL".to_string()),      // VK_CONTROL
                0x12 => Some("ALT".to_string()),       // VK_MENU
                0x5B => Some("LWIN".to_string()),      // VK_LWIN
                0x5C => Some("RWIN".to_string()),      // VK_RWIN
                0xA0 => Some("LSHIFT".to_string()),    // VK_LSHIFT
                0xA1 => Some("RSHIFT".to_string()),    // VK_RSHIFT
                0xA2 => Some("LCTRL".to_string()),     // VK_LCONTROL
                0xA3 => Some("RCTRL".to_string()),     // VK_RCONTROL
                0xA4 => Some("LALT".to_string()),      // VK_LMENU
                0xA5 => Some("RALT".to_string()),      // VK_RMENU
                // 数字键盘操作符
                0x6A => Some("MULTIPLY".to_string()),  // VK_MULTIPLY
                0x6B => Some("ADD".to_string()),       // VK_ADD
                0x6D => Some("SUBTRACT".to_string()),  // VK_SUBTRACT
                0x6E => Some("DECIMAL".to_string()),   // VK_DECIMAL
                0x6F => Some("DIVIDE".to_string()),    // VK_DIVIDE
                0x90 => Some("NUMLOCK".to_string()),   // VK_NUMLOCK
                // 其他按键
                0x14 => Some("CAPSLOCK".to_string()),  // VK_CAPITAL
                0x91 => Some("SCROLLLOCK".to_string()), // VK_SCROLL
                0x2C => Some("PRINTSCREEN".to_string()), // VK_SNAPSHOT
                0x13 => Some("PAUSE".to_string()),     // VK_PAUSE
                // OEM 键
                0xBA => Some(";".to_string()),         // VK_OEM_1
                0xBB => Some("=".to_string()),         // VK_OEM_PLUS
                0xBC => Some(",".to_string()),         // VK_OEM_COMMA
                0xBD => Some("-".to_string()),         // VK_OEM_MINUS
                0xBE => Some(".".to_string()),         // VK_OEM_PERIOD
                0xBF => Some("/".to_string()),         // VK_OEM_2
                0xC0 => Some("`".to_string()),         // VK_OEM_3
                0xDB => Some("[".to_string()),         // VK_OEM_4
                0xDC => Some("\\".to_string()),        // VK_OEM_5
                0xDD => Some("]".to_string()),         // VK_OEM_6
                0xDE => Some("'".to_string()),         // VK_OEM_7
                _ => None,
            }
        }

        /// 启动输入事件监听并转发到服务端
        ///
        /// 此方法会持续监听键盘事件，并将每个事件实时发送到服务端。
        pub async fn start(self, transport: Arc<RwLock<Box<dyn VerifierTransport>>>) -> Result<()> {
            info!("启动键盘输入上报模式 (Windows)");

            loop {
                // 检查事件队列
                let events_to_send: Vec<KeyEvent> = {
                    if let Ok(mut queue) = self.event_queue.lock() {
                        let events: Vec<_> = queue.drain(..).collect();
                        events
                    } else {
                        Vec::new()
                    }
                };

                // 发送事件
                for event in events_to_send {
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64;

                    let raw_event = RawInputEvent {
                        message_type: "raw_input_event".to_string(),
                        event_type: "keyboard".to_string(),
                        code: event.vk_code as u16,
                        value: event.value,
                        name: event.key.clone(),
                        timestamp,
                    };

                    // 发送到服务端
                    let mut transport = transport.write().await;
                    if let Err(e) = transport.send_raw_input_event(&raw_event).await {
                        warn!("发送键盘事件失败: {}", e);
                    } else {
                        debug!("已发送键盘事件: {} (value: {})", event.key, event.value);
                    }
                }

                // 短暂休眠避免 CPU 占用过高
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
        }

        /// 匹配按键名称
        fn match_key(&self, detected: &str, expected: &str) -> bool {
            // 移除 KEY_ 前缀（兼容 Linux 格式）
            let detected = detected.strip_prefix("KEY_").unwrap_or(detected);
            let expected = expected.strip_prefix("KEY_").unwrap_or(expected);

            // 不区分大小写比较
            detected.eq_ignore_ascii_case(expected)
        }
    }

    #[async_trait]
    impl Verifier for WindowsKeyboardVerifier {
        async fn verify(&self, _event: Event) -> Result<VerifyResult> {
            // 输入上报模式下不再使用 verify 方法
            Err(VerifierError::VerificationFailed(
                "输入上报模式下不支持 verify 方法".to_string(),
            ))
        }

        fn verifier_type(&self) -> VerifierType {
            VerifierType::Keyboard
        }
    }

    #[async_trait]
    impl KeyboardVerifier for WindowsKeyboardVerifier {
        async fn verify_keyboard(&self, _event: &Event) -> Result<VerifyResult> {
            // 输入上报模式下不再使用 verify 方法
            Err(VerifierError::VerificationFailed(
                "输入上报模式下不支持 verify_keyboard 方法".to_string(),
            ))
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_impl::WindowsKeyboardVerifier;

// ===== 其他平台 =====

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("键盘验证器暂不支持当前平台");

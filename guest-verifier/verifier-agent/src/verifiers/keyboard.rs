//! 键盘验证器实现

use async_trait::async_trait;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info};
use verifier_core::{Event, Result, Verifier, VerifierError, VerifierType, VerifyResult};

/// 键盘验证器 trait
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

        /// 查找所有键盘设备
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

        /// 监听键盘事件（带超时）
        async fn wait_for_key_event(&self, expected_key: &str, timeout_ms: u64) -> Result<bool> {
            let timeout = tokio::time::Duration::from_millis(timeout_ms);
            let start_time = tokio::time::Instant::now();

            debug!("等待键盘事件: {} (超时: {}ms)", expected_key, timeout_ms);

            loop {
                // 检查超时
                if start_time.elapsed() > timeout {
                    debug!("等待超时");
                    return Ok(false);
                }

                // 检查所有设备
                let mut devices = self.devices.lock().await;
                for device in devices.iter_mut() {
                    // 尝试读取事件（非阻塞）
                    while let Ok(events) = device.fetch_events() {
                        for event in events {
                            if let InputEventKind::Key(key) = event.kind() {
                                let key_name = format!("{:?}", key);
                                debug!("检测到按键: {} (value: {})", key_name, event.value());

                                // 只检查按下事件（value == 1）
                                if event.value() == 1 {
                                    // 简单的按键名称匹配
                                    if self.match_key(&key_name, expected_key) {
                                        info!("匹配到预期按键: {}", expected_key);
                                        return Ok(true);
                                    }
                                }
                            }
                        }
                    }
                }

                // 短暂休眠避免 CPU 占用过高
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }

        /// 匹配按键名称
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
        async fn verify(&self, event: Event) -> Result<VerifyResult> {
            self.verify_keyboard(&event).await
        }

        fn verifier_type(&self) -> VerifierType {
            VerifierType::Keyboard
        }
    }

    #[async_trait]
    impl KeyboardVerifier for LinuxKeyboardVerifier {
        async fn verify_keyboard(&self, event: &Event) -> Result<VerifyResult> {
            let start_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            debug!("验证键盘事件: {:?}", event);

            // 从事件数据中提取按键信息
            let key = event
                .data
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    VerifierError::VerificationFailed("事件缺少 key 字段".to_string())
                })?;

            // 获取超时时间（默认 5000ms）
            let timeout_ms = event
                .data
                .get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(5000);

            // 等待按键事件
            let verified = self.wait_for_key_event(key, timeout_ms).await?;

            let end_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            let latency_ms = (end_time - start_time) as u64;

            Ok(VerifyResult {
                event_id: event
                    .data
                    .get("event_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                verified,
                timestamp: end_time,
                latency_ms,
                details: json!({
                    "key": key,
                    "platform": "linux",
                    "method": "evdev",
                }),
            })
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::LinuxKeyboardVerifier;

// ===== Windows 实现 (Hook API) =====

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    /// 键盘事件
    #[derive(Debug, Clone)]
    struct KeyEvent {
        key: String,
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
            if code >= 0 && wparam.0 as u32 == WM_KEYDOWN {
                // 获取键盘信息
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                let vk_code = kb.vkCode;

                // 转换为按键名称
                if let Some(key_name) = Self::vk_code_to_key_name(vk_code) {
                    let event = KeyEvent {
                        key: key_name.clone(),
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

                    debug!("检测到按键: {} (VK: 0x{:X})", key_name, vk_code);
                }
            }

            CallNextHookEx(None, code, wparam, lparam)
        }

        /// 虚拟键码转换为按键名称
        fn vk_code_to_key_name(vk_code: u32) -> Option<String> {
            match vk_code as u16 {
                // 字母键 A-Z
                0x41..=0x5A => Some(format!("{}", (vk_code as u8) as char)),
                // 数字键 0-9
                0x30..=0x39 => Some(format!("{}", (vk_code - 0x30))),
                // 功能键
                VK_RETURN.0 => Some("ENTER".to_string()),
                VK_SPACE.0 => Some("SPACE".to_string()),
                VK_ESCAPE.0 => Some("ESC".to_string()),
                VK_TAB.0 => Some("TAB".to_string()),
                VK_BACK.0 => Some("BACKSPACE".to_string()),
                VK_DELETE.0 => Some("DELETE".to_string()),
                VK_INSERT.0 => Some("INSERT".to_string()),
                VK_HOME.0 => Some("HOME".to_string()),
                VK_END.0 => Some("END".to_string()),
                VK_PRIOR.0 => Some("PAGEUP".to_string()),
                VK_NEXT.0 => Some("PAGEDOWN".to_string()),
                // 方向键
                VK_LEFT.0 => Some("LEFT".to_string()),
                VK_RIGHT.0 => Some("RIGHT".to_string()),
                VK_UP.0 => Some("UP".to_string()),
                VK_DOWN.0 => Some("DOWN".to_string()),
                // F1-F12
                VK_F1.0 => Some("F1".to_string()),
                VK_F2.0 => Some("F2".to_string()),
                VK_F3.0 => Some("F3".to_string()),
                VK_F4.0 => Some("F4".to_string()),
                VK_F5.0 => Some("F5".to_string()),
                VK_F6.0 => Some("F6".to_string()),
                VK_F7.0 => Some("F7".to_string()),
                VK_F8.0 => Some("F8".to_string()),
                VK_F9.0 => Some("F9".to_string()),
                VK_F10.0 => Some("F10".to_string()),
                VK_F11.0 => Some("F11".to_string()),
                VK_F12.0 => Some("F12".to_string()),
                // 修饰键
                VK_SHIFT.0 => Some("SHIFT".to_string()),
                VK_CONTROL.0 => Some("CTRL".to_string()),
                VK_MENU.0 => Some("ALT".to_string()),
                VK_LWIN.0 => Some("LWIN".to_string()),
                VK_RWIN.0 => Some("RWIN".to_string()),
                // 数字键盘
                VK_NUMPAD0.0 => Some("NUMPAD0".to_string()),
                VK_NUMPAD1.0 => Some("NUMPAD1".to_string()),
                VK_NUMPAD2.0 => Some("NUMPAD2".to_string()),
                VK_NUMPAD3.0 => Some("NUMPAD3".to_string()),
                VK_NUMPAD4.0 => Some("NUMPAD4".to_string()),
                VK_NUMPAD5.0 => Some("NUMPAD5".to_string()),
                VK_NUMPAD6.0 => Some("NUMPAD6".to_string()),
                VK_NUMPAD7.0 => Some("NUMPAD7".to_string()),
                VK_NUMPAD8.0 => Some("NUMPAD8".to_string()),
                VK_NUMPAD9.0 => Some("NUMPAD9".to_string()),
                VK_MULTIPLY.0 => Some("MULTIPLY".to_string()),
                VK_ADD.0 => Some("ADD".to_string()),
                VK_SUBTRACT.0 => Some("SUBTRACT".to_string()),
                VK_DECIMAL.0 => Some("DECIMAL".to_string()),
                VK_DIVIDE.0 => Some("DIVIDE".to_string()),
                // OEM 键
                VK_OEM_1.0 => Some(";".to_string()),
                VK_OEM_PLUS.0 => Some("=".to_string()),
                VK_OEM_COMMA.0 => Some(",".to_string()),
                VK_OEM_MINUS.0 => Some("-".to_string()),
                VK_OEM_PERIOD.0 => Some(".".to_string()),
                VK_OEM_2.0 => Some("/".to_string()),
                VK_OEM_3.0 => Some("`".to_string()),
                VK_OEM_4.0 => Some("[".to_string()),
                VK_OEM_5.0 => Some("\\".to_string()),
                VK_OEM_6.0 => Some("]".to_string()),
                VK_OEM_7.0 => Some("'".to_string()),
                _ => None,
            }
        }

        /// 等待并匹配键盘事件
        async fn wait_for_key_event(&self, expected_key: &str, timeout_ms: u64) -> Result<bool> {
            let timeout = tokio::time::Duration::from_millis(timeout_ms);
            let start_time = tokio::time::Instant::now();

            debug!("等待键盘事件: {} (超时: {}ms)", expected_key, timeout_ms);

            loop {
                // 检查超时
                if start_time.elapsed() > timeout {
                    debug!("等待超时");
                    return Ok(false);
                }

                // 检查事件队列
                if let Ok(mut queue) = self.event_queue.lock() {
                    // 查找匹配的事件
                    let mut found_index = None;
                    for (i, event) in queue.iter().enumerate() {
                        if self.match_key(&event.key, expected_key) {
                            found_index = Some(i);
                            break;
                        }
                    }

                    // 如果找到匹配事件，移除它并返回成功
                    if let Some(index) = found_index {
                        queue.remove(index);
                        info!("匹配到预期按键: {}", expected_key);
                        return Ok(true);
                    }

                    // 清理过期事件（超过 10 秒）
                    let now = Instant::now();
                    queue.retain(|e| now.duration_since(e.timestamp).as_secs() < 10);
                }

                // 短暂休眠避免 CPU 占用过高
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
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
        async fn verify(&self, event: Event) -> Result<VerifyResult> {
            self.verify_keyboard(&event).await
        }

        fn verifier_type(&self) -> VerifierType {
            VerifierType::Keyboard
        }
    }

    #[async_trait]
    impl KeyboardVerifier for WindowsKeyboardVerifier {
        async fn verify_keyboard(&self, event: &Event) -> Result<VerifyResult> {
            let start_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            debug!("验证键盘事件: {:?}", event);

            // 从事件数据中提取按键信息
            let key = event
                .data
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    VerifierError::VerificationFailed("事件缺少 key 字段".to_string())
                })?;

            // 获取超时时间（默认 5000ms）
            let timeout_ms = event
                .data
                .get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(5000);

            // 等待按键事件
            let verified = self.wait_for_key_event(key, timeout_ms).await?;

            let end_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            let latency_ms = (end_time - start_time) as u64;

            Ok(VerifyResult {
                event_id: event
                    .data
                    .get("event_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                verified,
                timestamp: end_time,
                latency_ms,
                details: json!({
                    "key": key,
                    "platform": "windows",
                    "method": "hook_api",
                }),
            })
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::WindowsKeyboardVerifier;

// ===== 其他平台 =====

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("键盘验证器暂不支持当前平台");

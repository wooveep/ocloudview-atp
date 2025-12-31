//! 鼠标验证器实现

use async_trait::async_trait;
#[allow(unused_imports)]
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
#[allow(unused_imports)]
use verifier_core::{Event, RawInputEvent, Result, Verifier, VerifierError, VerifierTransport, VerifierType, VerifyResult};

/// 鼠标验证器 trait (保留用于兼容)
#[allow(dead_code)]
#[async_trait]
pub trait MouseVerifier: Verifier {
    /// 验证鼠标事件
    async fn verify_mouse(&self, event: &Event) -> Result<VerifyResult>;
}

// ===== Linux 实现 (evdev) =====

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use evdev::{Device, InputEventKind};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// Linux 鼠标验证器（使用 evdev）
    pub struct LinuxMouseVerifier {
        devices: Arc<Mutex<Vec<Device>>>,
    }

    impl LinuxMouseVerifier {
        /// 创建新的 Linux 鼠标验证器
        pub fn new() -> Result<Self> {
            info!("初始化 Linux 鼠标验证器 (evdev)");

            // 查找所有鼠标设备
            let devices = Self::find_mouse_devices()?;

            if devices.is_empty() {
                error!("未找到鼠标设备");
                return Err(VerifierError::VerificationFailed(
                    "未找到鼠标设备".to_string(),
                ));
            }

            info!("找到 {} 个鼠标设备", devices.len());
            Ok(Self {
                devices: Arc::new(Mutex::new(devices)),
            })
        }

        /// 查找所有鼠标设备
        fn find_mouse_devices() -> Result<Vec<Device>> {
            let mut mice = Vec::new();

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
                                // 检查是否支持鼠标事件（相对位置或鼠标按键）
                                let has_mouse_buttons = device
                                    .supported_keys()
                                    .map(|keys| {
                                        keys.contains(evdev::Key::BTN_LEFT)
                                            || keys.contains(evdev::Key::BTN_RIGHT)
                                            || keys.contains(evdev::Key::BTN_MIDDLE)
                                    })
                                    .unwrap_or(false);

                                let has_relative_axes = device
                                    .supported_relative_axes()
                                    .map(|axes| {
                                        axes.contains(evdev::RelativeAxisType::REL_X)
                                            || axes.contains(evdev::RelativeAxisType::REL_Y)
                                    })
                                    .unwrap_or(false);

                                if has_mouse_buttons || has_relative_axes {
                                    debug!(
                                        "找到鼠标设备: {:?} ({})",
                                        path,
                                        device.name().unwrap_or("未知")
                                    );
                                    
                                    // 设置为非阻塞模式以确保 fetch_events() 正常工作
                                    use std::os::unix::io::AsRawFd;
                                    let fd = device.as_raw_fd();
                                    unsafe {
                                        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
                                        if flags >= 0 {
                                            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                                        }
                                    }
                                    
                                    mice.push(device);
                                }
                            }
                        }
                    }
                }
            }

            Ok(mice)
        }

        /// 监听鼠标事件（带超时）(保留用于兼容)
        #[allow(dead_code)]
        async fn wait_for_mouse_event(
            &self,
            event_type: &str,
            timeout_ms: u64,
        ) -> Result<bool> {
            let timeout = tokio::time::Duration::from_millis(timeout_ms);
            let start_time = tokio::time::Instant::now();

            debug!("等待鼠标事件: {} (超时: {}ms)", event_type, timeout_ms);

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
                            match event.kind() {
                                InputEventKind::Key(key) => {
                                    // 鼠标按键事件
                                    if event.value() == 1 {
                                        // 按下事件
                                        let button_name = format!("{:?}", key);
                                        debug!("检测到鼠标按键: {}", button_name);

                                        if self.match_mouse_button(&button_name, event_type) {
                                            info!("匹配到预期鼠标事件: {}", event_type);
                                            return Ok(true);
                                        }
                                    }
                                }
                                InputEventKind::RelAxis(axis) => {
                                    // 鼠标移动事件
                                    let axis_name = format!("{:?}", axis);
                                    debug!("检测到鼠标移动: {} = {}", axis_name, event.value());

                                    if event_type == "move" && event.value() != 0 {
                                        info!("匹配到鼠标移动事件");
                                        return Ok(true);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // 短暂休眠避免 CPU 占用过高
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }

        /// 匹配鼠标按键 (保留用于兼容)
        #[allow(dead_code)]
        fn match_mouse_button(&self, detected: &str, expected: &str) -> bool {
            match expected.to_lowercase().as_str() {
                "left" | "left_click" => detected.contains("BTN_LEFT"),
                "right" | "right_click" => detected.contains("BTN_RIGHT"),
                "middle" | "middle_click" => detected.contains("BTN_MIDDLE"),
                _ => false,
            }
        }

        /// 获取鼠标按键名称
        fn button_to_name(key: evdev::Key) -> String {
            let key_name = format!("{:?}", key);
            // BTN_LEFT -> LEFT, BTN_RIGHT -> RIGHT, etc.
            key_name.strip_prefix("BTN_").unwrap_or(&key_name).to_string()
        }

        /// 获取相对轴名称
        fn axis_to_name(axis: evdev::RelativeAxisType) -> String {
            format!("{:?}", axis)
        }

        /// 启动输入事件监听并转发到服务端
        ///
        /// 此方法会持续监听鼠标事件，并将每个事件实时发送到服务端。
        pub async fn start(self, transport: Arc<RwLock<Box<dyn VerifierTransport>>>) -> Result<()> {
            info!("启动鼠标输入上报模式");

            loop {
                // 检查所有设备
                let mut devices = self.devices.lock().await;
                for device in devices.iter_mut() {
                    // 尝试读取事件（非阻塞）
                    match device.fetch_events() {
                        Ok(events) => {
                            for event in events {
                                let timestamp = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as i64;

                                match event.kind() {
                                    InputEventKind::Key(key) => {
                                        // 鼠标按键事件
                                        let button_code = key.code();
                                        let button_name = Self::button_to_name(key);
                                        let value = event.value();

                                        debug!("检测到鼠标按键: {} (code: {}, value: {})", button_name, button_code, value);

                                        let raw_event = RawInputEvent {
                                            message_type: "raw_input_event".to_string(),
                                            event_type: "mouse".to_string(),
                                            code: button_code,
                                            value,
                                            name: button_name.clone(),
                                            timestamp,
                                        };

                                        // 发送到服务端
                                        let mut transport = transport.write().await;
                                        if let Err(e) = transport.send_raw_input_event(&raw_event).await {
                                            warn!("发送鼠标按键事件失败: {}", e);
                                        } else {
                                            debug!("已发送鼠标按键事件: {} (value: {})", button_name, value);
                                        }
                                    }
                                    InputEventKind::RelAxis(axis) => {
                                        // 鼠标移动事件（仅在有实际移动时上报）
                                        if event.value() != 0 {
                                            let axis_name = Self::axis_to_name(axis);
                                            let axis_code = axis.0;
                                            let value = event.value();

                                            debug!("检测到鼠标移动: {} = {}", axis_name, value);

                                            let raw_event = RawInputEvent {
                                                message_type: "raw_input_event".to_string(),
                                                event_type: "mouse".to_string(),
                                                code: axis_code,
                                                value,
                                                name: axis_name.clone(),
                                                timestamp,
                                            };

                                            // 发送到服务端
                                            let mut transport = transport.write().await;
                                            if let Err(e) = transport.send_raw_input_event(&raw_event).await {
                                                warn!("发送鼠标移动事件失败: {}", e);
                                            } else {
                                                debug!("已发送鼠标移动事件: {} = {}", axis_name, value);
                                            }
                                        }
                                    }
                                    _ => {}
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
    }

    #[async_trait]
    impl Verifier for LinuxMouseVerifier {
        async fn verify(&self, _event: Event) -> Result<VerifyResult> {
            // 输入上报模式下不再使用 verify 方法
            Err(VerifierError::VerificationFailed(
                "输入上报模式下不支持 verify 方法".to_string(),
            ))
        }

        fn verifier_type(&self) -> VerifierType {
            VerifierType::Mouse
        }
    }

    #[async_trait]
    impl MouseVerifier for LinuxMouseVerifier {
        async fn verify_mouse(&self, _event: &Event) -> Result<VerifyResult> {
            // 输入上报模式下不再使用 verify_mouse 方法
            Err(VerifierError::VerificationFailed(
                "输入上报模式下不支持 verify_mouse 方法".to_string(),
            ))
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::LinuxMouseVerifier;

// ===== Windows 实现 (Hook API) =====

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use ::windows::Win32::UI::WindowsAndMessaging::*;

    /// 鼠标事件类型
    #[derive(Debug, Clone, PartialEq)]
    enum MouseEventType {
        LeftClick,
        RightClick,
        MiddleClick,
        Move,
    }

    /// 鼠标事件
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct MouseEvent {
        event_type: MouseEventType,
        position: Option<(i32, i32)>,
        timestamp: Instant,
    }

    // 全局事件队列（用于 Hook 回调）
    lazy_static::lazy_static! {
        static ref MOUSE_EVENTS: Arc<Mutex<VecDeque<MouseEvent>>> =
            Arc::new(Mutex::new(VecDeque::new()));
    }

    /// Windows 鼠标验证器（使用 Hook API）
    pub struct WindowsMouseVerifier {
        event_queue: Arc<Mutex<VecDeque<MouseEvent>>>,
        _hook_thread: std::thread::JoinHandle<()>,
    }

    impl WindowsMouseVerifier {
        /// 创建新的 Windows 鼠标验证器
        pub fn new() -> Result<Self> {
            info!("初始化 Windows 鼠标验证器 (Hook API)");

            let event_queue = MOUSE_EVENTS.clone();

            // 启动 Hook 线程
            let hook_thread = std::thread::spawn(|| {
                Self::hook_thread();
            });

            // 等待 Hook 安装完成
            std::thread::sleep(std::time::Duration::from_millis(100));

            info!("Windows 鼠标验证器初始化成功");
            Ok(Self {
                event_queue,
                _hook_thread: hook_thread,
            })
        }

        /// Hook 线程主函数
        fn hook_thread() {
            unsafe {
                // 设置鼠标钩子
                let hook = SetWindowsHookExW(
                    WH_MOUSE_LL,
                    Some(Self::mouse_proc),
                    None,
                    0,
                )
                .expect("Failed to set mouse hook");

                debug!("鼠标钩子已安装: {:?}", hook);

                // Windows 消息循环
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                // 清理钩子
                let _ = UnhookWindowsHookEx(hook);
                debug!("鼠标钩子已卸载");
            }
        }

        /// 鼠标钩子回调函数
        unsafe extern "system" fn mouse_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let mouse_data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
                let message = wparam.0 as u32;

                let event_type = match message {
                    WM_LBUTTONDOWN => Some(MouseEventType::LeftClick),
                    WM_RBUTTONDOWN => Some(MouseEventType::RightClick),
                    WM_MBUTTONDOWN => Some(MouseEventType::MiddleClick),
                    WM_MOUSEMOVE => Some(MouseEventType::Move),
                    _ => None,
                };

                if let Some(event_type) = event_type {
                    let position = Some((mouse_data.pt.x, mouse_data.pt.y));

                    let event = MouseEvent {
                        event_type: event_type.clone(),
                        position,
                        timestamp: Instant::now(),
                    };

                    // 推送到事件队列
                    if let Ok(mut queue) = MOUSE_EVENTS.lock() {
                        queue.push_back(event);
                        // 限制队列大小
                        if queue.len() > 100 {
                            queue.pop_front();
                        }
                    }

                    debug!(
                        "检测到鼠标事件: {:?} at ({}, {})",
                        event_type,
                        mouse_data.pt.x,
                        mouse_data.pt.y
                    );
                }
            }

            CallNextHookEx(None, code, wparam, lparam)
        }

        /// 等待并匹配鼠标事件
        async fn wait_for_mouse_event(
            &self,
            event_type: &str,
            timeout_ms: u64,
        ) -> Result<bool> {
            let timeout = tokio::time::Duration::from_millis(timeout_ms);
            let start_time = tokio::time::Instant::now();

            debug!("等待鼠标事件: {} (超时: {}ms)", event_type, timeout_ms);

            let expected_type = self.parse_mouse_event_type(event_type)?;

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
                        if event.event_type == expected_type {
                            found_index = Some(i);
                            break;
                        }
                    }

                    // 如果找到匹配事件，移除它并返回成功
                    if let Some(index) = found_index {
                        queue.remove(index);
                        info!("匹配到预期鼠标事件: {}", event_type);
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

        /// 解析鼠标事件类型字符串
        fn parse_mouse_event_type(&self, event_type: &str) -> Result<MouseEventType> {
            match event_type.to_lowercase().as_str() {
                "left" | "left_click" => Ok(MouseEventType::LeftClick),
                "right" | "right_click" => Ok(MouseEventType::RightClick),
                "middle" | "middle_click" => Ok(MouseEventType::MiddleClick),
                "move" => Ok(MouseEventType::Move),
                _ => Err(VerifierError::VerificationFailed(format!(
                    "未知的鼠标事件类型: {}",
                    event_type
                ))),
            }
        }

        /// 获取鼠标事件名称
        fn event_type_to_name(event_type: &MouseEventType) -> String {
            match event_type {
                MouseEventType::LeftClick => "LEFT".to_string(),
                MouseEventType::RightClick => "RIGHT".to_string(),
                MouseEventType::MiddleClick => "MIDDLE".to_string(),
                MouseEventType::Move => "MOVE".to_string(),
            }
        }

        /// 获取鼠标事件代码
        fn event_type_to_code(event_type: &MouseEventType) -> u16 {
            match event_type {
                MouseEventType::LeftClick => 0x01,  // VK_LBUTTON
                MouseEventType::RightClick => 0x02, // VK_RBUTTON
                MouseEventType::MiddleClick => 0x04, // VK_MBUTTON
                MouseEventType::Move => 0x00,
            }
        }

        /// 启动输入事件监听并转发到服务端
        ///
        /// 此方法会持续监听鼠标事件，并将每个事件实时发送到服务端。
        pub async fn start(self, transport: Arc<RwLock<Box<dyn VerifierTransport>>>) -> Result<()> {
            info!("启动鼠标输入上报模式 (Windows)");

            loop {
                // 检查事件队列
                let events_to_send: Vec<MouseEvent> = {
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

                    let name = Self::event_type_to_name(&event.event_type);
                    let code = Self::event_type_to_code(&event.event_type);

                    let raw_event = RawInputEvent {
                        message_type: "raw_input_event".to_string(),
                        event_type: "mouse".to_string(),
                        code,
                        value: 1, // 按下事件
                        name: name.clone(),
                        timestamp,
                    };

                    // 发送到服务端
                    let mut transport = transport.write().await;
                    if let Err(e) = transport.send_raw_input_event(&raw_event).await {
                        warn!("发送鼠标事件失败: {}", e);
                    } else {
                        debug!("已发送鼠标事件: {}", name);
                    }
                }

                // 短暂休眠避免 CPU 占用过高
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
        }
    }

    #[async_trait]
    impl Verifier for WindowsMouseVerifier {
        async fn verify(&self, _event: Event) -> Result<VerifyResult> {
            // 输入上报模式下不再使用 verify 方法
            Err(VerifierError::VerificationFailed(
                "输入上报模式下不支持 verify 方法".to_string(),
            ))
        }

        fn verifier_type(&self) -> VerifierType {
            VerifierType::Mouse
        }
    }

    #[async_trait]
    impl MouseVerifier for WindowsMouseVerifier {
        async fn verify_mouse(&self, _event: &Event) -> Result<VerifyResult> {
            // 输入上报模式下不再使用 verify_mouse 方法
            Err(VerifierError::VerificationFailed(
                "输入上报模式下不支持 verify_mouse 方法".to_string(),
            ))
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_impl::WindowsMouseVerifier;

// ===== 其他平台 =====

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("鼠标验证器暂不支持当前平台");

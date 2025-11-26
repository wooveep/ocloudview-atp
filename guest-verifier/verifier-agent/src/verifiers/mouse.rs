//! 鼠标验证器实现

use async_trait::async_trait;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info};
use verifier_core::{Event, Result, Verifier, VerifierError, VerifierType, VerifyResult};

/// 鼠标验证器 trait
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
                                    mice.push(device);
                                }
                            }
                        }
                    }
                }
            }

            Ok(mice)
        }

        /// 监听鼠标事件（带超时）
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

        /// 匹配鼠标按键
        fn match_mouse_button(&self, detected: &str, expected: &str) -> bool {
            match expected.to_lowercase().as_str() {
                "left" | "left_click" => detected.contains("BTN_LEFT"),
                "right" | "right_click" => detected.contains("BTN_RIGHT"),
                "middle" | "middle_click" => detected.contains("BTN_MIDDLE"),
                _ => false,
            }
        }
    }

    #[async_trait]
    impl Verifier for LinuxMouseVerifier {
        async fn verify(&self, event: Event) -> Result<VerifyResult> {
            self.verify_mouse(&event).await
        }

        fn verifier_type(&self) -> VerifierType {
            VerifierType::Mouse
        }
    }

    #[async_trait]
    impl MouseVerifier for LinuxMouseVerifier {
        async fn verify_mouse(&self, event: &Event) -> Result<VerifyResult> {
            let start_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            debug!("验证鼠标事件: {:?}", event);

            // 从事件数据中提取鼠标操作类型
            let action = event
                .data
                .get("action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    VerifierError::VerificationFailed("事件缺少 action 字段".to_string())
                })?;

            // 获取超时时间（默认 5000ms）
            let timeout_ms = event
                .data
                .get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(5000);

            // 等待鼠标事件
            let verified = self.wait_for_mouse_event(action, timeout_ms).await?;

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
                    "action": action,
                    "platform": "linux",
                    "method": "evdev",
                }),
            })
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::LinuxMouseVerifier;

// ===== Windows 实现 (Hook API) =====

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    // TODO: 实现 Windows 鼠标验证器（使用 Windows Hook API）

    pub struct WindowsMouseVerifier {
        // TODO: 添加 Windows 特定字段
    }

    impl WindowsMouseVerifier {
        pub fn new() -> Result<Self> {
            // TODO: 初始化 Windows 鼠标钩子
            Err(VerifierError::VerificationFailed(
                "Windows 鼠标验证器尚未实现".to_string(),
            ))
        }
    }

    #[async_trait]
    impl Verifier for WindowsMouseVerifier {
        async fn verify(&self, event: Event) -> Result<VerifyResult> {
            self.verify_mouse(&event).await
        }

        fn verifier_type(&self) -> VerifierType {
            VerifierType::Mouse
        }
    }

    #[async_trait]
    impl MouseVerifier for WindowsMouseVerifier {
        async fn verify_mouse(&self, _event: &Event) -> Result<VerifyResult> {
            // TODO: 实现 Windows 鼠标验证逻辑
            Err(VerifierError::VerificationFailed(
                "Windows 鼠标验证器尚未实现".to_string(),
            ))
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::WindowsMouseVerifier;

// ===== 其他平台 =====

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("鼠标验证器暂不支持当前平台");

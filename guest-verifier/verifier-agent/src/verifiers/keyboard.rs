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
    // TODO: 实现 Windows 键盘验证器（使用 Windows Hook API）

    pub struct WindowsKeyboardVerifier {
        // TODO: 添加 Windows 特定字段
    }

    impl WindowsKeyboardVerifier {
        pub fn new() -> Result<Self> {
            // TODO: 初始化 Windows 键盘钩子
            Err(VerifierError::VerificationFailed(
                "Windows 键盘验证器尚未实现".to_string(),
            ))
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
        async fn verify_keyboard(&self, _event: &Event) -> Result<VerifyResult> {
            // TODO: 实现 Windows 键盘验证逻辑
            Err(VerifierError::VerificationFailed(
                "Windows 键盘验证器尚未实现".to_string(),
            ))
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::WindowsKeyboardVerifier;

// ===== 其他平台 =====

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("键盘验证器暂不支持当前平台");

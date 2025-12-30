//! Guest 验证器 Agent
//!
//! 该 Agent 运行在 Guest OS 内部，接收测试事件并验证实际发生的输入/输出

mod verifiers;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use verifier_core::{
    Event, TcpTransport, Verifier, VerifierTransport, VerifierType, WebSocketTransport,
};

// 根据平台导入不同的验证器
#[cfg(target_os = "linux")]
use verifiers::{CommandVerifier, LinuxKeyboardVerifier, LinuxMouseVerifier};

#[cfg(target_os = "windows")]
use verifiers::{CommandVerifier, WindowsKeyboardVerifier, WindowsMouseVerifier};

/// 自动获取 VM ID
///
/// 尝试多种方式获取 VM ID：
/// 1. 从 DMI/SMBIOS 读取 (需要 libvirt 配置 sysinfo)
/// 2. 从系统主机名读取
fn auto_detect_vm_id() -> Result<String> {
    // 方式 1: 尝试从 DMI/SMBIOS 读取
    #[cfg(target_os = "linux")]
    {
        // 优先尝试 product_serial
        if let Ok(serial) = std::fs::read_to_string("/sys/class/dmi/id/product_serial") {
            let vm_id = serial.trim().to_string();
            if !vm_id.is_empty() && vm_id != "Not Specified" {
                debug!("从 DMI product_serial 获取 VM ID: {}", vm_id);
                return Ok(vm_id);
            }
        }

        // 尝试 product_uuid
        if let Ok(uuid) = std::fs::read_to_string("/sys/class/dmi/id/product_uuid") {
            let vm_id = uuid.trim().to_string();
            if !vm_id.is_empty() && vm_id != "Not Specified" {
                debug!("从 DMI product_uuid 获取 VM ID: {}", vm_id);
                return Ok(vm_id);
            }
        }

        debug!("DMI/SMBIOS 信息不可用或为空");
    }

    // 方式 2: 回退到主机名
    #[cfg(target_os = "linux")]
    {
        if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
            let vm_id = hostname.trim().to_string();
            if !vm_id.is_empty() {
                debug!("使用主机名作为 VM ID: {}", vm_id);
                return Ok(vm_id);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // 方式 1: 尝试通过 WMI 获取 BIOS 序列号（虚拟机中通常由 hypervisor 设置）
        if let Ok(output) = Command::new("wmic")
            .args(["bios", "get", "serialnumber"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // 解析 WMIC 输出（第一行是标题，第二行是值）
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.len() >= 2 {
                let serial = lines[1].trim().to_string();
                if !serial.is_empty()
                    && serial != "None"
                    && serial != "Not Specified"
                    && !serial.contains("To Be Filled")
                {
                    debug!("从 WMI BIOS SerialNumber 获取 VM ID: {}", serial);
                    return Ok(serial);
                }
            }
        }

        // 方式 2: 尝试通过 WMI 获取系统 UUID
        if let Ok(output) = Command::new("wmic")
            .args(["csproduct", "get", "uuid"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.len() >= 2 {
                let uuid = lines[1].trim().to_string();
                if !uuid.is_empty()
                    && uuid != "None"
                    && !uuid.starts_with("FFFFFFFF")
                {
                    debug!("从 WMI CSProduct UUID 获取 VM ID: {}", uuid);
                    return Ok(uuid);
                }
            }
        }

        // 方式 3: 尝试通过 PowerShell 获取系统信息（兼容 Win10 及以上）
        if let Ok(output) = Command::new("powershell")
            .args(["-Command", "(Get-WmiObject -Class Win32_ComputerSystemProduct).UUID"])
            .output()
        {
            let uuid = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
            if !uuid.is_empty()
                && uuid != "None"
                && !uuid.starts_with("FFFFFFFF")
            {
                debug!("从 PowerShell 获取系统 UUID 作为 VM ID: {}", uuid);
                return Ok(uuid);
            }
        }

        // 方式 4: 回退到使用计算机名
        if let Ok(output) = Command::new("hostname").output() {
            let hostname = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
            if !hostname.is_empty() {
                debug!("使用主机名作为 VM ID: {}", hostname);
                return Ok(hostname);
            }
        }
    }

    Err(anyhow::anyhow!(
        "无法自动获取 VM ID，请使用 --vm-id 手动指定"
    ))
}

/// 传输类型
#[derive(Debug, Clone, ValueEnum)]
enum TransportType {
    /// WebSocket 传输
    Websocket,
    /// TCP 传输
    Tcp,
}

/// 验证器 Agent CLI 参数
#[derive(Parser, Debug)]
#[command(name = "verifier-agent")]
#[command(about = "Guest 验证器 Agent - 运行在 Guest OS 内部验证输入/输出", long_about = None)]
struct Args {
    /// 服务器地址 (例如: localhost:8080 或 ws://localhost:8080)
    #[arg(short, long, default_value = "localhost:8080")]
    server: String,

    /// 虚拟机 ID（用于标识客户端）
    #[arg(long)]
    vm_id: Option<String>,

    /// 传输类型
    #[arg(short, long, value_enum, default_value = "websocket")]
    transport: TransportType,

    /// 启用的验证器类型 (可多次指定)
    #[arg(short, long, value_enum)]
    verifiers: Vec<VerifierTypeArg>,

    /// 日志级别
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// 自动重连
    #[arg(long, default_value = "true")]
    auto_reconnect: bool,

    /// 重连间隔（秒）
    #[arg(long, default_value = "5")]
    reconnect_interval: u64,

    /// 测试模式：仅监听并显示输入事件，不连接服务器
    #[arg(long)]
    test_input: bool,

    /// 测试模式持续时间（秒），默认 30 秒
    #[arg(long, default_value = "30")]
    test_duration: u64,
}

/// 验证器类型参数
#[derive(Debug, Clone, ValueEnum, PartialEq)]
enum VerifierTypeArg {
    Keyboard,
    Mouse,
    Command,
    All,
}

/// Agent 状态
struct AgentState {
    verifiers: HashMap<VerifierType, Arc<dyn Verifier>>,
    transport: Arc<RwLock<Box<dyn VerifierTransport>>>,
    args: Args,
    vm_id: String, // 实际使用的 VM ID（自动检测或手动指定）
}

impl AgentState {
    /// 创建新的 Agent 状态
    async fn new(args: Args) -> Result<Self> {
        // 确定 VM ID（手动指定优先，否则自动检测）
        let vm_id = match &args.vm_id {
            Some(id) => {
                info!("使用手动指定的 VM ID: {}", id);
                id.clone()
            }
            None => {
                info!("尝试自动获取 VM ID...");
                auto_detect_vm_id().context("自动获取 VM ID 失败")?
            }
        };

        info!("VM ID: {}", vm_id);

        // 创建传输层
        let transport: Box<dyn VerifierTransport> = match args.transport {
            TransportType::Websocket => {
                info!("使用 WebSocket 传输");
                Box::new(WebSocketTransport::new())
            }
            TransportType::Tcp => {
                info!("使用 TCP 传输");
                Box::new(TcpTransport::new())
            }
        };

        let transport = Arc::new(RwLock::new(transport));

        // 创建验证器
        let mut verifiers: HashMap<VerifierType, Arc<dyn Verifier>> = HashMap::new();

        // 确定启用的验证器类型
        let enabled_types = if args.verifiers.is_empty() || args.verifiers.contains(&VerifierTypeArg::All) {
            // 默认启用所有验证器
            vec![
                VerifierTypeArg::Keyboard,
                VerifierTypeArg::Mouse,
                VerifierTypeArg::Command,
            ]
        } else {
            args.verifiers.clone()
        };

        // 初始化验证器
        for verifier_type in enabled_types {
            match verifier_type {
                VerifierTypeArg::Keyboard => {
                    #[cfg(target_os = "linux")]
                    match LinuxKeyboardVerifier::new() {
                        Ok(v) => {
                            info!("启用键盘验证器 (Linux)");
                            verifiers.insert(VerifierType::Keyboard, Arc::new(v));
                        }
                        Err(e) => {
                            warn!("初始化键盘验证器失败: {}", e);
                        }
                    }

                    #[cfg(target_os = "windows")]
                    match WindowsKeyboardVerifier::new() {
                        Ok(v) => {
                            info!("启用键盘验证器 (Windows)");
                            verifiers.insert(VerifierType::Keyboard, Arc::new(v));
                        }
                        Err(e) => {
                            warn!("初始化键盘验证器失败: {}", e);
                        }
                    }
                }
                VerifierTypeArg::Mouse => {
                    #[cfg(target_os = "linux")]
                    match LinuxMouseVerifier::new() {
                        Ok(v) => {
                            info!("启用鼠标验证器 (Linux)");
                            verifiers.insert(VerifierType::Mouse, Arc::new(v));
                        }
                        Err(e) => {
                            warn!("初始化鼠标验证器失败: {}", e);
                        }
                    }

                    #[cfg(target_os = "windows")]
                    match WindowsMouseVerifier::new() {
                        Ok(v) => {
                            info!("启用鼠标验证器 (Windows)");
                            verifiers.insert(VerifierType::Mouse, Arc::new(v));
                        }
                        Err(e) => {
                            warn!("初始化鼠标验证器失败: {}", e);
                        }
                    }
                }
                VerifierTypeArg::Command => {
                    match CommandVerifier::new() {
                        Ok(v) => {
                            info!("启用命令验证器");
                            verifiers.insert(VerifierType::Command, Arc::new(v));
                        }
                        Err(e) => {
                            warn!("初始化命令验证器失败: {}", e);
                        }
                    }
                }
                VerifierTypeArg::All => {
                    // 已在上面处理
                }
            }
        }

        if verifiers.is_empty() {
            return Err(anyhow::anyhow!("没有可用的验证器"));
        }

        info!("已启用 {} 个验证器", verifiers.len());

        Ok(Self {
            verifiers,
            transport,
            args,
            vm_id,
        })
    }

    /// 连接到服务器
    async fn connect(&self) -> Result<()> {
        let mut transport = self.transport.write().await;
        transport
            .connect(&self.args.server, Some(&self.vm_id))
            .await
            .context("连接到服务器失败")?;
        info!("已连接到服务器: {}", self.args.server);
        Ok(())
    }

    /// 处理事件
    async fn handle_event(&self, event: Event) -> Result<()> {
        info!("收到事件: type={}", event.event_type);

        // 根据事件类型选择验证器
        let verifier = match event.event_type.as_str() {
            "keyboard" => self.verifiers.get(&VerifierType::Keyboard),
            "mouse" => self.verifiers.get(&VerifierType::Mouse),
            "command" => self.verifiers.get(&VerifierType::Command),
            _ => {
                warn!("未知事件类型: {}", event.event_type);
                None
            }
        };

        if let Some(verifier) = verifier {
            // 执行验证
            match verifier.verify(event).await {
                Ok(result) => {
                    info!(
                        "验证完成: verified={}, latency={}ms",
                        result.verified, result.latency_ms
                    );

                    // 发送验证结果
                    let mut transport = self.transport.write().await;
                    transport
                        .send_result(&result)
                        .await
                        .context("发送验证结果失败")?;
                }
                Err(e) => {
                    error!("验证失败: {}", e);
                }
            }
        } else {
            warn!("没有合适的验证器处理事件类型: {}", event.event_type);
        }

        Ok(())
    }

    /// 运行事件循环
    async fn run(&self) -> Result<()> {
        info!("启动事件循环");

        loop {
            // 接收事件
            let event = {
                let mut transport = self.transport.write().await;
                match transport.receive_event().await {
                    Ok(event) => event,
                    Err(e) => {
                        error!("接收事件失败: {}", e);

                        // 如果启用了自动重连，尝试重连
                        if self.args.auto_reconnect {
                            warn!(
                                "将在 {} 秒后尝试重连...",
                                self.args.reconnect_interval
                            );
                            tokio::time::sleep(tokio::time::Duration::from_secs(
                                self.args.reconnect_interval,
                            ))
                            .await;

                            // 尝试重连
                            drop(transport); // 释放锁
                            if let Err(e) = self.connect().await {
                                error!("重连失败: {}", e);
                            }
                            continue;
                        } else {
                            return Err(e.into());
                        }
                    }
                }
            };

            // 处理事件
            if let Err(e) = self.handle_event(event).await {
                error!("处理事件失败: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    let args = Args::parse();

    // 初始化日志
    let log_level = args.log_level.clone();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("verifier_agent={},verifier_core={}", log_level, log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 测试模式：仅监听输入事件
    if args.test_input {
        return run_input_test_mode(&args).await;
    }

    info!("启动 Guest 验证器 Agent");
    info!("服务器地址: {}", args.server);
    info!("传输类型: {:?}", args.transport);
    info!("启用的验证器: {:?}", args.verifiers);

    // 创建 Agent 状态
    let state = AgentState::new(args)
        .await
        .context("创建 Agent 状态失败")?;

    // 连接到服务器
    state.connect().await.context("初始连接失败")?;

    // 运行事件循环
    state.run().await.context("事件循环异常退出")?;

    Ok(())
}

/// 运行输入测试模式
async fn run_input_test_mode(args: &Args) -> Result<()> {
    println!("========================================");
    println!("  输入测试模式 (Input Test Mode)");
    println!("========================================");
    println!();
    println!("此模式将监听键盘和鼠标输入事件，并在终端显示检测结果。");
    println!("测试持续时间: {} 秒", args.test_duration);
    println!();
    println!("按 Ctrl+C 可提前退出测试。");
    println!();
    println!("----------------------------------------");
    println!("正在初始化验证器...");
    println!();

    // 确定启用的验证器类型
    let enabled_types = if args.verifiers.is_empty() || args.verifiers.contains(&VerifierTypeArg::All) {
        vec![VerifierTypeArg::Keyboard, VerifierTypeArg::Mouse]
    } else {
        args.verifiers.iter()
            .filter(|v| **v == VerifierTypeArg::Keyboard || **v == VerifierTypeArg::Mouse)
            .cloned()
            .collect()
    };

    if enabled_types.is_empty() {
        println!("错误：测试模式需要启用 keyboard 或 mouse 验证器");
        return Ok(());
    }

    // Windows 平台的测试实现
    #[cfg(target_os = "windows")]
    {
        run_windows_input_test(&enabled_types, args.test_duration).await?;
    }

    // Linux 平台的测试实现
    #[cfg(target_os = "linux")]
    {
        run_linux_input_test(&enabled_types, args.test_duration).await?;
    }

    println!();
    println!("----------------------------------------");
    println!("测试完成！");
    println!("========================================");

    Ok(())
}

/// Windows 平台输入测试
#[cfg(target_os = "windows")]
async fn run_windows_input_test(enabled_types: &[VerifierTypeArg], duration_secs: u64) -> Result<()> {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use ::windows::Win32::UI::WindowsAndMessaging::*;

    // 事件记录
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    enum InputEvent {
        Keyboard { key: String, timestamp: Instant },
        MouseClick { button: String, x: i32, y: i32, timestamp: Instant },
        MouseMove { x: i32, y: i32, timestamp: Instant },
    }

    lazy_static::lazy_static! {
        static ref TEST_EVENTS: Arc<Mutex<VecDeque<InputEvent>>> =
            Arc::new(Mutex::new(VecDeque::new()));
    }

    // 虚拟键码转换
    fn vk_to_name(vk: u32) -> Option<String> {
        let vk = vk as u16;
        if vk >= 0x41 && vk <= 0x5A {
            return Some(format!("{}", (vk as u8) as char));
        }
        if vk >= 0x30 && vk <= 0x39 {
            return Some(format!("{}", vk - 0x30));
        }
        if vk >= 0x70 && vk <= 0x7B {
            return Some(format!("F{}", vk - 0x70 + 1));
        }
        match vk {
            0x0D => Some("ENTER".to_string()),
            0x20 => Some("SPACE".to_string()),
            0x1B => Some("ESC".to_string()),
            0x09 => Some("TAB".to_string()),
            0x08 => Some("BACKSPACE".to_string()),
            0x25 => Some("LEFT".to_string()),
            0x27 => Some("RIGHT".to_string()),
            0x26 => Some("UP".to_string()),
            0x28 => Some("DOWN".to_string()),
            0x10 => Some("SHIFT".to_string()),
            0x11 => Some("CTRL".to_string()),
            0x12 => Some("ALT".to_string()),
            _ => Some(format!("VK_{:#04X}", vk)),
        }
    }

    // 键盘钩子回调
    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && wparam.0 as u32 == WM_KEYDOWN {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            if let Some(key_name) = vk_to_name(kb.vkCode) {
                if let Ok(mut events) = TEST_EVENTS.lock() {
                    events.push_back(InputEvent::Keyboard {
                        key: key_name,
                        timestamp: Instant::now(),
                    });
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    // 鼠标钩子回调
    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let mouse = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let msg = wparam.0 as u32;

            let event = match msg {
                WM_LBUTTONDOWN => Some(InputEvent::MouseClick {
                    button: "左键".to_string(),
                    x: mouse.pt.x,
                    y: mouse.pt.y,
                    timestamp: Instant::now(),
                }),
                WM_RBUTTONDOWN => Some(InputEvent::MouseClick {
                    button: "右键".to_string(),
                    x: mouse.pt.x,
                    y: mouse.pt.y,
                    timestamp: Instant::now(),
                }),
                WM_MBUTTONDOWN => Some(InputEvent::MouseClick {
                    button: "中键".to_string(),
                    x: mouse.pt.x,
                    y: mouse.pt.y,
                    timestamp: Instant::now(),
                }),
                _ => None,
            };

            if let Some(event) = event {
                if let Ok(mut events) = TEST_EVENTS.lock() {
                    events.push_back(event);
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    let test_keyboard = enabled_types.contains(&VerifierTypeArg::Keyboard);
    let test_mouse = enabled_types.contains(&VerifierTypeArg::Mouse);

    println!("启用的测试：");
    if test_keyboard {
        println!("  ✓ 键盘输入监听");
    }
    if test_mouse {
        println!("  ✓ 鼠标点击监听");
    }
    println!();
    println!("开始监听输入事件...");
    println!();

    // 在后台线程启动 Windows Hook
    let _hook_thread = std::thread::spawn(move || {
        unsafe {
            let kb_hook = if test_keyboard {
                Some(SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0)
                    .expect("无法安装键盘钩子"))
            } else {
                None
            };

            let mouse_hook = if test_mouse {
                Some(SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0)
                    .expect("无法安装鼠标钩子"))
            } else {
                None
            };

            // 消息循环
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // 清理
            if let Some(hook) = kb_hook {
                let _ = UnhookWindowsHookEx(hook);
            }
            if let Some(hook) = mouse_hook {
                let _ = UnhookWindowsHookEx(hook);
            }
        }
    });

    // 等待钩子安装
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 主循环：显示事件
    let start_time = Instant::now();
    let duration = std::time::Duration::from_secs(duration_secs);
    let mut event_count = 0u32;

    while start_time.elapsed() < duration {
        // 检查并显示新事件
        if let Ok(mut events) = TEST_EVENTS.lock() {
            while let Some(event) = events.pop_front() {
                event_count += 1;
                match event {
                    InputEvent::Keyboard { key, .. } => {
                        println!("[{:>4}] ⌨️  键盘: {}", event_count, key);
                    }
                    InputEvent::MouseClick { button, x, y, .. } => {
                        println!("[{:>4}] 🖱️  鼠标: {} 点击 @ ({}, {})", event_count, button, x, y);
                    }
                    InputEvent::MouseMove { x, y, .. } => {
                        println!("[{:>4}] 🖱️  鼠标: 移动 @ ({}, {})", event_count, x, y);
                    }
                }
            }
        }

        // 显示剩余时间
        let remaining = duration.saturating_sub(start_time.elapsed());
        if remaining.as_secs() % 5 == 0 && remaining.as_millis() % 1000 < 100 {
            println!("... 剩余 {} 秒 ...", remaining.as_secs());
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    println!();
    println!("总共检测到 {} 个输入事件", event_count);

    // 注意：hook_thread 会在程序退出时自动终止
    // 在生产环境中应该优雅地终止线程

    Ok(())
}

/// Linux 平台输入测试
#[cfg(target_os = "linux")]
async fn run_linux_input_test(enabled_types: &[VerifierTypeArg], duration_secs: u64) -> Result<()> {
    use evdev::{Device, InputEventKind, Key};
    use std::time::Instant;

    let test_keyboard = enabled_types.contains(&VerifierTypeArg::Keyboard);
    let test_mouse = enabled_types.contains(&VerifierTypeArg::Mouse);

    println!("启用的测试：");
    if test_keyboard {
        println!("  ✓ 键盘输入监听");
    }
    if test_mouse {
        println!("  ✓ 鼠标点击监听");
    }
    println!();

    // 查找输入设备
    let mut devices = Vec::new();
    for entry in std::fs::read_dir("/dev/input")? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("event") {
                if let Ok(device) = Device::open(&path) {
                    let has_keyboard = device.supported_keys()
                        .map(|k| k.contains(Key::KEY_A))
                        .unwrap_or(false);
                    let has_mouse = device.supported_keys()
                        .map(|k| k.contains(Key::BTN_LEFT))
                        .unwrap_or(false);

                    if (test_keyboard && has_keyboard) || (test_mouse && has_mouse) {
                        println!("找到设备: {} ({})",
                            device.name().unwrap_or("未知"),
                            path.display());
                        devices.push(device);
                    }
                }
            }
        }
    }

    if devices.is_empty() {
        println!("错误：未找到可用的输入设备");
        println!("提示：可能需要 root 权限或将用户添加到 input 组");
        return Ok(());
    }

    println!();
    println!("开始监听输入事件...");
    println!();

    let start_time = Instant::now();
    let duration = std::time::Duration::from_secs(duration_secs);
    let mut event_count = 0u32;

    while start_time.elapsed() < duration {
        for device in devices.iter_mut() {
            if let Ok(events) = device.fetch_events() {
                for event in events {
                    match event.kind() {
                        InputEventKind::Key(key) if event.value() == 1 => {
                            event_count += 1;
                            let key_name = format!("{:?}", key);
                            if key_name.starts_with("KEY_") {
                                println!("[{:>4}] ⌨️  键盘: {}", event_count,
                                    key_name.strip_prefix("KEY_").unwrap_or(&key_name));
                            } else if key_name.starts_with("BTN_") {
                                println!("[{:>4}] 🖱️  鼠标: {} 点击", event_count,
                                    match key_name.as_str() {
                                        "BTN_LEFT" => "左键",
                                        "BTN_RIGHT" => "右键",
                                        "BTN_MIDDLE" => "中键",
                                        _ => &key_name,
                                    });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    println!();
    println!("总共检测到 {} 个输入事件", event_count);

    Ok(())
}

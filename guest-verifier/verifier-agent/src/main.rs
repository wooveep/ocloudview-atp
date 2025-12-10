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
        // Windows: 使用计算机名
        use std::process::Command;
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

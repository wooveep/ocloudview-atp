mod capture;
mod websocket;

use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};

/// Native Guest Agent - 操作系统层面的输入捕获
#[derive(Parser, Debug)]
#[command(name = "guest-agent-native")]
#[command(about = "原生 Guest Agent，用于捕获操作系统层面的输入事件", long_about = None)]
struct Args {
    /// WebSocket 服务器地址
    #[arg(short, long, default_value = "ws://localhost:8081")]
    server: String,

    /// Agent ID
    #[arg(short, long)]
    agent_id: Option<String>,

    /// 日志级别
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 初始化日志
    let log_level = match args.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    info!("Native Guest Agent 启动中...");
    info!("WebSocket 服务器: {}", args.server);

    // 生成或使用提供的 Agent ID
    let agent_id = args.agent_id.unwrap_or_else(|| {
        format!("native-agent-{}", uuid::Uuid::new_v4())
    });
    info!("Agent ID: {}", agent_id);

    // TODO: 实现实际的功能
    // 1. 连接到 WebSocket 服务器
    // 2. 启动输入捕获（根据操作系统选择相应的实现）
    // 3. 将捕获的事件发送到服务器

    #[cfg(target_os = "linux")]
    {
        info!("检测到 Linux 系统，使用 evdev 捕获");
        // capture::linux::start_capture(&agent_id, &args.server).await?;
    }

    #[cfg(target_os = "windows")]
    {
        info!("检测到 Windows 系统，使用 Windows Hook 捕获");
        // capture::windows::start_capture(&agent_id, &args.server).await?;
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        anyhow::bail!("不支持的操作系统");
    }

    info!("Native Guest Agent 已退出");

    Ok(())
}

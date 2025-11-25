//! ATP CLI 应用

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};

mod commands;
mod config;

#[derive(Parser)]
#[command(name = "atp")]
#[command(about = "OCloudView ATP - 虚拟机自动化测试平台", long_about = None)]
#[command(version)]
struct Cli {
    /// 日志级别
    #[arg(short, long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 主机管理
    Host {
        #[command(subcommand)]
        action: HostAction,
    },

    /// 键盘操作
    Keyboard {
        #[command(subcommand)]
        action: KeyboardAction,
    },

    /// 鼠标操作
    Mouse {
        #[command(subcommand)]
        action: MouseAction,
    },

    /// 命令执行
    Command {
        #[command(subcommand)]
        action: CommandAction,
    },

    /// 场景管理
    Scenario {
        #[command(subcommand)]
        action: ScenarioAction,
    },
}

#[derive(Subcommand)]
enum HostAction {
    /// 添加主机
    Add {
        /// 主机 ID
        id: String,
        /// 主机地址
        host: String,
        /// Libvirt URI
        #[arg(long)]
        uri: Option<String>,
    },
    /// 列出主机
    List,
    /// 移除主机
    Remove { id: String },
}

#[derive(Subcommand)]
enum KeyboardAction {
    /// 发送按键
    Send {
        /// 主机 ID
        #[arg(long)]
        host: String,
        /// 虚拟机名称
        #[arg(long)]
        vm: String,
        /// 按键
        #[arg(long)]
        key: String,
    },
    /// 发送文本
    Text {
        /// 主机 ID
        #[arg(long)]
        host: String,
        /// 虚拟机名称
        #[arg(long)]
        vm: String,
        /// 文本内容
        text: String,
    },
}

#[derive(Subcommand)]
enum MouseAction {
    /// 鼠标点击
    Click {
        /// 主机 ID
        #[arg(long)]
        host: String,
        /// 虚拟机名称
        #[arg(long)]
        vm: String,
        /// X 坐标
        #[arg(long)]
        x: i32,
        /// Y 坐标
        #[arg(long)]
        y: i32,
        /// 按钮 (left/right/middle)
        #[arg(long, default_value = "left")]
        button: String,
    },
    /// 鼠标移动
    Move {
        /// 主机 ID
        #[arg(long)]
        host: String,
        /// 虚拟机名称
        #[arg(long)]
        vm: String,
        /// X 坐标
        #[arg(long)]
        x: i32,
        /// Y 坐标
        #[arg(long)]
        y: i32,
    },
}

#[derive(Subcommand)]
enum CommandAction {
    /// 执行命令
    Exec {
        /// 主机 ID
        #[arg(long)]
        host: String,
        /// 虚拟机名称
        #[arg(long)]
        vm: String,
        /// 命令
        cmd: String,
    },
}

#[derive(Subcommand)]
enum ScenarioAction {
    /// 运行场景
    Run {
        /// 场景文件路径
        file: String,
    },
    /// 列出场景
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    let log_level = match cli.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt().with_max_level(log_level).init();

    info!("ATP CLI 启动");

    // 处理命令
    match cli.command {
        Commands::Host { action } => commands::host::handle(action).await?,
        Commands::Keyboard { action } => commands::keyboard::handle(action).await?,
        Commands::Mouse { action } => commands::mouse::handle(action).await?,
        Commands::Command { action } => commands::command::handle(action).await?,
        Commands::Scenario { action } => commands::scenario::handle(action).await?,
    }

    Ok(())
}

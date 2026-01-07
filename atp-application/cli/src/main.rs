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

    /// 测试报告管理
    Report {
        #[command(subcommand)]
        action: ReportAction,
    },

    /// 数据库管理
    Db {
        #[command(subcommand)]
        action: DbAction,
    },

    /// VDI 平台管理和验证
    Vdi {
        #[command(subcommand)]
        action: VdiAction,
    },

    /// PowerShell 远程执行
    #[command(name = "ps", alias = "powershell")]
    PowerShell {
        #[command(subcommand)]
        action: PowerShellAction,
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
    /// 更新主机 SSH 配置
    UpdateSsh {
        /// 主机 ID
        id: String,
        /// SSH 用户名
        #[arg(long, short = 'u', default_value = "root")]
        username: String,
        /// SSH 密码
        #[arg(long)]
        password: Option<String>,
        /// SSH 端口
        #[arg(long, short = 'p', default_value = "22")]
        port: u16,
        /// SSH 密钥路径
        #[arg(long, short = 'k')]
        key: Option<String>,
    },
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

#[derive(Subcommand)]
pub enum ReportAction {
    /// 列出测试报告
    List {
        /// 场景名称过滤
        #[arg(short, long)]
        scenario: Option<String>,

        /// 只显示通过的
        #[arg(short, long)]
        passed: bool,

        /// 只显示失败的
        #[arg(short, long)]
        failed: bool,

        /// 限制数量
        #[arg(short, long, default_value = "10")]
        limit: i64,
    },

    /// 显示报告详情
    Show {
        /// 报告 ID
        id: i64,
    },

    /// 导出报告
    Export {
        /// 报告 ID
        id: i64,

        /// 输出文件路径
        #[arg(short, long)]
        output: String,

        /// 输出格式(json/yaml)
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// 删除报告
    Delete {
        /// 报告 ID
        id: i64,
    },

    /// 统计信息
    Stats {
        /// 场景名称
        scenario: String,

        /// 天数
        #[arg(short, long, default_value = "30")]
        days: i32,
    },

    /// 清理旧报告
    Cleanup {
        /// 保留最近N天的报告
        #[arg(short, long, default_value = "180")]
        days: i32,

        /// 强制删除不提示确认
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum DbAction {
    /// 初始化数据库
    Init {
        /// 数据库路径
        #[arg(short, long, default_value = "~/.config/atp/data.db")]
        db_path: String,
    },

    /// 升级数据库 (运行迁移)
    Upgrade {
        /// 数据库路径
        #[arg(short, long, default_value = "~/.config/atp/data.db")]
        db_path: String,
    },

    /// 备份数据库
    Backup {
        /// 备份名称(可选,默认使用时间戳)
        #[arg(short, long)]
        name: Option<String>,

        /// 数据库路径
        #[arg(short, long, default_value = "~/.config/atp/data.db")]
        db_path: String,

        /// 备份目录
        #[arg(short, long)]
        backup_dir: Option<String>,
    },

    /// 从备份恢复数据库
    Restore {
        /// 备份文件路径
        backup_path: String,

        /// 数据库路径
        #[arg(short, long, default_value = "~/.config/atp/data.db")]
        db_path: String,

        /// 恢复前先备份当前数据库
        #[arg(long, default_value = "true")]
        safety_backup: bool,
    },

    /// 列出所有备份
    List {
        /// 数据库路径
        #[arg(short, long, default_value = "~/.config/atp/data.db")]
        db_path: String,

        /// 备份目录
        #[arg(short, long)]
        backup_dir: Option<String>,
    },

    /// 删除备份
    Delete {
        /// 备份文件路径
        backup_path: String,
    },

    /// 清理旧备份
    Cleanup {
        /// 保留最新的N个备份
        #[arg(short, long, default_value = "10")]
        keep: usize,

        /// 数据库路径
        #[arg(short, long, default_value = "~/.config/atp/data.db")]
        db_path: String,

        /// 备份目录
        #[arg(short = 'b', long)]
        backup_dir: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum VdiAction {
    /// 验证 VDI 平台与 libvirt 虚拟机状态一致性
    Verify {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,

        /// 只显示不一致的虚拟机
        #[arg(short, long)]
        only_diff: bool,

        /// 输出格式 (table/json/yaml)
        #[arg(short = 'f', long, default_value = "table")]
        format: String,
    },

    /// 列出 VDI 平台的所有主机
    ListHosts {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,
    },

    /// 列出 VDI 平台的所有虚拟机
    ListVms {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,

        /// 主机名过滤
        #[arg(short = 'H', long)]
        host: Option<String>,
    },

    /// 同步 VDI 主机到本地配置
    SyncHosts {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,

        /// 自动连接测试
        #[arg(short, long)]
        test_connection: bool,
    },

    /// 查询虚拟机磁盘存储位置（支持 Gluster 分布式存储定位）
    DiskLocation {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,

        /// 虚拟机 ID 或名称
        #[arg(long)]
        vm: String,

        /// 启用 SSH 连接查询 Gluster 实际 brick 位置（自动从 VDI 获取主机列表）
        #[arg(long)]
        ssh: bool,

        /// SSH 用户名
        #[arg(long, default_value = "root")]
        ssh_user: String,

        /// SSH 密码（不建议在命令行使用，优先使用密钥认证）
        #[arg(long)]
        ssh_password: Option<String>,

        /// SSH 私钥路径
        #[arg(long)]
        ssh_key: Option<String>,

        /// 输出格式 (table/json)
        #[arg(short = 'f', long, default_value = "table")]
        format: String,
    },

    /// 批量启动虚拟机
    Start {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,

        /// 虚拟机名称模式 (*=全部, prefix*=前缀, *suffix=后缀, *middle*=包含, exact=精确)
        #[arg(short, long)]
        pattern: String,

        /// 预览模式，不执行实际操作
        #[arg(long)]
        dry_run: bool,

        /// 启动后通过 QGA 验证虚拟机是否真正启动成功
        #[arg(long)]
        verify: bool,

        /// 输出格式 (table/json)
        #[arg(short = 'f', long, default_value = "table")]
        format: String,
    },

    /// 批量分配虚拟机给用户
    Assign {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,

        /// 虚拟机名称模式
        #[arg(short, long)]
        pattern: String,

        /// 用户名列表（逗号分隔）
        #[arg(long, conflicts_with = "group")]
        users: Option<String>,

        /// 部门/组织单位名称
        #[arg(long, conflicts_with = "users")]
        group: Option<String>,

        /// 预览模式
        #[arg(long)]
        dry_run: bool,

        /// 输出格式 (table/json)
        #[arg(short = 'f', long, default_value = "table")]
        format: String,
    },

    /// 批量重命名虚拟机为绑定用户名
    Rename {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,

        /// 虚拟机名称模式
        #[arg(short, long)]
        pattern: String,

        /// 预览模式
        #[arg(long)]
        dry_run: bool,

        /// 输出格式 (table/json)
        #[arg(short = 'f', long, default_value = "table")]
        format: String,
    },

    /// 批量设置 autoJoinDomain
    SetAutoJoinDomain {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,

        /// 虚拟机名称模式
        #[arg(short, long)]
        pattern: String,

        /// 启用自动加域
        #[arg(long, conflicts_with = "disable")]
        enable: bool,

        /// 禁用自动加域
        #[arg(long, conflicts_with = "enable")]
        disable: bool,

        /// 预览模式
        #[arg(long)]
        dry_run: bool,

        /// 输出格式 (table/json)
        #[arg(short = 'f', long, default_value = "table")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum PowerShellAction {
    /// 执行 PowerShell 命令 (通过 QGA 协议)
    Exec {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,

        /// 目标虚拟机名称（单个）
        #[arg(long, conflicts_with_all = ["vms", "all"])]
        vm: Option<String>,

        /// 目标虚拟机列表（逗号分隔）
        #[arg(long, conflicts_with_all = ["vm", "all"])]
        vms: Option<String>,

        /// 所有虚拟机
        #[arg(long, conflicts_with_all = ["vm", "vms"])]
        all: bool,

        /// 按主机过滤（与 --all 配合使用）
        #[arg(short = 'H', long)]
        host: Option<String>,

        /// 要执行的 PowerShell 命令
        #[arg(long, conflicts_with = "script_file")]
        command: Option<String>,

        /// 要执行的 PowerShell 脚本文件路径
        #[arg(long, conflicts_with = "command")]
        script_file: Option<String>,

        /// 执行超时时间（秒）
        #[arg(short, long, default_value = "60")]
        timeout: u64,

        /// JSON 格式输出
        #[arg(long)]
        json_output: bool,
    },

    /// 列出可用的虚拟机
    ListVms {
        /// 配置文件路径
        #[arg(short, long, default_value = "config/atp.toml")]
        config: String,

        /// 按主机过滤
        #[arg(short = 'H', long)]
        host: Option<String>,
    },
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
        Commands::Report { action } => commands::report::handle(action).await?,
        Commands::Db { action } => commands::db::handle(action).await?,
        Commands::Vdi { action } => commands::vdi::handle(action).await?,
        Commands::PowerShell { action } => commands::powershell::handle(action).await?,
    }

    Ok(())
}

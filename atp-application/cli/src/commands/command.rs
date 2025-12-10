//! Command execution 命令处理

use anyhow::Result;
use colored::Colorize;

use crate::config::CliConfig;

pub async fn handle(action: crate::CommandAction) -> Result<()> {
    match action {
        crate::CommandAction::Exec { host, vm, cmd } => {
            exec_command(&host, &vm, &cmd).await
        }
    }
}

async fn exec_command(host_id: &str, vm_name: &str, cmd: &str) -> Result<()> {
    println!("{} 准备执行命令...", "⚙".cyan());
    println!("  主机: {}", host_id.yellow());
    println!("  虚拟机: {}", vm_name.yellow());
    println!("  命令: {}", cmd.green());

    // 验证主机配置存在
    let config = CliConfig::load()?;
    let _host_config = config.get_host(host_id)?;

    // TODO: 实现实际的命令执行
    // 需要通过 QGA 协议执行命令
    println!("\n{} 此功能需要通过场景文件使用", "ℹ".cyan());
    println!("  提示: 使用 'atp scenario run <file>' 来执行完整的测试场景");

    Ok(())
}

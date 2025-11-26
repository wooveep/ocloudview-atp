//! Mouse 命令处理

use anyhow::Result;
use colored::Colorize;

use crate::config::CliConfig;

pub async fn handle(action: crate::MouseAction) -> Result<()> {
    match action {
        crate::MouseAction::Click { host, vm, x, y, button } => {
            click(&host, &vm, x, y, &button).await
        }
        crate::MouseAction::Move { host, vm, x, y } => {
            move_mouse(&host, &vm, x, y).await
        }
    }
}

async fn click(host_id: &str, vm_name: &str, x: i32, y: i32, button: &str) -> Result<()> {
    println!("{} 准备鼠标点击...", "🖱".cyan());
    println!("  主机: {}", host_id.yellow());
    println!("  虚拟机: {}", vm_name.yellow());
    println!("  位置: ({}, {})", x.to_string().green(), y.to_string().green());
    println!("  按钮: {}", button.green());

    // 验证主机配置存在
    let config = CliConfig::load()?;
    let _host_config = config.get_host(host_id)?;

    // TODO: 实现实际的鼠标点击
    println!("\n{} 此功能需要通过场景文件使用", "ℹ".cyan());
    println!("  提示: 使用 'atp scenario run <file>' 来执行完整的测试场景");

    Ok(())
}

async fn move_mouse(host_id: &str, vm_name: &str, x: i32, y: i32) -> Result<()> {
    println!("{} 准备移动鼠标...", "🖱".cyan());
    println!("  主机: {}", host_id.yellow());
    println!("  虚拟机: {}", vm_name.yellow());
    println!("  位置: ({}, {})", x.to_string().green(), y.to_string().green());

    // 验证主机配置存在
    let config = CliConfig::load()?;
    let _host_config = config.get_host(host_id)?;

    // TODO: 实现实际的鼠标移动
    println!("\n{} 此功能需要通过场景文件使用", "ℹ".cyan());
    println!("  提示: 使用 'atp scenario run <file>' 来执行完整的测试场景");

    Ok(())
}

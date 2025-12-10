//! Keyboard 命令处理

use anyhow::Result;
use colored::Colorize;

use crate::config::CliConfig;

pub async fn handle(action: crate::KeyboardAction) -> Result<()> {
    match action {
        crate::KeyboardAction::Send { host, vm, key } => send_key(&host, &vm, &key).await,
        crate::KeyboardAction::Text { host, vm, text } => send_text(&host, &vm, &text).await,
    }
}

async fn send_key(host_id: &str, vm_name: &str, key: &str) -> Result<()> {
    println!("{} 准备发送按键...", "⌨".cyan());
    println!("  主机: {}", host_id.yellow());
    println!("  虚拟机: {}", vm_name.yellow());
    println!("  按键: {}", key.green());

    // 验证主机配置存在
    let config = CliConfig::load()?;
    let _host_config = config.get_host(host_id)?;

    // TODO: 实现实际的按键发送
    // 需要连接到虚拟机并通过 SPICE 协议发送按键
    println!("\n{} 此功能需要通过场景文件使用", "ℹ".cyan());
    println!("  提示: 使用 'atp scenario run <file>' 来执行完整的测试场景");

    Ok(())
}

async fn send_text(host_id: &str, vm_name: &str, text: &str) -> Result<()> {
    println!("{} 准备发送文本...", "⌨".cyan());
    println!("  主机: {}", host_id.yellow());
    println!("  虚拟机: {}", vm_name.yellow());
    println!("  文本: {}", text.green());

    // 验证主机配置存在
    let config = CliConfig::load()?;
    let _host_config = config.get_host(host_id)?;

    // TODO: 实现实际的文本发送
    println!("\n{} 此功能需要通过场景文件使用", "ℹ".cyan());
    println!("  提示: 使用 'atp scenario run <file>' 来执行完整的测试场景");

    Ok(())
}

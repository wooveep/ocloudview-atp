//! 主机管理命令

use anyhow::Result;
use colored::Colorize;
use crate::config::CliConfig;

pub async fn handle(action: crate::HostAction) -> Result<()> {
    match action {
        crate::HostAction::Add { id, host, uri } => add_host(&id, &host, uri).await,
        crate::HostAction::List => list_hosts().await,
        crate::HostAction::Remove { id } => remove_host(&id).await,
    }
}

async fn add_host(id: &str, host: &str, uri: Option<String>) -> Result<()> {
    let mut config = CliConfig::load()?;

    config.add_host(id, host, uri)?;
    config.save()?;

    println!("{} 主机 {} 添加成功", "✓".green().bold(), id.cyan().bold());
    println!("  地址: {}", host.yellow());

    if let Ok(host_config) = config.get_host(id) {
        if let Some(uri) = &host_config.uri {
            println!("  URI:  {}", uri.yellow());
        }
    }

    if config.default_host.as_deref() == Some(id) {
        println!("  {}", "已设置为默认主机".green());
    }

    Ok(())
}

async fn list_hosts() -> Result<()> {
    let config = CliConfig::load()?;

    if config.hosts.is_empty() {
        println!("{}", "没有配置任何主机".yellow());
        println!("\n使用以下命令添加主机:");
        println!("  {} atp host add <ID> <HOST> [--uri <URI>]", "$".bright_black());
        return Ok(());
    }

    println!("{}\n", "配置的主机列表:".bold());

    let mut hosts: Vec<_> = config.list_hosts();
    hosts.sort_by_key(|(id, _)| *id);

    for (id, host_config) in hosts {
        let is_default = config.default_host.as_deref() == Some(id);
        let marker = if is_default {
            "*".green().bold()
        } else {
            " ".into()
        };

        println!(
            "{} {} {}",
            marker,
            id.cyan().bold(),
            if is_default { "(默认)".green() } else { "".into() }
        );
        println!("    地址: {}", host_config.host.yellow());

        if let Some(uri) = &host_config.uri {
            println!("    URI:  {}", uri.yellow());
        }

        if !host_config.tags.is_empty() {
            println!("    标签: {}", host_config.tags.join(", ").bright_black());
        }

        println!();
    }

    Ok(())
}

async fn remove_host(id: &str) -> Result<()> {
    let mut config = CliConfig::load()?;

    config.remove_host(id)?;
    config.save()?;

    println!("{} 主机 {} 已移除", "✓".green().bold(), id.cyan().bold());

    Ok(())
}

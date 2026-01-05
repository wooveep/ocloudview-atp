//! 主机管理命令

use anyhow::{Context, Result};
use colored::Colorize;
use atp_storage::{Storage, StorageManager, HostRecord};
use chrono::Utc;

pub async fn handle(action: crate::HostAction) -> Result<()> {
    match action {
        crate::HostAction::Add { id, host, uri } => add_host(&id, &host, uri).await,
        crate::HostAction::List => list_hosts().await,
        crate::HostAction::Remove { id } => remove_host(&id).await,
        crate::HostAction::UpdateSsh { id, username, password, port, key } => {
            update_ssh(&id, &username, password.as_deref(), port, key.as_deref()).await
        }
    }
}

async fn get_storage() -> Result<atp_storage::StorageManager> {
    StorageManager::new("~/.config/atp/data.db")
        .await
        .context("无法连接数据库")
}

async fn add_host(id: &str, host: &str, uri: Option<String>) -> Result<()> {
    let storage_manager = get_storage().await?;
    let storage = Storage::from_manager(&storage_manager);
    let host_repo = storage.hosts();
    
    // 检查是否存在
    if host_repo.get_by_id(id).await?.is_some() {
        anyhow::bail!("主机 {} 已存在", id);
    }

    let uri_val = uri.unwrap_or_else(|| format!("qemu+tcp://{}/system", host));
    let now = Utc::now();

    let record = HostRecord {
        id: id.to_string(),
        host: host.to_string(),
        uri: uri_val.clone(),
        tags: None,
        metadata: None,
        ssh_username: Some("root".to_string()),
        ssh_password: None,
        ssh_port: Some(22),
        ssh_key_path: None,
        created_at: now,
        updated_at: now,
    };

    host_repo.upsert(&record).await?;

    println!("{} 主机 {} 添加成功", "✓".green().bold(), id.cyan().bold());
    println!("  地址: {}", host.yellow());
    println!("  URI:  {}", uri_val.yellow());

    Ok(())
}

async fn list_hosts() -> Result<()> {
    let storage_manager = get_storage().await?;
    let storage = Storage::from_manager(&storage_manager);
    let host_repo = storage.hosts();

    let hosts = host_repo.list_all().await?;

    if hosts.is_empty() {
        println!("{}", "没有配置任何主机".yellow());
        println!("\n使用以下命令同步或添加主机:");
        println!("  {} atp vdi sync-hosts", "$".bright_black());
        println!("  {} atp host add <ID> <HOST> [--uri <URI>]", "$".bright_black());
        return Ok(());
    }

    println!("{}\n", "配置的主机列表 (Database):".bold());

    for host in hosts {
        println!(
            "{} {}",
            "*".green().bold(), // 暂时默认都显示标记，因为数据库没有默认主机概念
            host.id.cyan().bold()
        );
        println!("    地址: {}", host.host.yellow());
        println!("    URI:  {}", host.uri.yellow());
        
        // 显示 SSH 配置
        let ssh_user = host.ssh_username.as_deref().unwrap_or("root");
        let ssh_port = host.ssh_port.unwrap_or(22);
        let ssh_info = if let Some(key) = &host.ssh_key_path {
            format!("{}@{}:{} (key: {})", ssh_user, host.host, ssh_port, key)
        } else {
            format!("{}@{}:{}", ssh_user, host.host, ssh_port)
        };
        println!("    SSH:  {}", ssh_info.bright_black());

        if let Some(tags) = host.tags {
            println!("    标签: {}", tags.bright_black());
        }

        println!();
    }

    Ok(())
}

async fn remove_host(id: &str) -> Result<()> {
    let storage_manager = get_storage().await?;
    let storage = Storage::from_manager(&storage_manager);
    let host_repo = storage.hosts();

    if host_repo.delete(id).await? {
        println!("{} 主机 {} 已移除", "✓".green().bold(), id.cyan().bold());
    } else {
        println!("{} 主机 {} 不存在", "✗".red().bold(), id);
    }

    Ok(())
}

/// 更新主机 SSH 配置（存储到数据库）
async fn update_ssh(id: &str, username: &str, password: Option<&str>, port: u16, key: Option<&str>) -> Result<()> {
    let storage_manager = get_storage().await?;
    let storage = Storage::from_manager(&storage_manager);
    let host_repo = storage.hosts();
    
    // 检查主机是否存在
    if host_repo.get_by_id(id).await?.is_none() {
        anyhow::bail!("主机 {} 不存在于数据库中。请先运行 `atp vdi sync-hosts` 同步主机。", id);
    }
    
    // 更新 SSH 配置
    let updated = host_repo
        .update_ssh(id, Some(username), password, Some(port as i32), key)
        .await
        .map_err(|e| anyhow::anyhow!("更新 SSH 配置失败: {}", e))?;
    
    if updated {
        println!("{} 主机 {} SSH 配置已更新", "✓".green().bold(), id.cyan().bold());
        println!("  用户名: {}", username.yellow());
        if let Some(pwd) = password {
            println!("  密码:   {}", "***".yellow());
        }
        println!("  端口:   {}", port.to_string().yellow());
        if let Some(key_path) = key {
            println!("  密钥:   {}", key_path.yellow());
        }
    } else {
        println!("{} 更新失败：主机 {} 不存在", "✗".red().bold(), id);
    }
    
    Ok(())
}

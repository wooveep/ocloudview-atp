use anyhow::Result;
use atp_storage::BackupManager;
use std::path::PathBuf;

pub async fn handle(action: crate::DbAction) -> Result<()> {
    match action {
        crate::DbAction::Backup {
            name,
            db_path,
            backup_dir,
        } => backup_database(name.as_deref(), &db_path, backup_dir).await,

        crate::DbAction::Restore {
            backup_path,
            db_path,
            safety_backup,
        } => restore_database(&backup_path, &db_path, safety_backup).await,

        crate::DbAction::List {
            db_path,
            backup_dir,
        } => list_backups(&db_path, backup_dir).await,

        crate::DbAction::Delete { backup_path } => delete_backup(&backup_path).await,

        crate::DbAction::Cleanup {
            keep,
            db_path,
            backup_dir,
        } => cleanup_backups(keep, &db_path, backup_dir).await,
    }
}

async fn backup_database(
    name: Option<&str>,
    db_path: &str,
    backup_dir: Option<String>,
) -> Result<()> {
    let expanded_db_path = shellexpand::tilde(db_path);
    let backup_dir = backup_dir.map(|p| {
        let expanded = shellexpand::tilde(&p);
        PathBuf::from(expanded.as_ref())
    });

    let manager = BackupManager::new(expanded_db_path.as_ref(), backup_dir)?;

    println!("🔄 正在备份数据库...");
    let backup_path = manager.backup(name)?;

    println!("✅ 数据库已成功备份到: {}", backup_path.display());

    // 显示备份信息
    let backups = manager.list_backups()?;
    if let Some(backup) = backups.iter().find(|b| b.path == backup_path) {
        println!("   大小: {}", backup.size_human_readable());
        println!("   时间: {}", backup.modified.format("%Y-%m-%d %H:%M:%S"));
    }

    Ok(())
}

async fn restore_database(
    backup_path: &str,
    db_path: &str,
    safety_backup: bool,
) -> Result<()> {
    let expanded_db_path = shellexpand::tilde(db_path);
    let expanded_backup_path = shellexpand::tilde(backup_path);

    let manager = BackupManager::new(expanded_db_path.as_ref(), None)?;

    println!("⚠️  警告: 此操作将覆盖当前数据库!");

    if safety_backup {
        println!("🔄 正在创建安全备份...");
    }

    manager.restore(expanded_backup_path.as_ref(), safety_backup)?;

    println!("✅ 数据库已从备份恢复: {}", backup_path);

    Ok(())
}

async fn list_backups(db_path: &str, backup_dir: Option<String>) -> Result<()> {
    let expanded_db_path = shellexpand::tilde(db_path);
    let backup_dir = backup_dir.map(|p| {
        let expanded = shellexpand::tilde(&p);
        PathBuf::from(expanded.as_ref())
    });

    let manager = BackupManager::new(expanded_db_path.as_ref(), backup_dir)?;
    let backups = manager.list_backups()?;

    if backups.is_empty() {
        println!("没有找到备份文件");
        return Ok(());
    }

    println!("📦 数据库备份列表:");
    println!();
    println!(
        "{:<50} {:<12} {:<20}",
        "文件路径", "大小", "备份时间"
    );
    println!("{}", "-".repeat(85));

    for backup in &backups {
        println!(
            "{:<50} {:<12} {:<20}",
            backup.path.display(),
            backup.size_human_readable(),
            backup.modified.format("%Y-%m-%d %H:%M:%S")
        );
    }

    println!();
    println!("总计: {} 个备份", backups.len());

    // 显示总大小
    let total_size: u64 = backups.iter().map(|b| b.size).sum();
    let total_size_mb = total_size as f64 / (1024.0 * 1024.0);
    println!("总大小: {:.2} MB", total_size_mb);

    Ok(())
}

async fn delete_backup(backup_path: &str) -> Result<()> {
    let expanded_path = shellexpand::tilde(backup_path);
    let path = PathBuf::from(expanded_path.as_ref());

    // 从路径推断数据库路径(假设备份在 backups 子目录)
    let db_path = path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("无法推断数据库路径"))?
        .join("data.db");

    let manager = BackupManager::new(db_path, None)?;

    println!("🔄 正在删除备份: {}", path.display());
    manager.delete_backup(&path)?;

    println!("✅ 备份已删除");

    Ok(())
}

async fn cleanup_backups(keep: usize, db_path: &str, backup_dir: Option<String>) -> Result<()> {
    let expanded_db_path = shellexpand::tilde(db_path);
    let backup_dir = backup_dir.map(|p| {
        let expanded = shellexpand::tilde(&p);
        PathBuf::from(expanded.as_ref())
    });

    let manager = BackupManager::new(expanded_db_path.as_ref(), backup_dir)?;

    let backups_before = manager.list_backups()?;
    println!(
        "🔄 正在清理旧备份... (当前: {}, 保留: {})",
        backups_before.len(),
        keep
    );

    let deleted = manager.cleanup_old_backups(keep)?;

    if deleted > 0 {
        println!("✅ 已删除 {} 个旧备份", deleted);

        let backups_after = manager.list_backups()?;
        println!("   剩余 {} 个备份", backups_after.len());
    } else {
        println!("✅ 无需清理,备份数量未超过保留数量");
    }

    Ok(())
}

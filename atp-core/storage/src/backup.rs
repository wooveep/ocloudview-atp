use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// 数据库备份管理器
pub struct BackupManager {
    db_path: PathBuf,
    backup_dir: PathBuf,
}

impl BackupManager {
    /// 创建备份管理器
    ///
    /// # 参数
    /// - `db_path`: 数据库文件路径
    /// - `backup_dir`: 备份目录路径(可选,默认为数据库目录下的 backups/)
    pub fn new(db_path: impl AsRef<Path>, backup_dir: Option<PathBuf>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        let backup_dir = backup_dir
            .unwrap_or_else(|| db_path.parent().unwrap_or(Path::new(".")).join("backups"));

        // 确保备份目录存在
        fs::create_dir_all(&backup_dir)
            .with_context(|| format!("创建备份目录失败: {:?}", backup_dir))?;

        Ok(Self {
            db_path,
            backup_dir,
        })
    }

    /// 创建数据库备份
    ///
    /// # 参数
    /// - `name`: 备份名称(可选,默认使用时间戳)
    ///
    /// # 返回
    /// 备份文件路径
    pub fn backup(&self, name: Option<&str>) -> Result<PathBuf> {
        // 检查数据库文件是否存在
        if !self.db_path.exists() {
            anyhow::bail!("数据库文件不存在: {:?}", self.db_path);
        }

        // 生成备份文件名
        let backup_name = if let Some(name) = name {
            format!("{}.db", name)
        } else {
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            format!("backup_{}.db", timestamp)
        };

        let backup_path = self.backup_dir.join(backup_name);

        // 复制数据库文件
        fs::copy(&self.db_path, &backup_path)
            .with_context(|| format!("备份失败: {:?} -> {:?}", self.db_path, backup_path))?;

        info!("数据库已备份到: {:?}", backup_path);
        Ok(backup_path)
    }

    /// 从备份恢复数据库
    ///
    /// # 参数
    /// - `backup_path`: 备份文件路径
    /// - `create_backup_before_restore`: 恢复前是否先备份当前数据库
    pub fn restore(
        &self,
        backup_path: impl AsRef<Path>,
        create_backup_before_restore: bool,
    ) -> Result<()> {
        let backup_path = backup_path.as_ref();

        // 检查备份文件是否存在
        if !backup_path.exists() {
            anyhow::bail!("备份文件不存在: {:?}", backup_path);
        }

        // 恢复前先备份当前数据库(如果存在)
        if create_backup_before_restore && self.db_path.exists() {
            let safety_backup = self.backup(Some("pre_restore"))?;
            info!("已创建安全备份: {:?}", safety_backup);
        }

        // 恢复数据库
        fs::copy(backup_path, &self.db_path)
            .with_context(|| format!("恢复失败: {:?} -> {:?}", backup_path, self.db_path))?;

        info!("数据库已从备份恢复: {:?}", backup_path);
        Ok(())
    }

    /// 列出所有备份文件
    ///
    /// # 返回
    /// 备份文件列表(按修改时间倒序)
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let mut backups = Vec::new();

        if !self.backup_dir.exists() {
            return Ok(backups);
        }

        for entry in fs::read_dir(&self.backup_dir)
            .with_context(|| format!("读取备份目录失败: {:?}", self.backup_dir))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("db") {
                let metadata = fs::metadata(&path)?;
                let size = metadata.len();
                let modified = metadata.modified()?;

                backups.push(BackupInfo {
                    path,
                    size,
                    modified: modified.into(),
                });
            }
        }

        // 按修改时间倒序排序
        backups.sort_by(|a, b| b.modified.cmp(&a.modified));

        Ok(backups)
    }

    /// 删除指定的备份文件
    ///
    /// # 参数
    /// - `backup_path`: 备份文件路径
    pub fn delete_backup(&self, backup_path: impl AsRef<Path>) -> Result<()> {
        let backup_path = backup_path.as_ref();

        // 确保文件在备份目录中
        if !backup_path.starts_with(&self.backup_dir) {
            anyhow::bail!("备份文件必须在备份目录中: {:?}", backup_path);
        }

        fs::remove_file(backup_path).with_context(|| format!("删除备份失败: {:?}", backup_path))?;

        info!("已删除备份: {:?}", backup_path);
        Ok(())
    }

    /// 清理旧备份
    ///
    /// # 参数
    /// - `keep_count`: 保留最新的N个备份
    pub fn cleanup_old_backups(&self, keep_count: usize) -> Result<usize> {
        let mut backups = self.list_backups()?;

        if backups.len() <= keep_count {
            info!(
                "备份数量({})未超过保留数量({}),无需清理",
                backups.len(),
                keep_count
            );
            return Ok(0);
        }

        // 保留最新的 keep_count 个,删除其余的
        let to_delete = backups.split_off(keep_count);
        let deleted_count = to_delete.len();

        for backup in to_delete {
            if let Err(e) = self.delete_backup(&backup.path) {
                warn!("删除备份失败 {:?}: {}", backup.path, e);
            }
        }

        info!(
            "已清理 {} 个旧备份,保留 {} 个最新备份",
            deleted_count, keep_count
        );
        Ok(deleted_count)
    }

    /// 获取数据库文件大小
    pub fn get_db_size(&self) -> Result<u64> {
        if !self.db_path.exists() {
            return Ok(0);
        }

        let metadata = fs::metadata(&self.db_path)
            .with_context(|| format!("读取数据库文件信息失败: {:?}", self.db_path))?;

        Ok(metadata.len())
    }

    /// 获取备份目录总大小
    pub fn get_backup_total_size(&self) -> Result<u64> {
        let backups = self.list_backups()?;
        Ok(backups.iter().map(|b| b.size).sum())
    }
}

/// 备份文件信息
#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: chrono::DateTime<Utc>,
}

impl BackupInfo {
    /// 格式化文件大小
    pub fn size_human_readable(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.size >= GB {
            format!("{:.2} GB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.2} MB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.2} KB", self.size as f64 / KB as f64)
        } else {
            format!("{} B", self.size)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_backup_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backup_dir = temp_dir.path().join("backups");

        // 创建测试数据库文件
        fs::write(&db_path, b"test data").unwrap();

        let manager = BackupManager::new(&db_path, Some(backup_dir.clone())).unwrap();

        // 测试备份
        let backup_path = manager.backup(None).unwrap();
        assert!(backup_path.exists());
        assert_eq!(fs::read(&backup_path).unwrap(), b"test data");

        // 修改原数据库
        fs::write(&db_path, b"modified data").unwrap();

        // 测试恢复
        manager.restore(&backup_path, false).unwrap();
        assert_eq!(fs::read(&db_path).unwrap(), b"test data");
    }

    #[test]
    fn test_list_backups() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        fs::write(&db_path, b"test").unwrap();

        let manager = BackupManager::new(&db_path, None).unwrap();

        // 创建多个备份
        for i in 0..3 {
            manager.backup(Some(&format!("backup_{}", i))).unwrap();
            thread::sleep(Duration::from_millis(10)); // 确保时间戳不同
        }

        let backups = manager.list_backups().unwrap();
        assert_eq!(backups.len(), 3);
    }

    #[test]
    fn test_cleanup_old_backups() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        fs::write(&db_path, b"test").unwrap();

        let manager = BackupManager::new(&db_path, None).unwrap();

        // 创建5个备份
        for i in 0..5 {
            manager.backup(Some(&format!("backup_{}", i))).unwrap();
            thread::sleep(Duration::from_millis(10));
        }

        // 清理,只保留2个最新的
        let deleted = manager.cleanup_old_backups(2).unwrap();
        assert_eq!(deleted, 3);

        let backups = manager.list_backups().unwrap();
        assert_eq!(backups.len(), 2);
    }

    #[test]
    fn test_delete_backup() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        fs::write(&db_path, b"test").unwrap();

        let manager = BackupManager::new(&db_path, None).unwrap();

        let backup_path = manager.backup(None).unwrap();
        assert!(backup_path.exists());

        manager.delete_backup(&backup_path).unwrap();
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_backup_info_size_human_readable() {
        let info = BackupInfo {
            path: PathBuf::from("test.db"),
            size: 1024,
            modified: Utc::now(),
        };
        assert_eq!(info.size_human_readable(), "1.00 KB");

        let info = BackupInfo {
            path: PathBuf::from("test.db"),
            size: 1024 * 1024,
            modified: Utc::now(),
        };
        assert_eq!(info.size_human_readable(), "1.00 MB");
    }
}

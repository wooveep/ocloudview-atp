//! CLI 配置管理
//!
//! **数据存储方式**: TOML 文件 (~/.config/atp/config.toml)
//! **建议**: 保持现状,主机数量较少时 TOML 文件更合适
//!
//! **未来优化** (当主机数 > 100 时):
//! - 考虑迁移到数据库 (使用 atp-storage 的 hosts 表)
//! - 提供导入/导出功能保持兼容性

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// CLI 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// 主机列表
    #[serde(default)]
    pub hosts: HashMap<String, HostConfig>,

    /// 默认主机 ID
    pub default_host: Option<String>,

    /// 场景目录
    pub scenario_dir: Option<String>,

    /// 配置版本
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// 主机配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    /// 主机地址
    pub host: String,

    /// Libvirt URI
    pub uri: Option<String>,

    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,

    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            hosts: HashMap::new(),
            default_host: None,
            scenario_dir: Some("./scenarios".to_string()),
            version: default_version(),
        }
    }
}

impl CliConfig {
    /// 获取配置文件路径
    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("无法获取用户主目录")?;
        Ok(home.join(".config").join("atp").join("config.toml"))
    }

    /// 加载配置
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("读取配置文件失败: {:?}", path))?;

        toml::from_str(&content)
            .with_context(|| format!("解析配置文件失败: {:?}", path))
    }

    /// 保存配置
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("创建配置目录失败: {:?}", parent))?;
        }

        let content = toml::to_string_pretty(self)
            .context("序列化配置失败")?;

        fs::write(&path, content)
            .with_context(|| format!("写入配置文件失败: {:?}", path))?;

        Ok(())
    }

    /// 添加主机
    pub fn add_host(&mut self, id: &str, host: &str, uri: Option<String>) -> Result<()> {
        if self.hosts.contains_key(id) {
            anyhow::bail!("主机 {} 已存在", id);
        }

        let config = HostConfig {
            host: host.to_string(),
            uri,
            tags: Vec::new(),
            metadata: HashMap::new(),
        };

        self.hosts.insert(id.to_string(), config);

        // 如果是第一个主机，设置为默认主机
        if self.default_host.is_none() {
            self.default_host = Some(id.to_string());
        }

        Ok(())
    }

    /// 移除主机
    pub fn remove_host(&mut self, id: &str) -> Result<()> {
        if !self.hosts.contains_key(id) {
            anyhow::bail!("主机 {} 不存在", id);
        }

        self.hosts.remove(id);

        // 如果移除的是默认主机，清除默认主机
        if self.default_host.as_deref() == Some(id) {
            self.default_host = None;
        }

        Ok(())
    }

    /// 获取主机配置
    pub fn get_host(&self, id: &str) -> Result<&HostConfig> {
        self.hosts
            .get(id)
            .with_context(|| format!("主机 {} 不存在", id))
    }

    /// 列出所有主机
    pub fn list_hosts(&self) -> Vec<(&str, &HostConfig)> {
        self.hosts
            .iter()
            .map(|(id, config)| (id.as_str(), config))
            .collect()
    }

    /// 设置默认主机
    pub fn set_default_host(&mut self, id: &str) -> Result<()> {
        if !self.hosts.contains_key(id) {
            anyhow::bail!("主机 {} 不存在", id);
        }

        self.default_host = Some(id.to_string());
        Ok(())
    }

    /// 获取场景目录
    pub fn get_scenario_dir(&self) -> PathBuf {
        PathBuf::from(self.scenario_dir.as_deref().unwrap_or("./scenarios"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CliConfig::default();
        assert_eq!(config.hosts.len(), 0);
        assert_eq!(config.default_host, None);
        assert_eq!(config.scenario_dir, Some("./scenarios".to_string()));
    }

    #[test]
    fn test_add_remove_host() {
        let mut config = CliConfig::default();

        // 添加主机
        config.add_host("host1", "192.168.1.100", None).unwrap();
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.default_host, Some("host1".to_string()));

        // 添加第二个主机
        config.add_host("host2", "192.168.1.101", Some("qemu:///system".to_string())).unwrap();
        assert_eq!(config.hosts.len(), 2);
        assert_eq!(config.default_host, Some("host1".to_string())); // 默认主机不变

        // 移除主机
        config.remove_host("host2").unwrap();
        assert_eq!(config.hosts.len(), 1);

        // 移除默认主机
        config.remove_host("host1").unwrap();
        assert_eq!(config.hosts.len(), 0);
        assert_eq!(config.default_host, None);
    }

    #[test]
    fn test_duplicate_host() {
        let mut config = CliConfig::default();
        config.add_host("host1", "192.168.1.100", None).unwrap();

        // 尝试添加重复主机应该失败
        let result = config.add_host("host1", "192.168.1.101", None);
        assert!(result.is_err());
    }
}

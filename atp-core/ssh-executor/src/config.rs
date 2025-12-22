//! SSH 配置

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// SSH 认证方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// 密码认证
    Password(String),
    /// 密钥认证（可选密码短语）
    Key {
        /// 私钥路径
        key_path: PathBuf,
        /// 密钥密码短语（可选）
        passphrase: Option<String>,
    },
    /// 使用默认密钥（~/.ssh/id_rsa, ~/.ssh/id_ed25519 等）
    DefaultKey,
}

/// SSH 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    /// 主机地址
    pub host: String,
    /// 端口（默认 22）
    pub port: u16,
    /// 用户名
    pub username: String,
    /// 认证方式
    pub auth: AuthMethod,
    /// 连接超时
    #[serde(with = "humantime_serde", default = "default_connect_timeout")]
    pub connect_timeout: Duration,
    /// 命令执行超时
    #[serde(with = "humantime_serde", default = "default_command_timeout")]
    pub command_timeout: Duration,
}

fn default_connect_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_command_timeout() -> Duration {
    Duration::from_secs(60)
}

impl SshConfig {
    /// 使用密码认证创建配置
    ///
    /// # Arguments
    /// * `host` - 主机地址
    /// * `username` - 用户名
    /// * `password` - 密码
    pub fn with_password(host: impl Into<String>, username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: 22,
            username: username.into(),
            auth: AuthMethod::Password(password.into()),
            connect_timeout: default_connect_timeout(),
            command_timeout: default_command_timeout(),
        }
    }

    /// 使用密钥认证创建配置
    ///
    /// # Arguments
    /// * `host` - 主机地址
    /// * `username` - 用户名
    /// * `key_path` - 私钥路径
    pub fn with_key(host: impl Into<String>, username: impl Into<String>, key_path: impl Into<PathBuf>) -> Self {
        Self {
            host: host.into(),
            port: 22,
            username: username.into(),
            auth: AuthMethod::Key {
                key_path: key_path.into(),
                passphrase: None,
            },
            connect_timeout: default_connect_timeout(),
            command_timeout: default_command_timeout(),
        }
    }

    /// 使用默认密钥认证创建配置
    ///
    /// 将尝试使用 ~/.ssh/id_rsa, ~/.ssh/id_ed25519 等默认密钥
    pub fn with_default_key(host: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: 22,
            username: username.into(),
            auth: AuthMethod::DefaultKey,
            connect_timeout: default_connect_timeout(),
            command_timeout: default_command_timeout(),
        }
    }

    /// 设置端口
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// 设置连接超时
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// 设置命令执行超时
    pub fn command_timeout(mut self, timeout: Duration) -> Self {
        self.command_timeout = timeout;
        self
    }

    /// 获取 SSH 地址字符串（host:port 格式）
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// humantime_serde 模块用于序列化 Duration
mod humantime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_config() {
        let config = SshConfig::with_password("192.168.1.100", "root", "password123");
        assert_eq!(config.host, "192.168.1.100");
        assert_eq!(config.port, 22);
        assert_eq!(config.username, "root");
        assert!(matches!(config.auth, AuthMethod::Password(_)));
    }

    #[test]
    fn test_key_config() {
        let config = SshConfig::with_key("192.168.1.100", "root", "/home/user/.ssh/id_rsa");
        assert!(matches!(config.auth, AuthMethod::Key { .. }));
    }

    #[test]
    fn test_config_builder() {
        let config = SshConfig::with_password("host", "user", "pass")
            .port(2222)
            .connect_timeout(Duration::from_secs(10));
        assert_eq!(config.port, 2222);
        assert_eq!(config.connect_timeout.as_secs(), 10);
    }
}

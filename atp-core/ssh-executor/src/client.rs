//! SSH 客户端实现
//!
//! 使用系统 ssh/sshpass 命令执行远程命令，兼容性更好

use std::path::PathBuf;
use std::process::Stdio;

use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::config::{AuthMethod, SshConfig};
use crate::error::{Result, SshError};

/// 命令执行输出
#[derive(Debug, Clone, Default)]
pub struct CommandOutput {
    /// 标准输出
    pub stdout: String,
    /// 标准错误
    pub stderr: String,
    /// 退出码
    pub exit_code: Option<u32>,
}

impl CommandOutput {
    /// 检查命令是否成功执行
    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// 获取合并的输出（stdout + stderr）
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// SSH 客户端（使用系统 ssh 命令）
pub struct SshClient {
    config: SshConfig,
}

impl SshClient {
    /// 连接到 SSH 服务器（验证连接）
    pub async fn connect(config: SshConfig) -> Result<Self> {
        info!("正在连接 SSH: {}@{}", config.username, config.address());

        let client = Self { config };

        // 验证连接（执行简单命令）
        debug!("验证 SSH 连接...");
        let output = client.execute("echo connected").await?;

        if output.stdout.trim() != "connected" {
            return Err(SshError::ConnectionError(format!(
                "SSH 连接验证失败: {}",
                output.stderr
            )));
        }

        info!("SSH 连接成功: {}@{}", client.config.username, client.config.address());
        Ok(client)
    }

    /// 执行命令
    pub async fn execute(&self, command: &str) -> Result<CommandOutput> {
        debug!("执行命令: {}", command);

        let result = timeout(self.config.command_timeout, self.execute_internal(command))
            .await
            .map_err(|_| SshError::TimeoutError(format!("命令执行超时: {}", command)))?;

        result
    }

    /// 执行命令内部实现
    async fn execute_internal(&self, command: &str) -> Result<CommandOutput> {
        let mut cmd = match &self.config.auth {
            AuthMethod::Password(password) => {
                // 使用 sshpass 进行密码认证
                let mut cmd = Command::new("sshpass");
                cmd.arg("-p").arg(password);
                cmd.arg("ssh");
                cmd
            }
            AuthMethod::Key { key_path, .. } => {
                let mut cmd = Command::new("ssh");
                let expanded_path = expand_path(key_path)?;
                cmd.arg("-i").arg(expanded_path);
                cmd
            }
            AuthMethod::DefaultKey => {
                Command::new("ssh")
            }
        };

        // 通用 SSH 参数
        cmd.arg("-o").arg("StrictHostKeyChecking=no")
            .arg("-o").arg("UserKnownHostsFile=/dev/null")
            .arg("-o").arg(format!("ConnectTimeout={}", self.config.connect_timeout.as_secs()))
            .arg("-o").arg("NumberOfPasswordPrompts=1")
            .arg("-p").arg(self.config.port.to_string())
            .arg(format!("{}@{}", self.config.username, self.config.host))
            .arg(command);

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("执行 SSH 命令...");

        let child = cmd.spawn()
            .map_err(|e| SshError::ExecutionError(format!("启动 SSH 进程失败: {}", e)))?;

        let output = child.wait_with_output().await
            .map_err(|e| SshError::ExecutionError(format!("等待 SSH 进程失败: {}", e)))?;

        let result = CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            exit_code: output.status.code().map(|c| c as u32),
        };

        // 检查是否是认证失败
        if result.exit_code == Some(5) || result.exit_code == Some(255) {
            if result.stderr.contains("Permission denied") ||
               result.stderr.contains("Authentication failed") ||
               result.stderr.contains("password") {
                return Err(SshError::AuthenticationError(format!(
                    "SSH 认证失败: {}",
                    result.stderr
                )));
            }
        }

        debug!(
            "命令执行完成, 退出码: {:?}, stdout 长度: {}, stderr 长度: {}",
            result.exit_code,
            result.stdout.len(),
            result.stderr.len()
        );

        Ok(result)
    }

    /// 执行命令并检查是否成功
    pub async fn execute_checked(&self, command: &str) -> Result<CommandOutput> {
        let output = self.execute(command).await?;

        if !output.is_success() {
            return Err(SshError::ExecutionError(format!(
                "命令执行失败 (退出码 {:?}): {}",
                output.exit_code,
                if output.stderr.is_empty() {
                    &output.stdout
                } else {
                    &output.stderr
                }
            )));
        }

        Ok(output)
    }

    /// 检查文件是否存在
    pub async fn file_exists(&self, path: &str) -> Result<bool> {
        let output = self.execute(&format!("test -e {} && echo 1 || echo 0", path)).await?;
        Ok(output.stdout.trim() == "1")
    }

    /// 读取文件内容
    pub async fn read_file(&self, path: &str) -> Result<String> {
        let output = self.execute_checked(&format!("cat {}", path)).await?;
        Ok(output.stdout)
    }

    /// 获取主机名
    pub async fn get_hostname(&self) -> Result<String> {
        let output = self.execute_checked("hostname").await?;
        Ok(output.stdout)
    }

    /// 关闭连接（对于系统 ssh 命令，无需显式关闭）
    pub async fn disconnect(self) -> Result<()> {
        Ok(())
    }

    /// 获取配置
    pub fn config(&self) -> &SshConfig {
        &self.config
    }
}

/// 展开路径（处理 ~ 等）
fn expand_path(path: &PathBuf) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();
    if path_str.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            let expanded = path_str.replacen('~', &home.to_string_lossy(), 1);
            return Ok(PathBuf::from(expanded));
        }
    }
    Ok(path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_output() {
        let output = CommandOutput {
            stdout: "hello".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
        };
        assert!(output.is_success());
        assert_eq!(output.combined_output(), "hello");
    }

    #[test]
    fn test_expand_path() {
        let path = PathBuf::from("/etc/hosts");
        let expanded = expand_path(&path).unwrap();
        assert_eq!(expanded, path);
    }
}

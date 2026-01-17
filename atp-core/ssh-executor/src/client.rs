//! SSH 客户端实现
//!
//! 使用 russh 库执行远程命令，自动接受所有 host key

use std::path::PathBuf;
use std::sync::Arc;

use russh::client::{self, Handle, KeyboardInteractiveAuthResponse, Msg};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};
use russh::{Channel, ChannelMsg, Disconnect};
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::config::{AuthMethod, SshConfig};
use crate::error::{Result, SshError};

/// 最大 keyboard-interactive 重试次数
const MAX_KEYBOARD_INTERACTIVE_ROUNDS: usize = 10;

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

/// SSH 客户端处理器
///
/// 根据配置决定是否验证 host key:
/// - verify_host_key = true: 拒绝连接（需实现 known_hosts 检查）
/// - verify_host_key = false: 自动接受（存在 MITM 风险）
struct ClientHandler {
    verify_host_key: bool,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    /// 根据配置决定是否接受 host key
    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        if self.verify_host_key {
            // 启用验证时拒绝连接（known_hosts 功能未实现）
            warn!("SSH Host Key 验证已启用，但 known_hosts 功能未实现，拒绝连接");
            Ok(false)
        } else {
            debug!("SSH Host Key 验证已禁用，自动接受 host key");
            Ok(true)
        }
    }
}

/// SSH 客户端（使用 russh 库）
pub struct SshClient {
    config: SshConfig,
    handle: Handle<ClientHandler>,
}

impl SshClient {
    /// 连接到 SSH 服务器
    pub async fn connect(config: SshConfig) -> Result<Self> {
        info!("正在连接 SSH: {}@{}", config.username, config.address());

        let ssh_config = client::Config {
            inactivity_timeout: Some(config.command_timeout),
            ..Default::default()
        };

        let handler = ClientHandler {
            verify_host_key: config.verify_host_key,
        };

        // 建立 TCP 连接并进行 SSH 握手
        let handle = timeout(
            config.connect_timeout,
            client::connect(Arc::new(ssh_config), config.address(), handler),
        )
        .await
        .map_err(|_| SshError::TimeoutError("SSH 连接超时".to_string()))?
        .map_err(|e| SshError::ConnectionError(format!("SSH 连接失败: {}", e)))?;

        let mut client = Self { config, handle };

        // 进行认证
        client.authenticate().await?;

        info!(
            "SSH 连接成功: {}@{}",
            client.config.username,
            client.config.address()
        );

        // 验证连接
        debug!("验证 SSH 连接...");
        let output = client.execute("echo connected").await?;
        if output.stdout.trim() != "connected" {
            return Err(SshError::ConnectionError(format!(
                "SSH 连接验证失败: {}",
                output.stderr
            )));
        }

        Ok(client)
    }

    /// 执行认证
    async fn authenticate(&mut self) -> Result<()> {
        let username = self.config.username.clone();

        match &self.config.auth {
            AuthMethod::Password(password) => {
                let password = password.clone();
                debug!(
                    "使用密码认证, 用户名: {}, 密码长度: {}",
                    username,
                    password.len()
                );

                // 首先尝试普通密码认证
                let auth_result = self
                    .handle
                    .authenticate_password(&username, &password)
                    .await
                    .map_err(|e| {
                        warn!("密码认证错误: {:?}", e);
                        SshError::AuthenticationError(format!("密码认证失败: {}", e))
                    })?;

                debug!("密码认证结果: {:?}", auth_result);

                if auth_result.success() {
                    debug!("密码认证成功");
                    return Ok(());
                }

                // 如果密码认证失败，检查是否支持 keyboard-interactive
                if let russh::client::AuthResult::Failure {
                    remaining_methods, ..
                } = &auth_result
                {
                    if remaining_methods.contains(&russh::MethodKind::KeyboardInteractive) {
                        debug!("服务器不支持密码认证，尝试 keyboard-interactive 认证");
                        return self
                            .authenticate_keyboard_interactive(&username, &password)
                            .await;
                    }
                }

                warn!("密码认证失败, AuthResult: {:?}", auth_result);
                return Err(SshError::AuthenticationError(format!(
                    "密码认证失败: 用户名或密码错误, 结果: {:?}",
                    auth_result
                )));
            }
            AuthMethod::Key {
                key_path,
                passphrase,
            } => {
                debug!("使用密钥认证: {:?}", key_path);
                let expanded_path = expand_path(key_path)?;
                let key_pair = load_key(&expanded_path, passphrase.as_deref())?;

                // 获取服务器支持的最佳 RSA 哈希算法
                let hash_alg = self
                    .handle
                    .best_supported_rsa_hash()
                    .await
                    .map_err(|e| SshError::SessionError(format!("获取哈希算法失败: {}", e)))?
                    .flatten();

                let key_with_hash = PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg);

                let auth_result = self
                    .handle
                    .authenticate_publickey(&username, key_with_hash)
                    .await
                    .map_err(|e| SshError::AuthenticationError(format!("密钥认证失败: {}", e)))?;

                if !auth_result.success() {
                    return Err(SshError::AuthenticationError(
                        "密钥认证失败: 密钥无效或不被服务器接受".to_string(),
                    ));
                }
            }
            AuthMethod::DefaultKey => {
                debug!("使用默认密钥认证");
                let key_pair = load_default_key()?;

                // 获取服务器支持的最佳 RSA 哈希算法
                let hash_alg = self
                    .handle
                    .best_supported_rsa_hash()
                    .await
                    .map_err(|e| SshError::SessionError(format!("获取哈希算法失败: {}", e)))?
                    .flatten();

                let key_with_hash = PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg);

                let auth_result = self
                    .handle
                    .authenticate_publickey(&username, key_with_hash)
                    .await
                    .map_err(|e| SshError::AuthenticationError(format!("密钥认证失败: {}", e)))?;

                if !auth_result.success() {
                    return Err(SshError::AuthenticationError(
                        "默认密钥认证失败".to_string(),
                    ));
                }
            }
        }

        debug!("认证成功");
        Ok(())
    }

    /// 使用 keyboard-interactive 方式进行认证
    async fn authenticate_keyboard_interactive(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<()> {
        debug!("开始 keyboard-interactive 认证");

        // 启动 keyboard-interactive 认证
        let mut response = self
            .handle
            .authenticate_keyboard_interactive_start(username, None)
            .await
            .map_err(|e| {
                SshError::AuthenticationError(format!("keyboard-interactive 认证失败: {}", e))
            })?;

        // 处理认证流程（可能需要多轮交互）
        for round in 0..MAX_KEYBOARD_INTERACTIVE_ROUNDS {
            debug!(
                "keyboard-interactive 第 {} 轮, 响应: {:?}",
                round + 1,
                response
            );

            match response {
                KeyboardInteractiveAuthResponse::Success => {
                    debug!("keyboard-interactive 认证成功");
                    return Ok(());
                }
                KeyboardInteractiveAuthResponse::Failure {
                    remaining_methods, ..
                } => {
                    warn!(
                        "keyboard-interactive 认证失败, 剩余方法: {:?}",
                        remaining_methods
                    );
                    return Err(SshError::AuthenticationError(
                        "keyboard-interactive 认证失败: 用户名或密码错误".to_string(),
                    ));
                }
                KeyboardInteractiveAuthResponse::InfoRequest {
                    name,
                    instructions,
                    prompts,
                } => {
                    debug!(
                        "收到 InfoRequest: name={}, instructions={}, prompts={:?}",
                        name, instructions, prompts
                    );

                    // 对每个 prompt 回复密码
                    // 通常第一个 prompt 是密码提示
                    let responses: Vec<String> =
                        prompts.iter().map(|_| password.to_string()).collect();

                    debug!("发送 {} 个响应", responses.len());

                    response = self
                        .handle
                        .authenticate_keyboard_interactive_respond(responses)
                        .await
                        .map_err(|e| {
                            SshError::AuthenticationError(format!(
                                "keyboard-interactive 响应失败: {}",
                                e
                            ))
                        })?;
                }
            }
        }

        Err(SshError::AuthenticationError(
            "keyboard-interactive 认证失败: 超过最大交互轮数".to_string(),
        ))
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
        // 打开一个新的会话通道
        let mut channel: Channel<Msg> = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| SshError::ChannelError(format!("打开通道失败: {}", e)))?;

        // 执行命令
        channel
            .exec(true, command)
            .await
            .map_err(|e| SshError::ExecutionError(format!("执行命令失败: {}", e)))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = None;

        // 读取命令输出
        loop {
            match channel.wait().await {
                Some(ChannelMsg::Data { data }) => {
                    stdout.extend_from_slice(&data);
                }
                Some(ChannelMsg::ExtendedData { data, ext }) => {
                    if ext == 1 {
                        // stderr
                        stderr.extend_from_slice(&data);
                    }
                }
                Some(ChannelMsg::ExitStatus { exit_status }) => {
                    exit_code = Some(exit_status);
                }
                Some(ChannelMsg::Eof) => {
                    // 命令输出结束
                }
                Some(ChannelMsg::Close) | None => {
                    break;
                }
                _ => {}
            }
        }

        let result = CommandOutput {
            stdout: String::from_utf8_lossy(&stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&stderr).trim().to_string(),
            exit_code,
        };

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
        let output = self
            .execute(&format!("test -e {} && echo 1 || echo 0", path))
            .await?;
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

    /// 关闭连接
    pub async fn disconnect(self) -> Result<()> {
        self.handle
            .disconnect(Disconnect::ByApplication, "", "English")
            .await
            .map_err(|e| SshError::SessionError(format!("断开连接失败: {}", e)))?;
        Ok(())
    }

    /// 获取配置
    pub fn config(&self) -> &SshConfig {
        &self.config
    }
}

/// 加载私钥文件
fn load_key(path: &PathBuf, passphrase: Option<&str>) -> Result<russh::keys::PrivateKey> {
    let key = load_secret_key(path, passphrase)
        .map_err(|e| SshError::KeyLoadError(format!("加载密钥失败 {:?}: {}", path, e)))?;
    Ok(key)
}

/// 加载默认密钥（尝试多个常见位置）
fn load_default_key() -> Result<russh::keys::PrivateKey> {
    let home =
        dirs::home_dir().ok_or_else(|| SshError::KeyLoadError("无法获取用户主目录".to_string()))?;

    let key_paths = [
        home.join(".ssh/id_ed25519"),
        home.join(".ssh/id_rsa"),
        home.join(".ssh/id_ecdsa"),
    ];

    for path in &key_paths {
        if path.exists() {
            debug!("尝试加载默认密钥: {:?}", path);
            match load_secret_key(path, None) {
                Ok(key) => {
                    info!("成功加载默认密钥: {:?}", path);
                    return Ok(key);
                }
                Err(e) => {
                    debug!("加载密钥失败 {:?}: {}", path, e);
                    continue;
                }
            }
        }
    }

    Err(SshError::KeyLoadError(
        "未找到可用的默认 SSH 密钥".to_string(),
    ))
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

    #[test]
    fn test_expand_home_path() {
        let path = PathBuf::from("~/.ssh/id_rsa");
        let expanded = expand_path(&path).unwrap();
        assert!(!expanded.to_string_lossy().starts_with('~'));
    }
}

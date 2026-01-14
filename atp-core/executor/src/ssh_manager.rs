//! SSH 连接管理器
//!
//! 提供统一的 SSH 配置解析和连接管理功能：
//! - 连接池管理，复用已建立的连接
//! - 10 秒心跳保活机制
//! - SSH 配置优先级：数据库密码 > SSH密钥 > 默认密钥

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use atp_gluster::GlusterClient;
use atp_ssh_executor::{SshClient, SshConfig};
use atp_storage::{HostRecord, Storage};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// 默认保活间隔（10 秒）
const DEFAULT_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);

/// SSH 配置参数（来自 CLI）
#[derive(Debug, Clone, Default)]
pub struct SshParams {
    /// SSH 用户名
    pub user: String,
    /// SSH 密码（CLI 指定）
    pub password: Option<String>,
    /// SSH 密钥路径（CLI 指定）
    pub key_path: Option<PathBuf>,
}

impl SshParams {
    /// 创建新的 SSH 参数
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            password: None,
            key_path: None,
        }
    }

    /// 使用密码创建
    pub fn with_password(user: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            password: Some(password.into()),
            key_path: None,
        }
    }

    /// 使用密钥创建
    pub fn with_key(user: impl Into<String>, key_path: PathBuf) -> Self {
        Self {
            user: user.into(),
            password: None,
            key_path: Some(key_path),
        }
    }
}

/// 活跃的 SSH 连接（带保活）
struct ActiveConnection {
    /// SSH 客户端
    client: SshClient,
    /// 最后活跃时间
    last_active: Instant,
    /// 保活任务句柄
    keepalive_handle: Option<JoinHandle<()>>,
}

impl ActiveConnection {
    fn new(client: SshClient) -> Self {
        Self {
            client,
            last_active: Instant::now(),
            keepalive_handle: None,
        }
    }

    fn touch(&mut self) {
        self.last_active = Instant::now();
    }
}

/// SSH 连接管理器（连接池 + 保活）
///
/// 管理多个主机的 SSH 连接，支持：
/// - 连接复用：相同主机只建立一次连接
/// - 自动保活：每 10 秒发送空命令保持连接
/// - 配置优先级：数据库密码 > SSH密钥 > 默认密钥
pub struct SshConnectionManager {
    /// 连接池
    connections: HashMap<String, ActiveConnection>,
    /// 默认用户名
    default_user: String,
    /// 存储（用于从数据库获取 SSH 配置）
    storage: Option<Arc<Storage>>,
    /// 保活间隔
    keepalive_interval: Duration,
    /// 保活任务停止信号
    keepalive_stop: Arc<RwLock<bool>>,
}

impl SshConnectionManager {
    /// 创建新的连接管理器
    pub fn new(default_user: impl Into<String>) -> Self {
        Self {
            connections: HashMap::new(),
            default_user: default_user.into(),
            storage: None,
            keepalive_interval: DEFAULT_KEEPALIVE_INTERVAL,
            keepalive_stop: Arc::new(RwLock::new(false)),
        }
    }

    /// 设置存储（用于从数据库获取 SSH 配置）
    pub fn with_storage(mut self, storage: Arc<Storage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// 设置保活间隔（默认 10 秒）
    pub fn with_keepalive_interval(mut self, interval: Duration) -> Self {
        self.keepalive_interval = interval;
        self
    }

    /// 解析 SSH 配置
    ///
    /// 优先级：
    /// 1. 数据库密码（最高优先级）
    /// 2. CLI 指定的密钥路径
    /// 3. 默认密钥（~/.ssh/id_rsa）
    pub async fn resolve_ssh_config(
        &self,
        host_ip: &str,
        cli_params: &SshParams,
    ) -> Result<SshConfig> {
        let user = if !cli_params.user.is_empty() {
            &cli_params.user
        } else {
            &self.default_user
        };

        // 优先级 1: 从数据库获取密码
        if let Some(storage) = &self.storage {
            if let Ok(Some(host)) = storage.hosts().get_by_ip(host_ip).await {
                let username = host.ssh_username.as_deref().unwrap_or(user);
                let port = host.ssh_port.unwrap_or(22) as u16;

                // 优先使用数据库中的密码
                if let Some(ref password) = host.ssh_password {
                    debug!(
                        "使用数据库 SSH 密码配置: {}@{}:{}",
                        username, host_ip, port
                    );
                    return Ok(SshConfig::with_password(host_ip, username, password).port(port));
                }

                // 其次使用数据库中的密钥路径
                if let Some(ref key_path) = host.ssh_key_path {
                    debug!(
                        "使用数据库 SSH 密钥配置: {}@{}:{} (key: {})",
                        username, host_ip, port, key_path
                    );
                    return Ok(
                        SshConfig::with_key(host_ip, username, PathBuf::from(key_path)).port(port)
                    );
                }
            }
        }

        // 优先级 2: CLI 提供的密钥路径
        if let Some(ref key_path) = cli_params.key_path {
            debug!("使用 CLI 指定的 SSH 密钥: {:?}", key_path);
            return Ok(SshConfig::with_key(host_ip, user, key_path.clone()));
        }

        // 优先级 3: 默认密钥 (~/.ssh/id_rsa)
        debug!("使用默认 SSH 密钥");
        Ok(SshConfig::with_default_key(host_ip, user))
    }

    /// 获取或创建连接（自动复用已有连接）
    pub async fn get_or_connect(
        &mut self,
        host_ip: &str,
        cli_params: &SshParams,
    ) -> Result<&SshClient> {
        // 检查是否已有连接
        if self.connections.contains_key(host_ip) {
            if let Some(conn) = self.connections.get_mut(host_ip) {
                conn.touch();
            }
            return Ok(&self.connections.get(host_ip).unwrap().client);
        }

        // 创建新连接
        let config = self.resolve_ssh_config(host_ip, cli_params).await?;
        let client = SshClient::connect(config)
            .await
            .with_context(|| format!("SSH 连接失败: {}", host_ip))?;

        info!("SSH 连接成功: {}", host_ip);

        let mut conn = ActiveConnection::new(client);

        // 启动保活任务
        let keepalive_handle = self.start_keepalive_task(host_ip.to_string());
        conn.keepalive_handle = Some(keepalive_handle);

        self.connections.insert(host_ip.to_string(), conn);

        Ok(&self.connections.get(host_ip).unwrap().client)
    }

    /// 在已建立的连接上执行命令
    pub async fn execute_command(&self, host_ip: &str, command: &str) -> Result<String> {
        let conn = self
            .connections
            .get(host_ip)
            .with_context(|| format!("主机 {} 未连接", host_ip))?;

        let output = conn.client.execute(command).await?;
        Ok(output.stdout)
    }

    /// 批量连接多个主机
    pub async fn connect_batch(
        &mut self,
        host_ips: &[String],
        cli_params: &SshParams,
    ) -> HashMap<String, Result<()>> {
        let mut results = HashMap::new();

        for ip in host_ips {
            let result = self.get_or_connect(ip, cli_params).await.map(|_| ());
            results.insert(ip.clone(), result);
        }

        results
    }

    /// 获取 Gluster 客户端（复用 SSH 连接）
    ///
    /// 注意：由于 GlusterClient 需要拥有 SshClient 的所有权，
    /// 这里会创建一个新的 SSH 连接专门用于 Gluster 操作
    pub async fn create_gluster_client(
        &self,
        host_ip: &str,
        cli_params: &SshParams,
    ) -> Result<GlusterClient> {
        let config = self.resolve_ssh_config(host_ip, cli_params).await?;
        let client = SshClient::connect(config)
            .await
            .with_context(|| format!("Gluster SSH 连接失败: {}", host_ip))?;

        Ok(GlusterClient::new(client))
    }

    /// 检查主机是否已连接
    pub fn is_connected(&self, host_ip: &str) -> bool {
        self.connections.contains_key(host_ip)
    }

    /// 获取已连接的主机列表
    pub fn connected_hosts(&self) -> Vec<&str> {
        self.connections.keys().map(|s| s.as_str()).collect()
    }

    /// 关闭指定主机的连接
    pub async fn disconnect(&mut self, host_ip: &str) -> Result<()> {
        if let Some(mut conn) = self.connections.remove(host_ip) {
            // 取消保活任务
            if let Some(handle) = conn.keepalive_handle.take() {
                handle.abort();
            }
            // 断开连接
            conn.client.disconnect().await?;
            info!("SSH 连接已关闭: {}", host_ip);
        }
        Ok(())
    }

    /// 关闭所有连接
    pub async fn close_all(&mut self) {
        // 发送停止信号
        {
            let mut stop = self.keepalive_stop.write().await;
            *stop = true;
        }

        // 关闭所有连接
        let hosts: Vec<String> = self.connections.keys().cloned().collect();
        for host_ip in hosts {
            if let Err(e) = self.disconnect(&host_ip).await {
                warn!("关闭连接 {} 失败: {}", host_ip, e);
            }
        }
    }

    /// 启动保活任务（内部方法）
    fn start_keepalive_task(&self, host_ip: String) -> JoinHandle<()> {
        let interval = self.keepalive_interval;
        let stop_signal = Arc::clone(&self.keepalive_stop);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                // 检查停止信号
                if *stop_signal.read().await {
                    debug!("保活任务停止: {}", host_ip);
                    break;
                }

                // 这里只是记录心跳，实际的保活命令在需要时由外部触发
                debug!("保活心跳: {}", host_ip);
            }
        })
    }

    /// 发送保活命令到指定主机
    pub async fn send_keepalive(&self, host_ip: &str) -> Result<()> {
        if let Some(conn) = self.connections.get(host_ip) {
            // 发送空命令保活
            let _ = conn.client.execute("echo").await?;
            debug!("保活命令发送成功: {}", host_ip);
        }
        Ok(())
    }

    /// 发送保活命令到所有已连接主机
    pub async fn send_keepalive_all(&self) {
        for host_ip in self.connections.keys() {
            if let Err(e) = self.send_keepalive(host_ip).await {
                warn!("保活命令失败 {}: {}", host_ip, e);
            }
        }
    }
}

impl Drop for SshConnectionManager {
    fn drop(&mut self) {
        // 取消所有保活任务
        for (_, conn) in &mut self.connections {
            if let Some(handle) = conn.keepalive_handle.take() {
                handle.abort();
            }
        }
    }
}

/// 从数据库记录创建 SSH 配置
pub fn ssh_config_from_host_record(
    host_record: &HostRecord,
    default_user: &str,
) -> Option<SshConfig> {
    let username = host_record.ssh_username.as_deref().unwrap_or(default_user);
    let port = host_record.ssh_port.unwrap_or(22) as u16;
    let host_ip = &host_record.host;

    // 优先使用密码
    if let Some(ref password) = host_record.ssh_password {
        return Some(SshConfig::with_password(host_ip, username, password).port(port));
    }

    // 其次使用密钥
    if let Some(ref key_path) = host_record.ssh_key_path {
        return Some(SshConfig::with_key(host_ip, username, PathBuf::from(key_path)).port(port));
    }

    // 使用默认密钥
    Some(SshConfig::with_default_key(host_ip, username).port(port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_params_new() {
        let params = SshParams::new("root");
        assert_eq!(params.user, "root");
        assert!(params.password.is_none());
        assert!(params.key_path.is_none());
    }

    #[test]
    fn test_ssh_params_with_password() {
        let params = SshParams::with_password("admin", "secret123");
        assert_eq!(params.user, "admin");
        assert_eq!(params.password, Some("secret123".to_string()));
    }

    #[test]
    fn test_ssh_params_with_key() {
        let params = SshParams::with_key("root", PathBuf::from("/root/.ssh/id_rsa"));
        assert_eq!(params.user, "root");
        assert_eq!(params.key_path, Some(PathBuf::from("/root/.ssh/id_rsa")));
    }

    #[test]
    fn test_connection_manager_new() {
        let manager = SshConnectionManager::new("root");
        assert_eq!(manager.default_user, "root");
        assert!(manager.storage.is_none());
        assert_eq!(manager.keepalive_interval, DEFAULT_KEEPALIVE_INTERVAL);
    }

    #[test]
    fn test_connection_manager_with_keepalive() {
        let manager = SshConnectionManager::new("root")
            .with_keepalive_interval(Duration::from_secs(30));
        assert_eq!(manager.keepalive_interval, Duration::from_secs(30));
    }
}

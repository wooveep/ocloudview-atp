//! 主机连接管理

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use virt::connect::Connect;

use crate::{HostInfo, Result, TransportConfig, TransportError};

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// 未连接
    Disconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 连接失败
    Failed,
}

/// 主机连接
pub struct HostConnection {
    /// 主机信息
    host_info: HostInfo,

    /// Libvirt 连接
    connection: Arc<Mutex<Option<Connect>>>,

    /// 连接状态
    state: Arc<Mutex<ConnectionState>>,

    /// 最后活跃时间
    last_active: Arc<Mutex<chrono::DateTime<chrono::Utc>>>,

    /// 传输配置
    config: Arc<TransportConfig>,

    /// 重连尝试次数
    reconnect_attempts: Arc<RwLock<u32>>,

    /// 心跳任务取消通道
    heartbeat_shutdown: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,

    /// 监控指标
    metrics: Arc<ConnectionMetrics>,
}

/// 连接监控指标
pub struct ConnectionMetrics {
    /// 当前活跃使用数（正在使用该连接的操作数）
    active_uses: Arc<RwLock<u32>>,

    /// 总请求数
    total_requests: Arc<RwLock<u64>>,

    /// 错误计数
    error_count: Arc<RwLock<u64>>,

    /// 最后错误时间
    last_error: Arc<Mutex<Option<chrono::DateTime<chrono::Utc>>>>,

    /// 连接创建时间
    created_at: chrono::DateTime<chrono::Utc>,
}

impl ConnectionMetrics {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            active_uses: Arc::new(RwLock::new(0)),
            total_requests: Arc::new(RwLock::new(0)),
            error_count: Arc::new(RwLock::new(0)),
            last_error: Arc::new(Mutex::new(None)),
            created_at: chrono::Utc::now(),
        })
    }

    /// 获取当前活跃使用数
    pub async fn active_uses(&self) -> u32 {
        *self.active_uses.read().await
    }

    /// 获取总请求数
    pub async fn total_requests(&self) -> u64 {
        *self.total_requests.read().await
    }

    /// 获取错误计数
    pub async fn error_count(&self) -> u64 {
        *self.error_count.read().await
    }

    /// 获取最后错误时间
    pub async fn last_error(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        *self.last_error.lock().await
    }

    /// 获取连接创建时间
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }

    /// 增加请求计数
    async fn increment_request(&self) {
        *self.total_requests.write().await += 1;
    }

    /// 增加活跃使用数
    #[allow(dead_code)]
    async fn increment_active_uses(&self) {
        *self.active_uses.write().await += 1;
    }

    /// 减少活跃使用数
    #[allow(dead_code)]
    async fn decrement_active_uses(&self) {
        let mut uses = self.active_uses.write().await;
        if *uses > 0 {
            *uses -= 1;
        }
    }

    /// 记录错误
    async fn record_error(&self) {
        *self.error_count.write().await += 1;
        *self.last_error.lock().await = Some(chrono::Utc::now());
    }
}

impl HostConnection {
    /// 创建新的主机连接（使用默认配置）
    pub fn new(host_info: HostInfo) -> Self {
        Self::with_config(host_info, Arc::new(TransportConfig::default()))
    }

    /// 创建新的主机连接（指定配置）
    pub fn with_config(host_info: HostInfo, config: Arc<TransportConfig>) -> Self {
        Self {
            host_info,
            connection: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            last_active: Arc::new(Mutex::new(chrono::Utc::now())),
            config,
            reconnect_attempts: Arc::new(RwLock::new(0)),
            heartbeat_shutdown: Arc::new(Mutex::new(None)),
            metrics: ConnectionMetrics::new(),
        }
    }

    /// 连接到主机
    pub async fn connect(&self) -> Result<()> {
        info!("连接到主机: {} ({})", self.host_info.id, self.host_info.uri);

        // 设置状态为连接中
        *self.state.lock().await = ConnectionState::Connecting;

        // 连接 Libvirt（带超时）
        let timeout = self.config.connect_timeout();
        let uri = self.host_info.uri.clone();

        let conn_result = tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || Connect::open(Some(&uri)))
        ).await;

        let conn = match conn_result {
            Ok(Ok(Ok(conn))) => conn,
            Ok(Ok(Err(e))) => {
                *self.state.lock().await = ConnectionState::Failed;
                self.metrics.record_error().await;
                return Err(TransportError::ConnectionFailed(e.to_string()));
            }
            Ok(Err(_)) => {
                *self.state.lock().await = ConnectionState::Failed;
                self.metrics.record_error().await;
                return Err(TransportError::ConnectionFailed("任务失败".to_string()));
            }
            Err(_) => {
                *self.state.lock().await = ConnectionState::Failed;
                self.metrics.record_error().await;
                return Err(TransportError::Timeout);
            }
        };

        // 保存连接
        *self.connection.lock().await = Some(conn);
        *self.state.lock().await = ConnectionState::Connected;
        *self.last_active.lock().await = chrono::Utc::now();

        // 重置重连计数
        *self.reconnect_attempts.write().await = 0;

        info!("成功连接到主机: {}", self.host_info.id);

        // 启动心跳监控
        self.start_heartbeat().await;

        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&self) -> Result<()> {
        info!("断开主机连接: {}", self.host_info.id);

        // 停止心跳任务
        self.stop_heartbeat().await;

        // 关闭连接
        let mut conn_guard = self.connection.lock().await;
        if let Some(mut conn) = conn_guard.take() {
            let _ = tokio::task::spawn_blocking(move || conn.close()).await;
        }

        *self.state.lock().await = ConnectionState::Disconnected;

        Ok(())
    }

    /// 检查连接是否活跃
    pub async fn is_alive(&self) -> bool {
        let state = *self.state.lock().await;
        if state != ConnectionState::Connected {
            return false;
        }

        let conn_guard = self.connection.lock().await;
        if let Some(conn) = conn_guard.as_ref() {
            // 尝试执行一个简单的操作来验证连接
            conn.is_alive().unwrap_or(false)
        } else {
            false
        }
    }

    /// 获取连接（用于协议层）
    pub async fn get_connection(&self) -> Result<Arc<Mutex<Option<Connect>>>> {
        let state = *self.state.lock().await;
        if state != ConnectionState::Connected {
            return Err(TransportError::Disconnected);
        }

        // 更新指标
        self.metrics.increment_request().await;

        // 更新最后活跃时间
        *self.last_active.lock().await = chrono::Utc::now();

        Ok(Arc::clone(&self.connection))
    }

    /// 获取监控指标
    pub fn metrics(&self) -> Arc<ConnectionMetrics> {
        Arc::clone(&self.metrics)
    }

    /// 获取主机信息
    pub fn host_info(&self) -> &HostInfo {
        &self.host_info
    }

    /// 获取连接状态
    pub async fn state(&self) -> ConnectionState {
        *self.state.lock().await
    }

    /// 获取最后活跃时间
    pub async fn last_active(&self) -> chrono::DateTime<chrono::Utc> {
        *self.last_active.lock().await
    }

    /// 自动重连（带指数退避）
    pub async fn reconnect_with_backoff(&self) -> Result<()> {
        if !self.config.auto_reconnect {
            return Err(TransportError::ConnectionFailed("自动重连已禁用".to_string()));
        }

        let max_attempts = self.config.reconnect.max_attempts;
        let mut current_attempt = *self.reconnect_attempts.read().await;

        // 检查是否超过最大重连次数（0 表示无限重连）
        if max_attempts > 0 && current_attempt >= max_attempts {
            error!(
                "主机 {} 重连失败: 已达最大重连次数 {}",
                self.host_info.id, max_attempts
            );
            return Err(TransportError::ConnectionFailed("超过最大重连次数".to_string()));
        }

        loop {
            // 计算延迟
            let delay = self.config.reconnect.calculate_delay(current_attempt);

            warn!(
                "主机 {} 将在 {:?} 后进行第 {} 次重连...",
                self.host_info.id,
                delay,
                current_attempt + 1
            );

            sleep(delay).await;

            // 尝试连接
            match self.connect().await {
                Ok(_) => {
                    info!("主机 {} 重连成功", self.host_info.id);
                    return Ok(());
                }
                Err(e) => {
                    current_attempt += 1;
                    *self.reconnect_attempts.write().await = current_attempt;

                    error!(
                        "主机 {} 第 {} 次重连失败: {}",
                        self.host_info.id, current_attempt, e
                    );

                    // 检查是否超过最大重连次数
                    if max_attempts > 0 && current_attempt >= max_attempts {
                        return Err(TransportError::ConnectionFailed(
                            format!("重连失败，已尝试 {} 次", current_attempt)
                        ));
                    }
                }
            }
        }
    }

    /// 启动心跳监控
    async fn start_heartbeat(&self) {
        // 先停止已有的心跳任务
        self.stop_heartbeat().await;

        let interval = self.config.heartbeat_interval();
        let state = Arc::clone(&self.state);
        let connection = Arc::clone(&self.connection);
        let host_id = self.host_info.id.clone();
        let auto_reconnect = self.config.auto_reconnect;

        // 创建取消通道
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        *self.heartbeat_shutdown.lock().await = Some(tx);

        // 启动心跳任务
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        debug!("检查主机 {} 连接状态", host_id);

                        let current_state = *state.lock().await;
                        if current_state != ConnectionState::Connected {
                            debug!("主机 {} 未连接，跳过心跳检测", host_id);
                            continue;
                        }

                        // 检查连接是否存活
                        let conn_guard = connection.lock().await;
                        let is_alive = if let Some(conn) = conn_guard.as_ref() {
                            tokio::task::spawn_blocking({
                                let conn = conn.clone();
                                move || conn.is_alive().unwrap_or(false)
                            })
                            .await
                            .unwrap_or(false)
                        } else {
                            false
                        };
                        drop(conn_guard);

                        if !is_alive {
                            warn!("主机 {} 连接已断开", host_id);
                            *state.lock().await = ConnectionState::Disconnected;

                            // 如果启用了自动重连，触发重连
                            // 注意：这里我们只是标记状态，实际重连会在下次 get_connection 时触发
                            // 或者用户可以手动调用 reconnect_with_backoff
                            if auto_reconnect {
                                info!("主机 {} 连接断开，需要重连", host_id);
                            }
                        } else {
                            debug!("主机 {} 连接正常", host_id);
                        }
                    }
                    _ = &mut rx => {
                        info!("主机 {} 心跳监控已停止", host_id);
                        break;
                    }
                }
            }
        });
    }

    /// 停止心跳监控
    async fn stop_heartbeat(&self) {
        let mut shutdown_guard = self.heartbeat_shutdown.lock().await;
        if let Some(tx) = shutdown_guard.take() {
            let _ = tx.send(());
        }
    }
}

impl std::fmt::Debug for HostConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostConnection")
            .field("host_info", &self.host_info)
            .finish()
    }
}

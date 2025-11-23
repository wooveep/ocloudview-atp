//! 主机连接管理

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use virt::connect::Connect;

use crate::{HostInfo, Result, TransportError};

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
}

impl HostConnection {
    /// 创建新的主机连接
    pub fn new(host_info: HostInfo) -> Self {
        Self {
            host_info,
            connection: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            last_active: Arc::new(Mutex::new(chrono::Utc::now())),
        }
    }

    /// 连接到主机
    pub async fn connect(&self) -> Result<()> {
        info!("连接到主机: {} ({})", self.host_info.id, self.host_info.uri);

        // 设置状态为连接中
        *self.state.lock().await = ConnectionState::Connecting;

        // 连接 Libvirt
        let conn = Connect::open(&self.host_info.uri)
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        // 保存连接
        *self.connection.lock().await = Some(conn);
        *self.state.lock().await = ConnectionState::Connected;
        *self.last_active.lock().await = chrono::Utc::now();

        info!("成功连接到主机: {}", self.host_info.id);

        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&self) -> Result<()> {
        info!("断开主机连接: {}", self.host_info.id);

        let mut conn_guard = self.connection.lock().await;
        if let Some(conn) = conn_guard.take() {
            conn.close()
                .map_err(|e| TransportError::LibvirtError(e.to_string()))?;
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

        // 更新最后活跃时间
        *self.last_active.lock().await = chrono::Utc::now();

        Ok(Arc::clone(&self.connection))
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
}

impl std::fmt::Debug for HostConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostConnection")
            .field("host_info", &self.host_info)
            .finish()
    }
}

// TODO: 实现连接池和心跳检测

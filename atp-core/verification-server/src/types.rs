//! 数据类型定义

use uuid::Uuid;

// 从 atp-common 重新导出共享类型
pub use atp_common::{Event, RawInputEvent, VerifiedInputEvent, VerifyResult};

// 为了向后兼容，保留 InputEvent 别名
#[deprecated(since = "0.1.0", note = "请使用 VerifiedInputEvent")]
pub type InputEvent = VerifiedInputEvent;

/// 客户端连接信息
#[derive(Debug, Clone)]
pub struct ClientInfo {
    /// VM ID
    pub vm_id: String,

    /// 连接时间
    pub connected_at: chrono::DateTime<chrono::Utc>,

    /// 客户端地址
    pub remote_addr: Option<String>,
}

/// 待验证事件
#[derive(Debug)]
pub struct PendingEvent {
    /// 事件 ID
    pub event_id: Uuid,

    /// VM ID
    pub vm_id: String,

    /// 事件数据
    pub event: Event,

    /// 结果发送器
    pub result_tx: tokio::sync::oneshot::Sender<VerifyResult>,

    /// 创建时间
    pub created_at: tokio::time::Instant,
}

/// 客户端连接（抽象）
#[derive(Debug, Clone)]
pub enum ClientConnection {
    /// WebSocket 连接
    WebSocket {
        vm_id: String,
        addr: String,
    },

    /// TCP 连接
    Tcp {
        vm_id: String,
        addr: String,
    },
}

impl ClientConnection {
    pub fn vm_id(&self) -> &str {
        match self {
            ClientConnection::WebSocket { vm_id, .. } => vm_id,
            ClientConnection::Tcp { vm_id, .. } => vm_id,
        }
    }

    pub fn addr(&self) -> &str {
        match self {
            ClientConnection::WebSocket { addr, .. } => addr,
            ClientConnection::Tcp { addr, .. } => addr,
        }
    }
}

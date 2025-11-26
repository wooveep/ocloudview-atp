//! 客户端连接管理

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use crate::types::{ClientInfo, Event, VerifyResult};
use crate::{Result, VerificationError};

/// 客户端会话
pub struct ClientSession {
    /// 客户端信息
    pub info: ClientInfo,

    /// 发送事件到客户端的通道
    pub event_tx: mpsc::UnboundedSender<Event>,

    /// 是否已连接
    pub connected: bool,
}

/// 客户端管理器
pub struct ClientManager {
    /// VM ID -> 客户端会话
    clients: Arc<RwLock<HashMap<String, ClientSession>>>,

    /// 结果接收通道（所有客户端共享）
    result_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<VerifyResult>>>>,
    result_tx: mpsc::UnboundedSender<VerifyResult>,
}

impl ClientManager {
    /// 创建新的客户端管理器
    pub fn new() -> Self {
        let (result_tx, result_rx) = mpsc::unbounded_channel();

        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            result_rx: Arc::new(RwLock::new(Some(result_rx))),
            result_tx,
        }
    }

    /// 注册客户端
    pub async fn register_client(
        &self,
        info: ClientInfo,
    ) -> Result<mpsc::UnboundedReceiver<Event>> {
        let vm_id = info.vm_id.clone();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let session = ClientSession {
            info,
            event_tx,
            connected: true,
        };

        let mut clients = self.clients.write().await;
        if clients.contains_key(&vm_id) {
            warn!("客户端 {} 已存在，将被替换", vm_id);
        }

        clients.insert(vm_id.clone(), session);
        info!("注册客户端: {}", vm_id);

        Ok(event_rx)
    }

    /// 注销客户端
    pub async fn unregister_client(&self, vm_id: &str) {
        let mut clients = self.clients.write().await;
        if clients.remove(vm_id).is_some() {
            info!("注销客户端: {}", vm_id);
        }
    }

    /// 发送事件到指定客户端
    pub async fn send_event(&self, vm_id: &str, event: Event) -> Result<()> {
        let clients = self.clients.read().await;

        if let Some(session) = clients.get(vm_id) {
            if !session.connected {
                return Err(VerificationError::ClientNotConnected(vm_id.to_string()));
            }

            session
                .event_tx
                .send(event)
                .map_err(|_| {
                    VerificationError::ServerError(format!("发送事件到客户端 {} 失败", vm_id))
                })?;

            debug!("发送事件到客户端: {}", vm_id);
            Ok(())
        } else {
            Err(VerificationError::ClientNotConnected(vm_id.to_string()))
        }
    }

    /// 获取结果接收器（只能获取一次）
    pub async fn take_result_receiver(&self) -> Option<mpsc::UnboundedReceiver<VerifyResult>> {
        let mut rx = self.result_rx.write().await;
        rx.take()
    }

    /// 获取结果发送器（克隆用于多个客户端）
    pub fn get_result_sender(&self) -> mpsc::UnboundedSender<VerifyResult> {
        self.result_tx.clone()
    }

    /// 获取客户端列表
    pub async fn get_clients(&self) -> Vec<ClientInfo> {
        let clients = self.clients.read().await;
        clients
            .values()
            .map(|session| session.info.clone())
            .collect()
    }

    /// 检查客户端是否连接
    pub async fn is_connected(&self, vm_id: &str) -> bool {
        let clients = self.clients.read().await;
        clients
            .get(vm_id)
            .map(|session| session.connected)
            .unwrap_or(false)
    }

    /// 标记客户端为断开
    pub async fn mark_disconnected(&self, vm_id: &str) {
        let mut clients = self.clients.write().await;
        if let Some(session) = clients.get_mut(vm_id) {
            session.connected = false;
            info!("客户端 {} 已断开连接", vm_id);
        }
    }
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_manager_registration() {
        let manager = ClientManager::new();

        let info = ClientInfo {
            vm_id: "vm-123".to_string(),
            connected_at: chrono::Utc::now(),
            remote_addr: Some("192.168.1.100:5000".to_string()),
        };

        let _event_rx = manager.register_client(info).await.unwrap();

        assert!(manager.is_connected("vm-123").await);

        let clients = manager.get_clients().await;
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].vm_id, "vm-123");
    }

    #[tokio::test]
    async fn test_send_event() {
        let manager = ClientManager::new();

        let info = ClientInfo {
            vm_id: "vm-123".to_string(),
            connected_at: chrono::Utc::now(),
            remote_addr: None,
        };

        let mut event_rx = manager.register_client(info).await.unwrap();

        let event = Event {
            event_type: "test".to_string(),
            data: serde_json::json!({}),
            timestamp: 12345,
        };

        manager.send_event("vm-123", event.clone()).await.unwrap();

        let received = event_rx.recv().await.unwrap();
        assert_eq!(received.event_type, "test");
    }
}

//! 验证服务 - 事件跟踪和结果匹配

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{timeout, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::client::ClientManager;
use crate::types::{Event, PendingEvent, VerifyResult};
use crate::{Result, VerificationError};

/// 验证服务配置
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// 默认超时时间
    pub default_timeout: Duration,

    /// 事件清理间隔
    pub cleanup_interval: Duration,

    /// 最大待验证事件数
    pub max_pending_events: usize,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            cleanup_interval: Duration::from_secs(60),
            max_pending_events: 10000,
        }
    }
}

/// 验证服务
pub struct VerificationService {
    /// 客户端管理器（公开以便外部访问）
    pub client_manager: Arc<ClientManager>,

    /// 待验证事件 (event_id -> PendingEvent)
    pending_events: Arc<RwLock<HashMap<Uuid, PendingEvent>>>,

    /// 配置
    config: ServiceConfig,
}

impl VerificationService {
    /// 创建新的验证服务
    pub fn new(client_manager: Arc<ClientManager>, config: ServiceConfig) -> Self {
        let service = Self {
            client_manager,
            pending_events: Arc::new(RwLock::new(HashMap::new())),
            config,
        };

        // 启动结果处理任务
        service.spawn_result_processor();

        // 启动清理任务
        service.spawn_cleanup_task();

        service
    }

    /// 发送验证事件并等待结果
    ///
    /// # 参数
    /// - `vm_id`: 虚拟机 ID
    /// - `event`: 事件数据
    /// - `timeout_duration`: 超时时间（None 使用默认值）
    ///
    /// # 返回
    /// - `Ok(VerifyResult)`: 验证成功，返回结果
    /// - `Err(VerificationError::Timeout)`: 超时
    /// - `Err(VerificationError::ClientNotConnected)`: 客户端未连接
    pub async fn verify_event(
        &self,
        vm_id: &str,
        mut event: Event,
        timeout_duration: Option<Duration>,
    ) -> Result<VerifyResult> {
        // 生成唯一事件 ID
        let event_id = Uuid::new_v4();

        // 在事件数据中添加 event_id
        if let Some(data) = event.data.as_object_mut() {
            data.insert("event_id".to_string(), serde_json::json!(event_id.to_string()));
        }

        debug!("创建验证事件: vm_id={}, event_id={}, type={}",
               vm_id, event_id, event.event_type);

        // 创建结果通道
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        // 注册待验证事件
        let pending = PendingEvent {
            event_id,
            vm_id: vm_id.to_string(),
            event: event.clone(),
            result_tx,
            created_at: Instant::now(),
        };

        {
            let mut events = self.pending_events.write().await;

            // 检查是否超过最大数量
            if events.len() >= self.config.max_pending_events {
                return Err(VerificationError::ServerError(
                    "待验证事件过多，请稍后重试".to_string(),
                ));
            }

            events.insert(event_id, pending);
        }

        // 发送事件到客户端
        if let Err(e) = self.client_manager.send_event(vm_id, event).await {
            // 发送失败，移除待验证事件
            self.pending_events.write().await.remove(&event_id);
            return Err(e);
        }

        // 等待结果（带超时）
        let timeout_duration = timeout_duration.unwrap_or(self.config.default_timeout);

        match timeout(timeout_duration, result_rx).await {
            Ok(Ok(result)) => {
                debug!("收到验证结果: event_id={}, verified={}",
                       event_id, result.verified);
                Ok(result)
            }
            Ok(Err(_)) => {
                // 通道关闭（不应该发生）
                error!("验证结果通道意外关闭: event_id={}", event_id);
                Err(VerificationError::ServerError(
                    "结果通道关闭".to_string(),
                ))
            }
            Err(_) => {
                // 超时
                warn!("验证超时: vm_id={}, event_id={}", vm_id, event_id);
                self.pending_events.write().await.remove(&event_id);
                Err(VerificationError::Timeout)
            }
        }
    }

    /// 启动结果处理任务
    fn spawn_result_processor(&self) {
        let pending_events = self.pending_events.clone();
        let client_manager = self.client_manager.clone();

        tokio::spawn(async move {
            // 获取结果接收器
            let mut result_rx = match client_manager.take_result_receiver().await {
                Some(rx) => rx,
                None => {
                    error!("无法获取结果接收器");
                    return;
                }
            };

            info!("启动验证结果处理任务");

            // 处理接收到的结果
            while let Some(result) = result_rx.recv().await {
                debug!("处理验证结果: event_id={}", result.event_id);

                // 解析 event_id
                let event_id = match Uuid::parse_str(&result.event_id) {
                    Ok(id) => id,
                    Err(e) => {
                        error!("无效的 event_id: {}, 错误: {}", result.event_id, e);
                        continue;
                    }
                };

                // 查找并移除待验证事件
                let mut events = pending_events.write().await;
                if let Some(pending) = events.remove(&event_id) {
                    // 发送结果
                    if pending.result_tx.send(result).is_err() {
                        warn!("发送验证结果失败，接收方已关闭: event_id={}", event_id);
                    }
                } else {
                    warn!("收到未知事件的验证结果: event_id={}", event_id);
                }
            }

            info!("验证结果处理任务已停止");
        });
    }

    /// 启动清理任务（移除超时的待验证事件）
    fn spawn_cleanup_task(&self) {
        let pending_events = self.pending_events.clone();
        let cleanup_interval = self.config.cleanup_interval;
        let default_timeout = self.config.default_timeout;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);

            loop {
                interval.tick().await;

                let mut events = pending_events.write().await;
                let now = Instant::now();

                // 找出所有超时的事件
                let expired: Vec<Uuid> = events
                    .iter()
                    .filter(|(_, pending)| {
                        now.duration_since(pending.created_at) > default_timeout
                    })
                    .map(|(id, _)| *id)
                    .collect();

                // 移除超时事件
                for event_id in expired {
                    if let Some(pending) = events.remove(&event_id) {
                        warn!(
                            "清理超时事件: vm_id={}, event_id={}, age={}s",
                            pending.vm_id,
                            event_id,
                            now.duration_since(pending.created_at).as_secs()
                        );
                        // 通道会自动关闭，等待方会收到错误
                    }
                }

                if !events.is_empty() {
                    debug!("当前待验证事件数: {}", events.len());
                }
            }
        });
    }

    /// 获取待验证事件数量
    pub async fn pending_count(&self) -> usize {
        self.pending_events.read().await.len()
    }

    /// 取消待验证事件
    pub async fn cancel_event(&self, event_id: Uuid) -> bool {
        self.pending_events.write().await.remove(&event_id).is_some()
    }

    /// 取消指定 VM 的所有待验证事件
    pub async fn cancel_vm_events(&self, vm_id: &str) -> usize {
        let mut events = self.pending_events.write().await;
        let to_remove: Vec<Uuid> = events
            .iter()
            .filter(|(_, pending)| pending.vm_id == vm_id)
            .map(|(id, _)| *id)
            .collect();

        let count = to_remove.len();
        for event_id in to_remove {
            events.remove(&event_id);
        }

        if count > 0 {
            info!("取消 VM {} 的 {} 个待验证事件", vm_id, count);
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ClientInfo;

    #[tokio::test]
    async fn test_verification_service() {
        let client_manager = Arc::new(ClientManager::new());
        let config = ServiceConfig {
            default_timeout: Duration::from_millis(100),
            cleanup_interval: Duration::from_secs(1),
            max_pending_events: 100,
        };

        let service = VerificationService::new(client_manager.clone(), config);

        // 注册客户端
        let info = ClientInfo {
            vm_id: "vm-test".to_string(),
            connected_at: chrono::Utc::now(),
            remote_addr: None,
        };

        let _event_rx = client_manager.register_client(info).await.unwrap();

        // 测试超时
        let event = Event {
            event_type: "test".to_string(),
            data: serde_json::json!({}),
            timestamp: 12345,
        };

        let result = service.verify_event("vm-test", event, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerificationError::Timeout));
    }

    #[tokio::test]
    async fn test_pending_count() {
        let client_manager = Arc::new(ClientManager::new());
        let service = VerificationService::new(client_manager, ServiceConfig::default());

        assert_eq!(service.pending_count().await, 0);
    }
}

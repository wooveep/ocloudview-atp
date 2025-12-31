//! 验证服务 - 事件跟踪和结果匹配
//!
//! 新架构：
//! 1. 输入注入：通过 SPICE/QMP 向 VM 注入输入事件（外部系统负责）
//! 2. 事件上报：Guest Agent 捕获实际输入并上报 RawInputEvent
//! 3. 事件验证：服务端比对"期望注入的事件"与"实际上报的事件"

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::client::ClientManager;
use crate::types::{PendingEvent, RawInputEvent, VerifyResult};
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

/// 期望输入事件（等待 Agent 上报匹配）
#[derive(Debug)]
pub struct ExpectedInputEvent {
    /// 事件 ID
    pub event_id: Uuid,

    /// VM ID
    pub vm_id: String,

    /// 期望的事件类型 ("keyboard" 或 "mouse")
    pub event_type: String,

    /// 期望的按键名称或鼠标操作
    pub expected_name: String,

    /// 期望的 value（1=按下，0=释放，None=任意）
    pub expected_value: Option<i32>,

    /// 结果发送器
    pub result_tx: tokio::sync::oneshot::Sender<VerifyResult>,

    /// 创建时间
    pub created_at: Instant,

    /// 超时时间
    pub timeout: Duration,
}

/// 验证服务
pub struct VerificationService {
    /// 客户端管理器（公开以便外部访问）
    pub client_manager: Arc<ClientManager>,

    /// 待验证事件 (event_id -> PendingEvent) - 用于旧模式兼容
    #[allow(dead_code)]
    pending_events: Arc<RwLock<HashMap<Uuid, PendingEvent>>>,

    /// 期望输入事件列表 (vm_id -> Vec<ExpectedInputEvent>)
    expected_events: Arc<RwLock<HashMap<String, Vec<ExpectedInputEvent>>>>,

    /// 配置
    config: ServiceConfig,
}

impl VerificationService {
    /// 创建新的验证服务
    pub fn new(client_manager: Arc<ClientManager>, config: ServiceConfig) -> Self {
        let service = Self {
            client_manager,
            pending_events: Arc::new(RwLock::new(HashMap::new())),
            expected_events: Arc::new(RwLock::new(HashMap::new())),
            config,
        };

        // 启动结果处理任务（用于旧模式兼容）
        service.spawn_result_processor();

        // 启动原始输入事件处理任务（新模式）
        service.spawn_raw_input_processor();

        // 启动清理任务
        service.spawn_cleanup_task();

        service
    }

    /// 注册期望输入事件并等待匹配
    ///
    /// 调用此方法后，应通过 SPICE/QMP 向 VM 注入对应的输入事件。
    /// 当 Guest Agent 上报匹配的 RawInputEvent 时，返回验证结果。
    ///
    /// # 参数
    /// - `vm_id`: 虚拟机 ID
    /// - `event_type`: 事件类型 ("keyboard" 或 "mouse")
    /// - `expected_name`: 期望的按键名称（如 "A", "LEFT" 等）
    /// - `expected_value`: 期望的 value（1=按下，0=释放，None=任意）
    /// - `timeout_duration`: 超时时间
    ///
    /// # 返回
    /// - `Ok(VerifyResult)`: 验证成功
    /// - `Err(VerificationError::Timeout)`: 超时
    pub async fn expect_input(
        &self,
        vm_id: &str,
        event_type: &str,
        expected_name: &str,
        expected_value: Option<i32>,
        timeout_duration: Option<Duration>,
    ) -> Result<VerifyResult> {
        let event_id = Uuid::new_v4();
        let timeout = timeout_duration.unwrap_or(self.config.default_timeout);

        debug!(
            "注册期望输入: vm_id={}, event_id={}, type={}, name={}, value={:?}",
            vm_id, event_id, event_type, expected_name, expected_value
        );

        // 创建结果通道
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        // 创建期望事件
        let expected = ExpectedInputEvent {
            event_id,
            vm_id: vm_id.to_string(),
            event_type: event_type.to_string(),
            expected_name: expected_name.to_uppercase(),
            expected_value,
            result_tx,
            created_at: Instant::now(),
            timeout,
        };

        // 注册期望事件
        {
            let mut events = self.expected_events.write().await;
            events
                .entry(vm_id.to_string())
                .or_insert_with(Vec::new)
                .push(expected);
        }

        // 等待结果（带超时）
        match tokio::time::timeout(timeout, result_rx).await {
            Ok(Ok(result)) => {
                debug!(
                    "输入验证成功: event_id={}, verified={}, latency={}ms",
                    event_id, result.verified, result.latency_ms
                );
                Ok(result)
            }
            Ok(Err(_)) => {
                // 通道关闭（被清理任务移除）
                warn!("输入验证超时（通道关闭）: event_id={}", event_id);
                Err(VerificationError::Timeout)
            }
            Err(_) => {
                // 超时，移除期望事件
                warn!("输入验证超时: vm_id={}, event_id={}", vm_id, event_id);
                self.remove_expected_event(vm_id, event_id).await;
                Err(VerificationError::Timeout)
            }
        }
    }

    /// 移除期望事件
    async fn remove_expected_event(&self, vm_id: &str, event_id: Uuid) {
        let mut events = self.expected_events.write().await;
        if let Some(vm_events) = events.get_mut(vm_id) {
            vm_events.retain(|e| e.event_id != event_id);
            if vm_events.is_empty() {
                events.remove(vm_id);
            }
        }
    }

    /// 启动原始输入事件处理任务
    fn spawn_raw_input_processor(&self) {
        let expected_events = self.expected_events.clone();
        let client_manager = self.client_manager.clone();

        tokio::spawn(async move {
            // 获取原始输入事件接收器
            let mut raw_rx = match client_manager.take_raw_input_event_receiver().await {
                Some(rx) => rx,
                None => {
                    error!("无法获取原始输入事件接收器");
                    return;
                }
            };

            info!("启动原始输入事件处理任务");

            // 处理接收到的原始输入事件
            while let Some((vm_id, raw_event)) = raw_rx.recv().await {
                debug!(
                    "处理原始输入事件: vm_id={}, type={}, name={}, value={}",
                    vm_id, raw_event.event_type, raw_event.name, raw_event.value
                );

                // 查找匹配的期望事件
                let mut events = expected_events.write().await;
                if let Some(vm_events) = events.get_mut(&vm_id) {
                    // 查找第一个匹配的期望事件
                    let matched_idx = vm_events.iter().position(|expected| {
                        Self::match_event(expected, &raw_event)
                    });

                    if let Some(idx) = matched_idx {
                        // 移除并获取匹配的期望事件
                        let expected = vm_events.remove(idx);
                        let latency_ms = expected.created_at.elapsed().as_millis() as u64;

                        info!(
                            "输入事件匹配成功: vm_id={}, event_id={}, name={}, latency={}ms",
                            vm_id, expected.event_id, raw_event.name, latency_ms
                        );

                        // 创建验证结果
                        let result = VerifyResult {
                            message_type: "verify_result".to_string(),
                            event_id: expected.event_id.to_string(),
                            verified: true,
                            timestamp: chrono::Utc::now().timestamp_millis(),
                            latency_ms,
                            details: serde_json::json!({
                                "matched_name": raw_event.name,
                                "matched_value": raw_event.value,
                                "matched_code": raw_event.code,
                            }),
                        };

                        // 发送结果
                        let _ = expected.result_tx.send(result);

                        // 清理空的 VM 条目
                        if vm_events.is_empty() {
                            events.remove(&vm_id);
                        }
                    } else {
                        debug!(
                            "未匹配的输入事件: vm_id={}, name={} (期望数: {})",
                            vm_id, raw_event.name, vm_events.len()
                        );
                    }
                }
            }

            info!("原始输入事件处理任务已停止");
        });
    }

    /// 匹配期望事件与实际事件
    fn match_event(expected: &ExpectedInputEvent, actual: &RawInputEvent) -> bool {
        // 事件类型必须匹配
        if expected.event_type != actual.event_type {
            return false;
        }

        // 按键名称匹配（不区分大小写）
        if expected.expected_name != actual.name.to_uppercase() {
            return false;
        }

        // 如果指定了 value，则必须匹配
        if let Some(expected_value) = expected.expected_value {
            if expected_value != actual.value {
                return false;
            }
        }

        true
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
    async fn test_expect_input_timeout() {
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

        // 测试超时（无匹配事件上报时应超时）
        let result = service
            .expect_input("vm-test", "keyboard", "A", Some(1), Some(Duration::from_millis(50)))
            .await;

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

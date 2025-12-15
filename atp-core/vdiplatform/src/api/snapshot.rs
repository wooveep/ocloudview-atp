//! 快照管理 API
//!
//! 提供虚拟机快照管理功能，包括：
//! - 创建快照
//! - 删除快照
//! - 使用快照恢复
//! - 查询快照列表

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;

/// 快照管理 API
pub struct SnapshotApi<'a> {
    client: &'a VdiClient,
}

impl<'a> SnapshotApi<'a> {
    /// 创建新的快照 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    /// 获取虚拟机所有快照
    pub async fn list(&self, domain_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("获取虚拟机快照列表: {}", domain_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/snapshot", domain_id),
            None::<()>,
        ).await
    }

    /// 创建快照
    ///
    /// # Arguments
    /// * `domain_id` - 虚拟机 ID
    /// * `name` - 快照名称
    /// * `description` - 快照描述 (可选)
    pub async fn create(&self, domain_id: &str, name: &str, description: Option<&str>) -> Result<serde_json::Value> {
        info!("创建虚拟机快照: {} -> {}", domain_id, name);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/snapshot", domain_id),
            Some(serde_json::json!({
                "name": name,
                "description": description,
            })),
        ).await
    }

    /// 获取单个快照详情
    pub async fn get(&self, snapshot_id: &str) -> Result<serde_json::Value> {
        info!("获取快照详情: {}", snapshot_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/snapshot/{}", snapshot_id),
            None::<()>,
        ).await
    }

    /// 删除快照
    pub async fn delete(&self, snapshot_id: &str) -> Result<()> {
        info!("删除快照: {}", snapshot_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/snapshot/{}", snapshot_id),
            None::<()>,
        ).await
    }

    /// 使用快照恢复
    pub async fn restore(&self, snapshot_id: &str) -> Result<()> {
        info!("使用快照恢复: {}", snapshot_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/snapshot/{}/restore", snapshot_id),
            None::<()>,
        ).await
    }
}

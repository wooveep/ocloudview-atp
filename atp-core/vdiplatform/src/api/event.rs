//! 事件管理 API
//!
//! 提供事件管理功能，包括：
//! - 查询事件列表（分页）
//! - 获取事件详情
//! - 事件类型查询
//! - 异步任务状态跟踪

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;

/// 事件管理 API
pub struct EventApi<'a> {
    client: &'a VdiClient,
}

impl<'a> EventApi<'a> {
    /// 创建新的事件 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    // ============================================
    // 事件查询
    // ============================================

    /// 查询事件列表（分页）
    pub async fn list(&self, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询事件列表: 第{}页, 每页{}条", page_num, page_size);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/event?pageNum={}&pageSize={}", page_num, page_size),
            None::<()>,
        ).await
    }

    /// 查询事件列表（带过滤条件）
    pub async fn list_with_filter(
        &self,
        page_num: u32,
        page_size: u32,
        event_type: Option<&str>,
        status: Option<&str>,
        start_time: Option<&str>,
        end_time: Option<&str>,
    ) -> Result<serde_json::Value> {
        info!("查询事件列表（带过滤）: 第{}页", page_num);

        let mut url = format!("/ocloud/v1/event?pageNum={}&pageSize={}", page_num, page_size);

        if let Some(t) = event_type {
            url.push_str(&format!("&eventType={}", t));
        }
        if let Some(s) = status {
            url.push_str(&format!("&status={}", s));
        }
        if let Some(st) = start_time {
            url.push_str(&format!("&startTime={}", st));
        }
        if let Some(et) = end_time {
            url.push_str(&format!("&endTime={}", et));
        }

        self.client.request(
            Method::GET,
            &url,
            None::<()>,
        ).await
    }

    /// 获取事件详情
    pub async fn get(&self, event_id: &str) -> Result<serde_json::Value> {
        info!("获取事件详情: {}", event_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/event/{}", event_id),
            None::<()>,
        ).await
    }

    /// 获取事件状态
    ///
    /// 用于跟踪异步任务的执行状态
    pub async fn get_status(&self, event_id: &str) -> Result<serde_json::Value> {
        info!("获取事件状态: {}", event_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/event/{}/status", event_id),
            None::<()>,
        ).await
    }

    /// 查询事件类型列表
    pub async fn list_types(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询事件类型列表");
        self.client.request(
            Method::GET,
            "/ocloud/v1/event/types",
            None::<()>,
        ).await
    }

    /// 取消事件/任务
    pub async fn cancel(&self, event_id: &str) -> Result<()> {
        info!("取消事件: {}", event_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/event/{}/cancel", event_id),
            None::<()>,
        ).await
    }

    /// 删除事件记录
    pub async fn delete(&self, event_id: &str) -> Result<()> {
        info!("删除事件记录: {}", event_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/event/{}", event_id),
            None::<()>,
        ).await
    }

    /// 批量删除事件记录
    pub async fn batch_delete(&self, event_ids: Vec<String>) -> Result<()> {
        info!("批量删除事件记录: {} 个", event_ids.len());
        self.client.request(
            Method::POST,
            "/ocloud/v1/event/batch-delete",
            Some(serde_json::json!({ "eventIds": event_ids })),
        ).await
    }

    /// 清理历史事件记录
    ///
    /// # Arguments
    /// * `days` - 清理多少天之前的事件记录
    pub async fn cleanup(&self, days: u32) -> Result<()> {
        info!("清理 {} 天前的事件记录", days);
        self.client.request(
            Method::POST,
            "/ocloud/v1/event/cleanup",
            Some(serde_json::json!({ "days": days })),
        ).await
    }
}

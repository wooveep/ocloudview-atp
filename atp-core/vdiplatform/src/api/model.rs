//! 模板管理 API
//!
//! 提供模板管理功能，包括：
//! - 基本操作：查询模板列表、模板详情
//! - 克隆操作：从模板批量创建虚拟机（链接克隆）
//! - 转换操作：模板转虚拟机

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;
use crate::models::{Model, ModelCloneRequest, EventIdResponse};

/// 模板管理 API
pub struct ModelApi<'a> {
    client: &'a VdiClient,
}

impl<'a> ModelApi<'a> {
    /// 创建新的模板 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    /// 查询模板列表
    pub async fn list(&self) -> Result<Vec<Model>> {
        info!("查询模板列表");
        self.client.request(
            Method::GET,
            "/ocloud/v1/model",
            None::<()>,
        ).await
    }

    /// 查询模板详情
    pub async fn get(&self, model_id: &str) -> Result<Model> {
        info!("查询模板详情: {}", model_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/model/{}", model_id),
            None::<()>,
        ).await
    }

    // ============================================
    // 克隆操作
    // ============================================

    /// 从模板批量创建虚拟机（链接克隆）
    ///
    /// 使用链接克隆方式从模板快速创建多个虚拟机。
    /// 链接克隆只复制差异数据，创建速度快，但依赖源模板。
    ///
    /// # Arguments
    /// * `model_id` - 源模板 ID
    /// * `req` - 克隆请求，包含名称前缀和数量
    ///
    /// # Returns
    /// 返回事件 ID，可用于跟踪异步任务状态
    ///
    /// # Example
    /// ```ignore
    /// // 从模板创建 10 个虚拟机
    /// let req = ModelCloneRequest::new("win10-desktop-".into(), 10);
    /// let response = client.model().batch_clone("template-id", req).await?;
    /// println!("任务 ID: {:?}", response.event_id);
    ///
    /// // 指定存储池
    /// let req = ModelCloneRequest::new("vm-".into(), 5)
    ///     .with_storage_pools(vec!["pool-1".into(), "pool-2".into()]);
    /// let response = client.model().batch_clone("template-id", req).await?;
    /// ```
    pub async fn batch_clone(&self, model_id: &str, req: ModelCloneRequest) -> Result<EventIdResponse> {
        info!("从模板批量创建虚拟机: 模板={}, 名称前缀={}, 数量={}",
            model_id, req.name, req.amount
        );
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/model/{}/clone", model_id),
            Some(req),
        ).await
    }

    /// 模板转虚拟机
    ///
    /// 将模板转换为普通虚拟机。转换后模板将变为可运行的虚拟机。
    ///
    /// # Arguments
    /// * `model_id` - 模板 ID
    ///
    /// # Warning
    /// 此操作会将模板转换为虚拟机，原模板将不再可用作模板。
    pub async fn to_domain(&self, model_id: &str) -> Result<()> {
        info!("模板转虚拟机: {}", model_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/model/{}/to-domain", model_id),
            None::<()>,
        ).await
    }

    /// 克隆模板
    ///
    /// 完全克隆一个模板，创建独立的新模板。
    ///
    /// # Arguments
    /// * `model_id` - 源模板 ID
    /// * `name` - 新模板名称
    ///
    /// # Returns
    /// 返回事件 ID，可用于跟踪异步任务状态
    pub async fn clone(&self, model_id: &str, name: &str) -> Result<EventIdResponse> {
        info!("克隆模板: {} -> {}", model_id, name);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/model/{}/clone-model", model_id),
            Some(serde_json::json!({ "name": name })),
        ).await
    }
}

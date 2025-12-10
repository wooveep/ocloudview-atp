//! 模板管理 API

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;
use crate::models::Model;

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
}

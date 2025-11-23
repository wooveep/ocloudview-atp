//! 桌面池管理 API

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;
use crate::models::{DeskPool, CreateDeskPoolRequest, Domain};

/// 桌面池管理 API
pub struct DeskPoolApi<'a> {
    client: &'a VdiClient,
}

impl<'a> DeskPoolApi<'a> {
    /// 创建新的桌面池 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    /// 创建桌面池
    pub async fn create(&self, req: CreateDeskPoolRequest) -> Result<DeskPool> {
        info!("创建桌面池: {}", req.name);
        self.client.request(
            Method::POST,
            "/ocloud/v1/desk-pool",
            Some(req),
        ).await
    }

    /// 查询桌面池详情
    pub async fn get(&self, pool_id: &str) -> Result<DeskPool> {
        info!("查询桌面池详情: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/desk-pool/{}", pool_id),
            None::<()>,
        ).await
    }

    /// 启用桌面池
    pub async fn enable(&self, pool_id: &str) -> Result<()> {
        info!("启用桌面池: {}", pool_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/enable", pool_id),
            None::<()>,
        ).await
    }

    /// 禁用桌面池
    pub async fn disable(&self, pool_id: &str) -> Result<()> {
        info!("禁用桌面池: {}", pool_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/disable", pool_id),
            None::<()>,
        ).await
    }

    /// 激活桌面池
    pub async fn activate(&self, pool_id: &str) -> Result<()> {
        info!("激活桌面池: {}", pool_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/active", pool_id),
            None::<()>,
        ).await
    }

    /// 删除桌面池
    pub async fn delete(&self, pool_id: &str) -> Result<()> {
        info!("删除桌面池: {}", pool_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/desk-pool/{}", pool_id),
            None::<()>,
        ).await
    }

    /// 获取桌面池中的虚拟机列表
    pub async fn list_domains(&self, pool_id: &str) -> Result<Vec<Domain>> {
        info!("获取桌面池虚拟机列表: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/desk-pool/{}/domain/list", pool_id),
            None::<()>,
        ).await
    }

    /// 切换桌面池模板
    pub async fn switch_model(&self, pool_id: &str, model_id: &str) -> Result<()> {
        info!("切换桌面池模板: {} -> {}", pool_id, model_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/desk-pool/switch-model",
            Some(serde_json::json!({
                "pool_id": pool_id,
                "model_id": model_id,
            })),
        ).await
    }
}

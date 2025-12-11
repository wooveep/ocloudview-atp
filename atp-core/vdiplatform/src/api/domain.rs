//! 虚拟机管理 API

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;
use crate::models::{Domain, CreateDomainRequest};

/// 虚拟机管理 API
pub struct DomainApi<'a> {
    client: &'a VdiClient,
}

impl<'a> DomainApi<'a> {
    /// 创建新的虚拟机 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    /// 查询虚拟机列表(支持分页)
    ///
    /// # Arguments
    /// * `page_num` - 页码(从1开始)
    /// * `page_size` - 每页数量
    pub async fn list_paged(&self, page_num: u32, page_size: u32) -> Result<Vec<serde_json::Value>> {
        info!("查询虚拟机列表: 第{}页, 每页{}条", page_num, page_size);

        let url = format!("/ocloud/v1/domain?pageNum={}&pageSize={}", page_num, page_size);
        let token = self.client.get_token().await?;

        let response: serde_json::Value = self.client.http_client()
            .get(&format!("{}{}", self.client.base_url(), url))
            .header("Token", &token)
            .send()
            .await
            .map_err(|e| crate::error::VdiError::HttpError(e.to_string()))?
            .json()
            .await
            .map_err(|e| crate::error::VdiError::ParseError(e.to_string()))?;

        if response["status"].as_i64().unwrap_or(-1) != 0 {
            let msg = response["msg"].as_str().unwrap_or("未知错误");
            return Err(crate::error::VdiError::ApiError(500, msg.to_string()));
        }

        Ok(response["data"]["list"]
            .as_array()
            .unwrap_or(&vec![])
            .clone())
    }

    /// 查询所有虚拟机(自动处理分页)
    pub async fn list_all(&self) -> Result<Vec<serde_json::Value>> {
        self.list_paged(1, 1000).await
    }

    /// 创建虚拟机
    pub async fn create(&self, req: CreateDomainRequest) -> Result<Domain> {
        info!("创建虚拟机: {}", req.name);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain",
            Some(req),
        ).await
    }

    /// 查询虚拟机详情
    pub async fn get(&self, domain_id: &str) -> Result<Domain> {
        info!("查询虚拟机详情: {}", domain_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}", domain_id),
            None::<()>,
        ).await
    }

    /// 启动虚拟机
    pub async fn start(&self, domain_id: &str) -> Result<()> {
        info!("启动虚拟机: {}", domain_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/start",
            Some(serde_json::json!({ "domain_id": domain_id })),
        ).await
    }

    /// 关闭虚拟机
    pub async fn shutdown(&self, domain_id: &str) -> Result<()> {
        info!("关闭虚拟机: {}", domain_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/close",
            Some(serde_json::json!({ "domain_id": domain_id })),
        ).await
    }

    /// 重启虚拟机
    pub async fn reboot(&self, domain_id: &str) -> Result<()> {
        info!("重启虚拟机: {}", domain_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/reboot",
            Some(serde_json::json!({ "domain_id": domain_id })),
        ).await
    }

    /// 删除虚拟机
    pub async fn delete(&self, domain_id: &str) -> Result<()> {
        info!("删除虚拟机: {}", domain_id);
        self.client.request(
            Method::DELETE,
            "/ocloud/v1/domain/delete",
            Some(serde_json::json!({ "domain_id": domain_id })),
        ).await
    }

    /// 暂停虚拟机
    pub async fn suspend(&self, domain_id: &str) -> Result<()> {
        info!("暂停虚拟机: {}", domain_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/suspend",
            Some(serde_json::json!({ "domain_id": domain_id })),
        ).await
    }

    /// 恢复虚拟机
    pub async fn resume(&self, domain_id: &str) -> Result<()> {
        info!("恢复虚拟机: {}", domain_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/resume",
            Some(serde_json::json!({ "domain_id": domain_id })),
        ).await
    }

    /// 绑定用户
    pub async fn bind_user(&self, domain_id: &str, user_id: &str) -> Result<()> {
        info!("绑定用户到虚拟机: {} -> {}", user_id, domain_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/bind-user",
            Some(serde_json::json!({
                "domain_id": domain_id,
                "user_id": user_id,
            })),
        ).await
    }

    /// 解绑用户
    pub async fn unbind_user(&self, domain_id: &str, user_id: &str) -> Result<()> {
        info!("解绑用户从虚拟机: {} <- {}", user_id, domain_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/unbind-user",
            Some(serde_json::json!({
                "domain_id": domain_id,
                "user_id": user_id,
            })),
        ).await
    }
}


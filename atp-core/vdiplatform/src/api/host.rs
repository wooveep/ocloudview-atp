//! 主机管理 API

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;
use crate::models::{Host, HostStatus};

/// 主机管理 API
pub struct HostApi<'a> {
    client: &'a VdiClient,
}

impl<'a> HostApi<'a> {
    /// 创建新的主机 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    /// 查询主机列表(支持分页)
    ///
    /// # Arguments
    /// * `page_num` - 页码(从1开始)
    /// * `page_size` - 每页数量
    pub async fn list_paged(&self, page_num: u32, page_size: u32) -> Result<Vec<serde_json::Value>> {
        info!("查询主机列表: 第{}页, 每页{}条", page_num, page_size);

        let url = format!("/ocloud/v1/host?pageNum={}&pageSize={}", page_num, page_size);
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

    /// 查询所有主机
    pub async fn list_all(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询所有主机");

        let url = "/ocloud/v1/host/all";
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

        Ok(response["data"]
            .as_array()
            .unwrap_or(&vec![])
            .clone())
    }

    /// 根据资源池 ID 查询主机列表
    pub async fn list_by_pool_id(&self, pool_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("根据资源池查询主机列表: {}", pool_id);

        let url = format!("/ocloud/v1/host/all?poolId={}", pool_id);
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

        Ok(response["data"]
            .as_array()
            .unwrap_or(&vec![])
            .clone())
    }

    /// 查询主机列表
    pub async fn list(&self) -> Result<Vec<Host>> {
        info!("查询主机列表");
        self.client.request(
            Method::GET,
            "/ocloud/v1/host",
            None::<()>,
        ).await
    }

    /// 查询主机详情
    pub async fn get(&self, host_id: &str) -> Result<Host> {
        info!("查询主机详情: {}", host_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/host/{}", host_id),
            None::<()>,
        ).await
    }

    /// 查询主机状态
    pub async fn get_status(&self, host_id: &str) -> Result<HostStatus> {
        info!("查询主机状态: {}", host_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/host/{}/status", host_id),
            None::<()>,
        ).await
    }

    /// 查询主机运行时间
    pub async fn get_uptime(&self, host_id: &str) -> Result<u64> {
        info!("查询主机运行时间: {}", host_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/host/{}/uptime", host_id),
            None::<()>,
        ).await
    }
}


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

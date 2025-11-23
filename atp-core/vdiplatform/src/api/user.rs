//! 用户管理 API

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;
use crate::models::User;

/// 用户管理 API
pub struct UserApi<'a> {
    client: &'a VdiClient,
}

impl<'a> UserApi<'a> {
    /// 创建新的用户 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    /// 查询用户列表
    pub async fn list(&self) -> Result<Vec<User>> {
        info!("查询用户列表");
        self.client.request(
            Method::GET,
            "/ocloud/v1/user",
            None::<()>,
        ).await
    }

    /// 查询用户详情
    pub async fn get(&self, user_id: &str) -> Result<User> {
        info!("查询用户详情: {}", user_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/user/{}", user_id),
            None::<()>,
        ).await
    }
}

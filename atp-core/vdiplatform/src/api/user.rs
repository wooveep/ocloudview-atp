//! 用户管理 API

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::{Result, VdiError};
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

    /// 按组织单位查询用户列表
    ///
    /// # Arguments
    /// * `distinguished_name` - 组织单位的 distinguishedName，如 "ou=研发部,ou=公司"
    ///
    /// # Example
    /// ```ignore
    /// // 查询研发部的所有用户
    /// let users = client.user().list_by_group("ou=研发部,ou=公司").await?;
    /// ```
    pub async fn list_by_group(&self, distinguished_name: &str) -> Result<Vec<User>> {
        info!("按组织单位查询用户: {}", distinguished_name);

        let url = format!(
            "/ocloud/v1/user?group={}&pageNum=1&pageSize=1000",
            urlencoding::encode(distinguished_name)
        );
        let token = self.client.get_token().await?;

        let response: serde_json::Value = self
            .client
            .http_client()
            .get(&format!("{}{}", self.client.base_url(), url))
            .header("Token", &token)
            .send()
            .await
            .map_err(|e| VdiError::HttpError(e.to_string()))?
            .json()
            .await
            .map_err(|e| VdiError::ParseError(e.to_string()))?;

        if response["status"].as_i64().unwrap_or(-1) != 0 {
            let msg = response["msg"].as_str().unwrap_or("未知错误");
            return Err(VdiError::ApiError(500, msg.to_string()));
        }

        // 解析用户列表
        let users: Vec<User> = response["data"]["list"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();

        info!("找到 {} 个用户", users.len());
        Ok(users)
    }

    /// 按用户名列表查询用户
    ///
    /// 从所有用户中筛选出指定用户名的用户
    ///
    /// # Arguments
    /// * `usernames` - 用户名列表
    ///
    /// # Example
    /// ```ignore
    /// let users = client.user().find_by_usernames(&["user1".into(), "user2".into()]).await?;
    /// ```
    pub async fn find_by_usernames(&self, usernames: &[String]) -> Result<Vec<User>> {
        info!("按用户名列表查询用户: {:?}", usernames);
        let all_users = self.list().await?;
        let matched: Vec<User> = all_users
            .into_iter()
            .filter(|u| usernames.contains(&u.username))
            .collect();
        info!("匹配到 {} 个用户", matched.len());
        Ok(matched)
    }

    /// 分页查询用户列表
    ///
    /// # Arguments
    /// * `page_num` - 页码（从 1 开始）
    /// * `page_size` - 每页数量
    pub async fn list_paged(&self, page_num: u32, page_size: u32) -> Result<Vec<User>> {
        info!("分页查询用户列表: 第{}页, 每页{}条", page_num, page_size);

        let url = format!(
            "/ocloud/v1/user?pageNum={}&pageSize={}",
            page_num, page_size
        );
        let token = self.client.get_token().await?;

        let response: serde_json::Value = self
            .client
            .http_client()
            .get(&format!("{}{}", self.client.base_url(), url))
            .header("Token", &token)
            .send()
            .await
            .map_err(|e| VdiError::HttpError(e.to_string()))?
            .json()
            .await
            .map_err(|e| VdiError::ParseError(e.to_string()))?;

        if response["status"].as_i64().unwrap_or(-1) != 0 {
            let msg = response["msg"].as_str().unwrap_or("未知错误");
            return Err(VdiError::ApiError(500, msg.to_string()));
        }

        let users: Vec<User> = response["data"]["list"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();

        Ok(users)
    }
}

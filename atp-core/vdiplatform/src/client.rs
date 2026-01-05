//! VDI 平台客户端核心实现

use std::sync::Arc;
use tokio::sync::RwLock;
use reqwest::{Client, Method};
use serde::{Serialize, de::DeserializeOwned};
use tracing::{debug, info, warn};

use crate::error::{VdiError, Result};
use crate::api::{
    DomainApi, DeskPoolApi, HostApi, ModelApi, UserApi, GroupApi,
    SnapshotApi, StorageApi, NetworkApi, EventApi, RecycleApi,
};

/// VDI 平台客户端配置
#[derive(Debug, Clone)]
pub struct VdiConfig {
    /// 连接超时（秒）
    pub connect_timeout: u64,

    /// 请求超时（秒）
    pub request_timeout: u64,

    /// 最大重试次数
    pub max_retries: u32,

    /// 是否验证 SSL 证书
    pub verify_ssl: bool,
}

impl Default for VdiConfig {
    fn default() -> Self {
        Self {
            connect_timeout: 10,
            request_timeout: 30,
            max_retries: 3,
            verify_ssl: true,
        }
    }
}

/// VDI 平台客户端
pub struct VdiClient {
    /// API 基础 URL
    base_url: String,

    /// HTTP 客户端
    http_client: Client,

    /// 认证令牌
    access_token: Arc<RwLock<Option<String>>>,

    /// 配置
    config: VdiConfig,
}

impl VdiClient {
    /// 创建新的 VDI 客户端
    pub fn new(base_url: &str, config: VdiConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.request_timeout))
            .connect_timeout(std::time::Duration::from_secs(config.connect_timeout))
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()
            .map_err(|e| VdiError::HttpError(e.to_string()))?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http_client,
            access_token: Arc::new(RwLock::new(None)),
            config,
        })
    }

    /// 认证登录
    ///
    /// # Arguments
    /// * `username` - 用户名
    /// * `password` - 明文密码(将自动转换为MD5)
    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        info!("VDI 客户端登录: {}", username);

        // 将密码转换为 MD5
        let password_md5 = format!("{:x}", md5::compute(password.as_bytes()));

        let login_url = format!("{}/ocloud/v1/login", self.base_url);
        let login_data = serde_json::json!({
            "username": username,
            "password": password_md5,
            "client": ""
        });

        let response = self.http_client
            .post(&login_url)
            .json(&login_data)
            .send()
            .await
            .map_err(|e| VdiError::HttpError(e.to_string()))?;

        let login_result: serde_json::Value = response.json().await
            .map_err(|e| VdiError::ParseError(e.to_string()))?;

        // 检查登录状态
        if login_result["status"].as_i64().unwrap_or(-1) != 0 {
            let msg = login_result["msg"].as_str().unwrap_or("未知错误");
            return Err(VdiError::AuthError(format!("VDI 登录失败: {}", msg)));
        }

        // 提取 token
        let token = login_result["data"]["token"]
            .as_str()
            .ok_or_else(|| VdiError::AuthError("未获取到 Token".to_string()))?
            .to_string();

        *self.access_token.write().await = Some(token);

        info!("VDI 客户端登录成功");
        Ok(())
    }

    /// 注销登出
    pub async fn logout(&mut self) -> Result<()> {
        info!("VDI 客户端登出");
        *self.access_token.write().await = None;
        Ok(())
    }

    /// 获取虚拟机管理 API
    pub fn domain(&self) -> DomainApi<'_> {
        DomainApi::new(self)
    }

    /// 获取桌面池管理 API
    pub fn desk_pool(&self) -> DeskPoolApi<'_> {
        DeskPoolApi::new(self)
    }

    /// 获取主机管理 API
    pub fn host(&self) -> HostApi<'_> {
        HostApi::new(self)
    }

    /// 获取模板管理 API
    pub fn model(&self) -> ModelApi<'_> {
        ModelApi::new(self)
    }

    /// 获取用户管理 API
    pub fn user(&self) -> UserApi<'_> {
        UserApi::new(self)
    }

    /// 获取组织单位管理 API
    pub fn group(&self) -> GroupApi<'_> {
        GroupApi::new(self)
    }

    /// 获取快照管理 API
    pub fn snapshot(&self) -> SnapshotApi<'_> {
        SnapshotApi::new(self)
    }

    /// 获取存储管理 API
    pub fn storage(&self) -> StorageApi<'_> {
        StorageApi::new(self)
    }

    /// 获取网络管理 API
    pub fn network(&self) -> NetworkApi<'_> {
        NetworkApi::new(self)
    }

    /// 获取事件管理 API
    pub fn event(&self) -> EventApi<'_> {
        EventApi::new(self)
    }

    /// 获取回收站管理 API
    pub fn recycle(&self) -> RecycleApi<'_> {
        RecycleApi::new(self)
    }

    /// 发送 HTTP 请求
    pub(crate) async fn request<T: Serialize, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<T>,
    ) -> Result<R> {
        let url = format!("{}{}", self.base_url, path);
        debug!("VDI API 请求: {} {}", method, url);

        let token = self.access_token.read().await;
        let token_str = token.as_ref()
            .ok_or_else(|| VdiError::AuthError("未认证，请先登录".to_string()))?;

        let mut request = self.http_client
            .request(method.clone(), &url)
            .header("Token", token_str)
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await
            .map_err(|e| VdiError::HttpError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "无法读取错误响应".to_string());
            warn!("API 请求失败: {} - {}", status, error_text);
            return Err(VdiError::ApiError(status.as_u16(), error_text));
        }

        let result = response.json::<R>().await
            .map_err(|e| VdiError::ParseError(e.to_string()))?;

        Ok(result)
    }

    /// 获取 HTTP 客户端（内部使用）
    pub(crate) fn http_client(&self) -> &Client {
        &self.http_client
    }

    /// 获取基础 URL
    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    /// 获取当前访问令牌
    pub async fn get_token(&self) -> Result<String> {
        let token = self.access_token.read().await;
        token.clone()
            .ok_or_else(|| VdiError::AuthError("未认证，请先登录".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vdi_client_creation() {
        let client = VdiClient::new(
            "http://192.168.1.11:8088",
            VdiConfig::default()
        );
        assert!(client.is_ok());
    }
}

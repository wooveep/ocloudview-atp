//! VDI 平台客户端核心实现

use std::sync::Arc;
use tokio::sync::RwLock;
use reqwest::{Client, Method};
use serde::{Serialize, de::DeserializeOwned};
use tracing::{debug, info, warn};

use crate::error::{VdiError, Result};
use crate::api::{DomainApi, DeskPoolApi, HostApi, ModelApi, UserApi};

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
    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        info!("VDI 客户端登录: {}", username);

        // TODO: 实现实际的登录逻辑
        // 这里需要根据实际的 VDI 平台 API 实现
        let _login_request = serde_json::json!({
            "username": username,
            "password": password,
        });

        // 模拟获取 token
        let token = "mock_token".to_string();
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
    pub fn domain(&self) -> DomainApi {
        DomainApi::new(self)
    }

    /// 获取桌面池管理 API
    pub fn desk_pool(&self) -> DeskPoolApi {
        DeskPoolApi::new(self)
    }

    /// 获取主机管理 API
    pub fn host(&self) -> HostApi {
        HostApi::new(self)
    }

    /// 获取模板管理 API
    pub fn model(&self) -> ModelApi {
        ModelApi::new(self)
    }

    /// 获取用户管理 API
    pub fn user(&self) -> UserApi {
        UserApi::new(self)
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
            .header("Authorization", format!("Bearer {}", token_str))
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

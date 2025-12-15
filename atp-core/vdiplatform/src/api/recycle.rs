//! 回收站管理 API
//!
//! 提供回收站管理功能，包括：
//! - 查询回收站列表
//! - 还原虚拟机/模板
//! - 彻底删除
//! - 清空回收站

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;

/// 回收站管理 API
pub struct RecycleApi<'a> {
    client: &'a VdiClient,
}

impl<'a> RecycleApi<'a> {
    /// 创建新的回收站 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    // ============================================
    // 回收站查询
    // ============================================

    /// 查询回收站列表（分页）
    pub async fn list(&self, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询回收站列表: 第{}页, 每页{}条", page_num, page_size);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/recycle?pageNum={}&pageSize={}", page_num, page_size),
            None::<()>,
        ).await
    }

    /// 查询回收站中的虚拟机
    pub async fn list_domains(&self, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询回收站虚拟机: 第{}页", page_num);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/recycle/domain?pageNum={}&pageSize={}", page_num, page_size),
            None::<()>,
        ).await
    }

    /// 查询回收站中的模板
    pub async fn list_models(&self, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询回收站模板: 第{}页", page_num);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/recycle/model?pageNum={}&pageSize={}", page_num, page_size),
            None::<()>,
        ).await
    }

    // ============================================
    // 还原操作
    // ============================================

    /// 还原虚拟机
    pub async fn restore_domain(&self, domain_id: &str) -> Result<()> {
        info!("还原虚拟机: {}", domain_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/recycle/domain/{}/restore", domain_id),
            None::<()>,
        ).await
    }

    /// 批量还原虚拟机
    pub async fn batch_restore_domains(&self, domain_ids: Vec<String>) -> Result<serde_json::Value> {
        info!("批量还原虚拟机: {} 个", domain_ids.len());
        self.client.request(
            Method::POST,
            "/ocloud/v1/recycle/domain/batch-restore",
            Some(serde_json::json!({ "domainIdList": domain_ids })),
        ).await
    }

    /// 还原模板
    pub async fn restore_model(&self, model_id: &str) -> Result<()> {
        info!("还原模板: {}", model_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/recycle/model/{}/restore", model_id),
            None::<()>,
        ).await
    }

    /// 批量还原模板
    pub async fn batch_restore_models(&self, model_ids: Vec<String>) -> Result<serde_json::Value> {
        info!("批量还原模板: {} 个", model_ids.len());
        self.client.request(
            Method::POST,
            "/ocloud/v1/recycle/model/batch-restore",
            Some(serde_json::json!({ "modelIdList": model_ids })),
        ).await
    }

    // ============================================
    // 彻底删除操作
    // ============================================

    /// 彻底删除虚拟机
    pub async fn permanent_delete_domain(&self, domain_id: &str) -> Result<()> {
        info!("彻底删除虚拟机: {}", domain_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/recycle/domain/{}", domain_id),
            None::<()>,
        ).await
    }

    /// 批量彻底删除虚拟机
    pub async fn batch_permanent_delete_domains(&self, domain_ids: Vec<String>) -> Result<()> {
        info!("批量彻底删除虚拟机: {} 个", domain_ids.len());
        self.client.request(
            Method::POST,
            "/ocloud/v1/recycle/domain/batch-delete",
            Some(serde_json::json!({ "domainIdList": domain_ids })),
        ).await
    }

    /// 彻底删除模板
    pub async fn permanent_delete_model(&self, model_id: &str) -> Result<()> {
        info!("彻底删除模板: {}", model_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/recycle/model/{}", model_id),
            None::<()>,
        ).await
    }

    /// 批量彻底删除模板
    pub async fn batch_permanent_delete_models(&self, model_ids: Vec<String>) -> Result<()> {
        info!("批量彻底删除模板: {} 个", model_ids.len());
        self.client.request(
            Method::POST,
            "/ocloud/v1/recycle/model/batch-delete",
            Some(serde_json::json!({ "modelIdList": model_ids })),
        ).await
    }

    // ============================================
    // 清空操作
    // ============================================

    /// 清空回收站（所有内容）
    pub async fn clear_all(&self) -> Result<()> {
        info!("清空回收站");
        self.client.request(
            Method::POST,
            "/ocloud/v1/recycle/clear",
            None::<()>,
        ).await
    }

    /// 清空回收站中的虚拟机
    pub async fn clear_domains(&self) -> Result<()> {
        info!("清空回收站虚拟机");
        self.client.request(
            Method::POST,
            "/ocloud/v1/recycle/domain/clear",
            None::<()>,
        ).await
    }

    /// 清空回收站中的模板
    pub async fn clear_models(&self) -> Result<()> {
        info!("清空回收站模板");
        self.client.request(
            Method::POST,
            "/ocloud/v1/recycle/model/clear",
            None::<()>,
        ).await
    }

    /// 获取回收站统计信息
    pub async fn get_stats(&self) -> Result<serde_json::Value> {
        info!("获取回收站统计");
        self.client.request(
            Method::GET,
            "/ocloud/v1/recycle/stats",
            None::<()>,
        ).await
    }
}

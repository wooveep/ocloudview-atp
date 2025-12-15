//! 桌面池管理 API
//!
//! 提供桌面池管理功能，包括：
//! - 桌面池 CRUD 操作
//! - 桌面池启用/禁用/激活
//! - 桌面池虚拟机管理
//! - 桌面池策略配置
//! - 桌面池用户绑定

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

    // ============================================
    // 基本查询操作
    // ============================================

    /// 查询桌面池列表（分页）
    pub async fn list(&self, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询桌面池列表: 第{}页, 每页{}条", page_num, page_size);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/desk-pool?pageNum={}&pageSize={}", page_num, page_size),
            None::<()>,
        ).await
    }

    /// 查询所有桌面池
    pub async fn list_all(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询所有桌面池");
        self.client.request(
            Method::GET,
            "/ocloud/v1/desk-pool/all",
            None::<()>,
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

    // ============================================
    // 创建和删除
    // ============================================

    /// 创建桌面池
    pub async fn create(&self, req: CreateDeskPoolRequest) -> Result<DeskPool> {
        info!("创建桌面池: {}", req.name);
        self.client.request(
            Method::POST,
            "/ocloud/v1/desk-pool",
            Some(req),
        ).await
    }

    /// 创建桌面池（完整参数）
    pub async fn create_full(&self, config: serde_json::Value) -> Result<serde_json::Value> {
        info!("创建桌面池（完整参数）");
        self.client.request(
            Method::POST,
            "/ocloud/v1/desk-pool",
            Some(config),
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

    // ============================================
    // 状态管理
    // ============================================

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

    // ============================================
    // 虚拟机管理
    // ============================================

    /// 获取桌面池中的虚拟机列表
    pub async fn list_domains(&self, pool_id: &str) -> Result<Vec<Domain>> {
        info!("获取桌面池虚拟机列表: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/desk-pool/{}/domain/list", pool_id),
            None::<()>,
        ).await
    }

    /// 获取桌面池中的虚拟机列表（分页）
    pub async fn list_domains_paged(&self, pool_id: &str, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("获取桌面池虚拟机列表（分页）: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/desk-pool/{}/domain?pageNum={}&pageSize={}", pool_id, page_num, page_size),
            None::<()>,
        ).await
    }

    /// 添加虚拟机到桌面池
    pub async fn add_domain(&self, pool_id: &str, domain_id: &str) -> Result<()> {
        info!("添加虚拟机到桌面池: {} -> {}", domain_id, pool_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/domain/{}", pool_id, domain_id),
            None::<()>,
        ).await
    }

    /// 从桌面池移除虚拟机
    pub async fn remove_domain(&self, pool_id: &str, domain_id: &str) -> Result<()> {
        info!("从桌面池移除虚拟机: {} <- {}", domain_id, pool_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/desk-pool/{}/domain/{}", pool_id, domain_id),
            None::<()>,
        ).await
    }

    /// 批量添加虚拟机到桌面池
    pub async fn batch_add_domains(&self, pool_id: &str, domain_ids: Vec<String>) -> Result<()> {
        info!("批量添加虚拟机到桌面池: {} 个 -> {}", domain_ids.len(), pool_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/domain/batch", pool_id),
            Some(serde_json::json!({ "domainIdList": domain_ids })),
        ).await
    }

    /// 批量从桌面池移除虚拟机
    pub async fn batch_remove_domains(&self, pool_id: &str, domain_ids: Vec<String>) -> Result<()> {
        info!("批量从桌面池移除虚拟机: {} 个 <- {}", domain_ids.len(), pool_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/desk-pool/{}/domain/batch", pool_id),
            Some(serde_json::json!({ "domainIdList": domain_ids })),
        ).await
    }

    // ============================================
    // 模板管理
    // ============================================

    /// 切换桌面池模板
    pub async fn switch_model(&self, pool_id: &str, model_id: &str) -> Result<()> {
        info!("切换桌面池模板: {} -> {}", pool_id, model_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/desk-pool/switch-model",
            Some(serde_json::json!({
                "poolId": pool_id,
                "modelId": model_id,
            })),
        ).await
    }

    /// 获取桌面池关联的模板
    pub async fn get_model(&self, pool_id: &str) -> Result<serde_json::Value> {
        info!("获取桌面池模板: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/desk-pool/{}/model", pool_id),
            None::<()>,
        ).await
    }

    // ============================================
    // 用户绑定
    // ============================================

    /// 获取桌面池用户列表
    pub async fn list_users(&self, pool_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("获取桌面池用户列表: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/desk-pool/{}/user", pool_id),
            None::<()>,
        ).await
    }

    /// 绑定用户到桌面池
    pub async fn bind_user(&self, pool_id: &str, user_id: &str) -> Result<()> {
        info!("绑定用户到桌面池: {} -> {}", user_id, pool_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/user/{}", pool_id, user_id),
            None::<()>,
        ).await
    }

    /// 解绑桌面池用户
    pub async fn unbind_user(&self, pool_id: &str, user_id: &str) -> Result<()> {
        info!("解绑桌面池用户: {} <- {}", user_id, pool_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/desk-pool/{}/user/{}", pool_id, user_id),
            None::<()>,
        ).await
    }

    /// 批量绑定用户到桌面池
    pub async fn batch_bind_users(&self, pool_id: &str, user_ids: Vec<String>) -> Result<()> {
        info!("批量绑定用户到桌面池: {} 个 -> {}", user_ids.len(), pool_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/user/batch", pool_id),
            Some(serde_json::json!({ "userIdList": user_ids })),
        ).await
    }

    // ============================================
    // 配置管理
    // ============================================

    /// 修改桌面池名称
    pub async fn update_name(&self, pool_id: &str, name: &str) -> Result<()> {
        info!("修改桌面池名称: {} -> {}", pool_id, name);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/desk-pool/{}/name", pool_id),
            Some(serde_json::json!({ "name": name })),
        ).await
    }

    /// 修改桌面池备注
    pub async fn update_remark(&self, pool_id: &str, remark: &str) -> Result<()> {
        info!("修改桌面池备注: {}", pool_id);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/desk-pool/{}/remark", pool_id),
            Some(serde_json::json!({ "remark": remark })),
        ).await
    }

    /// 修改桌面池配置
    pub async fn update_config(&self, pool_id: &str, config: serde_json::Value) -> Result<()> {
        info!("修改桌面池配置: {}", pool_id);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/desk-pool/{}", pool_id),
            Some(config),
        ).await
    }

    // ============================================
    // 策略管理
    // ============================================

    /// 获取桌面池策略
    pub async fn get_policy(&self, pool_id: &str) -> Result<serde_json::Value> {
        info!("获取桌面池策略: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/desk-pool/{}/policy", pool_id),
            None::<()>,
        ).await
    }

    /// 设置桌面池策略
    pub async fn set_policy(&self, pool_id: &str, policy: serde_json::Value) -> Result<()> {
        info!("设置桌面池策略: {}", pool_id);
        self.client.request(
            Method::PUT,
            &format!("/ocloud/v1/desk-pool/{}/policy", pool_id),
            Some(policy),
        ).await
    }

    // ============================================
    // 扩容缩容
    // ============================================

    /// 桌面池扩容
    ///
    /// # Arguments
    /// * `pool_id` - 桌面池 ID
    /// * `count` - 扩容数量
    pub async fn expand(&self, pool_id: &str, count: u32) -> Result<serde_json::Value> {
        info!("桌面池扩容: {} + {}", pool_id, count);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/expand", pool_id),
            Some(serde_json::json!({ "count": count })),
        ).await
    }

    /// 桌面池缩容
    ///
    /// # Arguments
    /// * `pool_id` - 桌面池 ID
    /// * `count` - 缩容数量
    pub async fn shrink(&self, pool_id: &str, count: u32) -> Result<serde_json::Value> {
        info!("桌面池缩容: {} - {}", pool_id, count);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/shrink", pool_id),
            Some(serde_json::json!({ "count": count })),
        ).await
    }

    // ============================================
    // 批量操作
    // ============================================

    /// 批量启动桌面池中的虚拟机
    pub async fn batch_start_domains(&self, pool_id: &str) -> Result<serde_json::Value> {
        info!("批量启动桌面池虚拟机: {}", pool_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/domain/start", pool_id),
            None::<()>,
        ).await
    }

    /// 批量关闭桌面池中的虚拟机
    pub async fn batch_shutdown_domains(&self, pool_id: &str) -> Result<serde_json::Value> {
        info!("批量关闭桌面池虚拟机: {}", pool_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/domain/shutdown", pool_id),
            None::<()>,
        ).await
    }

    /// 批量重启桌面池中的虚拟机
    pub async fn batch_reboot_domains(&self, pool_id: &str) -> Result<serde_json::Value> {
        info!("批量重启桌面池虚拟机: {}", pool_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/desk-pool/{}/domain/reboot", pool_id),
            None::<()>,
        ).await
    }
}

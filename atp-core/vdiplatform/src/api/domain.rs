//! 虚拟机管理 API
//!
//! 提供完整的虚拟机管理功能，包括：
//! - 基本操作：创建、启动、关闭、重启、删除
//! - 批量操作：批量启动、批量关闭、批量重启、批量删除
//! - 克隆操作：完全克隆虚拟机
//! - 配置修改：修改 CPU/内存、网络配置
//! - 用户绑定：绑定/解绑用户

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;
use crate::models::{
    Domain, CreateDomainRequest, DomainStatus,
    BatchTaskRequest, BatchTaskResponse, BatchDeleteRequest,
    CloneDomainRequest, CloneDomainResponse,
    UpdateMemCpuRequest, BatchUpdateConfigRequest,
    NetworkConfigRequest, NicInfo,
    CreateDomainFullRequest, CreateDomainResponse,
};

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

    /// 按状态查询虚拟机
    ///
    /// # Arguments
    /// * `status` - 要筛选的状态，None 表示查询所有状态
    ///
    /// # Example
    /// ```ignore
    /// // 查询所有运行中的虚拟机
    /// let running_vms = client.domain().list_by_status(Some(DomainStatus::Running)).await?;
    ///
    /// // 查询所有关机的虚拟机
    /// let shutoff_vms = client.domain().list_by_status(Some(DomainStatus::Shutoff)).await?;
    /// ```
    pub async fn list_by_status(&self, status: Option<DomainStatus>) -> Result<Vec<serde_json::Value>> {
        let all = self.list_all().await?;

        match status {
            Some(target_status) => {
                let target_code = target_status.code();
                Ok(all
                    .into_iter()
                    .filter(|vm| vm["status"].as_i64().unwrap_or(-1) == target_code)
                    .collect())
            }
            None => Ok(all),
        }
    }

    /// 按多个状态查询虚拟机
    ///
    /// # Arguments
    /// * `statuses` - 要筛选的状态列表
    ///
    /// # Example
    /// ```ignore
    /// // 查询运行中或挂起的虚拟机
    /// let vms = client.domain()
    ///     .list_by_statuses(&[DomainStatus::Running, DomainStatus::Paused])
    ///     .await?;
    /// ```
    pub async fn list_by_statuses(&self, statuses: &[DomainStatus]) -> Result<Vec<serde_json::Value>> {
        let all = self.list_all().await?;
        let target_codes: Vec<i64> = statuses.iter().map(|s| s.code()).collect();

        Ok(all
            .into_iter()
            .filter(|vm| {
                let code = vm["status"].as_i64().unwrap_or(-1);
                target_codes.contains(&code)
            })
            .collect())
    }

    /// 查询运行中的虚拟机
    pub async fn list_running(&self) -> Result<Vec<serde_json::Value>> {
        self.list_by_status(Some(DomainStatus::Running)).await
    }

    /// 查询关机的虚拟机
    pub async fn list_shutoff(&self) -> Result<Vec<serde_json::Value>> {
        self.list_by_status(Some(DomainStatus::Shutoff)).await
    }

    /// 查询可操作状态的虚拟机（非过渡状态）
    pub async fn list_operable(&self) -> Result<Vec<serde_json::Value>> {
        self.list_by_statuses(&[
            DomainStatus::Running,
            DomainStatus::Shutoff,
            DomainStatus::Paused,
            DomainStatus::Hibernated,
        ]).await
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

    // ============================================
    // 批量操作
    // ============================================

    /// 批量启动虚拟机
    ///
    /// # Arguments
    /// * `req` - 批量任务请求，包含虚拟机 ID 列表和可选的目标主机
    ///
    /// # Example
    /// ```ignore
    /// let req = BatchTaskRequest::new(vec!["vm-1".into(), "vm-2".into()])
    ///     .with_host("host-1".into());
    /// let response = client.domain().batch_start(req).await?;
    /// ```
    pub async fn batch_start(&self, req: BatchTaskRequest) -> Result<BatchTaskResponse> {
        info!("批量启动虚拟机: {} 个", req.id_list.len());
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/start",
            Some(req),
        ).await
    }

    /// 批量关闭虚拟机
    ///
    /// # Arguments
    /// * `req` - 批量任务请求，可设置 is_force 为强制关机
    pub async fn batch_shutdown(&self, req: BatchTaskRequest) -> Result<BatchTaskResponse> {
        info!("批量关闭虚拟机: {} 个", req.id_list.len());
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/close",
            Some(req),
        ).await
    }

    /// 批量强制关闭虚拟机
    ///
    /// # Arguments
    /// * `id_list` - 虚拟机 ID 列表
    pub async fn batch_force_shutdown(&self, id_list: Vec<String>) -> Result<BatchTaskResponse> {
        info!("批量强制关闭虚拟机: {} 个", id_list.len());
        let req = BatchTaskRequest::new(id_list).with_force(true);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/close",
            Some(req),
        ).await
    }

    /// 批量重启虚拟机
    ///
    /// # Arguments
    /// * `id_list` - 虚拟机 ID 列表
    pub async fn batch_reboot(&self, id_list: Vec<String>) -> Result<BatchTaskResponse> {
        info!("批量重启虚拟机: {} 个", id_list.len());
        let req = BatchTaskRequest::new(id_list);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/reboot",
            Some(req),
        ).await
    }

    /// 批量删除虚拟机
    ///
    /// # Arguments
    /// * `req` - 批量删除请求，可选择彻底删除或移至回收站
    ///
    /// # Example
    /// ```ignore
    /// // 彻底删除
    /// let req = BatchDeleteRequest::permanent(vec!["vm-1".into(), "vm-2".into()]);
    /// client.domain().batch_delete(req).await?;
    ///
    /// // 移至回收站
    /// let req = BatchDeleteRequest::to_recycle(vec!["vm-1".into()]);
    /// client.domain().batch_delete(req).await?;
    /// ```
    pub async fn batch_delete(&self, req: BatchDeleteRequest) -> Result<()> {
        info!("批量删除虚拟机: {} 个, 回收站: {}",
            req.domain_id_list.len(),
            req.is_recycle == 1
        );
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/delete",
            Some(req),
        ).await
    }

    // ============================================
    // 克隆操作
    // ============================================

    /// 克隆虚拟机（完全克隆）
    ///
    /// # Arguments
    /// * `domain_id` - 源虚拟机 ID
    /// * `req` - 克隆请求，包含目标名称、数量等
    ///
    /// # Example
    /// ```ignore
    /// // 单个克隆
    /// let req = CloneDomainRequest::single("new-vm-name".into());
    /// let response = client.domain().clone("source-vm-id", req).await?;
    ///
    /// // 批量克隆
    /// let req = CloneDomainRequest::batch("vm-prefix-".into(), 5);
    /// let response = client.domain().clone("source-vm-id", req).await?;
    /// ```
    pub async fn clone(&self, domain_id: &str, req: CloneDomainRequest) -> Result<CloneDomainResponse> {
        info!("克隆虚拟机: {} -> {} (数量: {:?})", domain_id, req.name, req.number);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/clone", domain_id),
            Some(req),
        ).await
    }

    // ============================================
    // 配置修改
    // ============================================

    /// 批量修改虚拟机 CPU 和内存
    ///
    /// # Arguments
    /// * `req` - 修改请求，包含虚拟机 ID 列表和新的 CPU/内存值
    ///
    /// # Example
    /// ```ignore
    /// let req = UpdateMemCpuRequest::new(vec!["vm-1".into(), "vm-2".into()])
    ///     .with_cpu(4)
    ///     .with_memory(8192);
    /// client.domain().batch_update_mem_cpu(req).await?;
    /// ```
    pub async fn batch_update_mem_cpu(&self, req: UpdateMemCpuRequest) -> Result<()> {
        info!("批量修改 CPU/内存: {} 个虚拟机, CPU: {:?}, 内存: {:?} MB",
            req.list_id.len(), req.cpu, req.memory
        );
        self.client.request(
            Method::PATCH,
            "/ocloud/v1/domain/mem-cpu",
            Some(req),
        ).await
    }

    /// 批量修改虚拟机其他配置
    ///
    /// 可修改的配置包括：
    /// - domain_fake: 虚拟伪装
    /// - gpu_type: GPU 型号
    /// - host_bios_enable: 启用主机 BIOS
    /// - host_model_enable: 启用主机型号
    /// - nested_virtual: 嵌套虚拟化
    pub async fn batch_update_config(&self, req: BatchUpdateConfigRequest) -> Result<()> {
        info!("批量修改虚拟机配置: {} 个", req.id_list.len());
        self.client.request(
            Method::PATCH,
            "/ocloud/v1/domain",
            Some(req),
        ).await
    }

    // ============================================
    // 网络管理
    // ============================================

    /// 添加或修改虚拟机网卡
    ///
    /// # Arguments
    /// * `domain_id` - 虚拟机 ID
    /// * `req` - 网络配置请求
    ///
    /// # Example
    /// ```ignore
    /// let req = NetworkConfigRequest::new("ovs-id".into())
    ///     .with_vlan("100".into())
    ///     .with_inbound_limit(1000, 500, 2000)
    ///     .with_outbound_limit(1000, 500, 2000);
    /// client.domain().update_nic("vm-id", req).await?;
    /// ```
    pub async fn update_nic(&self, domain_id: &str, req: NetworkConfigRequest) -> Result<()> {
        info!("更新虚拟机网卡: {}, OVS: {}", domain_id, req.ovs_id);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/domain/{}/nic", domain_id),
            Some(req),
        ).await
    }

    /// 删除虚拟机网卡
    ///
    /// # Arguments
    /// * `domain_id` - 虚拟机 ID
    /// * `mac` - 网卡 MAC 地址
    pub async fn delete_nic(&self, domain_id: &str, mac: &str) -> Result<()> {
        info!("删除虚拟机网卡: {}, MAC: {}", domain_id, mac);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/domain/{}/nic/{}", domain_id, mac),
            None::<()>,
        ).await
    }

    /// 获取虚拟机网卡列表
    ///
    /// # Arguments
    /// * `domain_id` - 虚拟机 ID
    pub async fn get_nics(&self, domain_id: &str) -> Result<Vec<NicInfo>> {
        info!("获取虚拟机网卡列表: {}", domain_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/nic", domain_id),
            None::<()>,
        ).await
    }

    // ============================================
    // 完整创建
    // ============================================

    /// 创建虚拟机（完整参数版）
    ///
    /// 支持设置完整的虚拟机参数，包括 CPU 拓扑、启动引导、网络、磁盘等。
    ///
    /// # Arguments
    /// * `req` - 完整创建请求
    ///
    /// # Example
    /// ```ignore
    /// let req = CreateDomainFullRequest::basic("new-vm".into(), 4, 8192)
    ///     .with_uefi()
    ///     .with_storage_pool("pool-id".into())
    ///     .with_os_type("windows".into())
    ///     .with_networks(vec![
    ///         NetworkConfigRequest::new("ovs-id".into()).with_vlan("100".into())
    ///     ]);
    /// let response = client.domain().create_full(req).await?;
    /// ```
    pub async fn create_full(&self, req: CreateDomainFullRequest) -> Result<CreateDomainResponse> {
        info!("创建虚拟机（完整参数）: {}, CPU: {:?}, 内存: {:?} MB",
            req.name, req.cpu, req.memory
        );
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain",
            Some(req),
        ).await
    }

    // ============================================
    // 更多批量操作
    // ============================================

    /// 批量挂起虚拟机
    pub async fn batch_suspend(&self, id_list: Vec<String>) -> Result<BatchTaskResponse> {
        info!("批量挂起虚拟机: {} 个", id_list.len());
        let req = BatchTaskRequest::new(id_list);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/suspend",
            Some(req),
        ).await
    }

    /// 批量恢复虚拟机
    pub async fn batch_resume(&self, id_list: Vec<String>) -> Result<BatchTaskResponse> {
        info!("批量恢复虚拟机: {} 个", id_list.len());
        let req = BatchTaskRequest::new(id_list);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/resume",
            Some(req),
        ).await
    }

    /// 批量休眠虚拟机
    pub async fn batch_sleep(&self, id_list: Vec<String>) -> Result<BatchTaskResponse> {
        info!("批量休眠虚拟机: {} 个", id_list.len());
        let req = BatchTaskRequest::new(id_list);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/sleep",
            Some(req),
        ).await
    }

    /// 批量唤醒虚拟机
    pub async fn batch_wakeup(&self, id_list: Vec<String>) -> Result<BatchTaskResponse> {
        info!("批量唤醒虚拟机: {} 个", id_list.len());
        let req = BatchTaskRequest::new(id_list);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/wakeup",
            Some(req),
        ).await
    }

    /// 批量设置还原点
    pub async fn batch_freeze(&self, id_list: Vec<String>) -> Result<BatchTaskResponse> {
        info!("批量设置还原点: {} 个", id_list.len());
        let req = BatchTaskRequest::new(id_list);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/freeze",
            Some(req),
        ).await
    }

    /// 批量取消还原点
    pub async fn batch_restore(&self, id_list: Vec<String>) -> Result<BatchTaskResponse> {
        info!("批量取消还原点: {} 个", id_list.len());
        let req = BatchTaskRequest::new(id_list);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/restore",
            Some(req),
        ).await
    }

    /// 批量重置虚拟机
    pub async fn batch_rebase(&self, id_list: Vec<String>) -> Result<BatchTaskResponse> {
        info!("批量重置虚拟机: {} 个", id_list.len());
        let req = BatchTaskRequest::new(id_list);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/rebase",
            Some(req),
        ).await
    }

    /// 无 GPU 模式启动虚拟机
    pub async fn batch_start_without_gpu(&self, id_list: Vec<String>) -> Result<BatchTaskResponse> {
        info!("无 GPU 模式启动虚拟机: {} 个", id_list.len());
        let req = BatchTaskRequest::new(id_list);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/start-without-gpu",
            Some(req),
        ).await
    }

    // ============================================
    // 磁盘管理
    // ============================================

    /// 获取虚拟机磁盘列表
    pub async fn get_disks(&self, domain_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("获取虚拟机磁盘列表: {}", domain_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/disk", domain_id),
            None::<()>,
        ).await
    }

    /// 添加磁盘
    pub async fn add_disk(&self, domain_id: &str, disk: serde_json::Value) -> Result<()> {
        info!("添加磁盘到虚拟机: {}", domain_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/disk", domain_id),
            Some(disk),
        ).await
    }

    /// 删除磁盘
    pub async fn delete_disk(&self, domain_id: &str, volume_id: &str) -> Result<()> {
        info!("删除虚拟机磁盘: {} -> {}", domain_id, volume_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/domain/{}/disk/{}", domain_id, volume_id),
            None::<()>,
        ).await
    }

    /// 卸载磁盘
    pub async fn unmount_disk(&self, domain_id: &str, volume_id: &str) -> Result<()> {
        info!("卸载虚拟机磁盘: {} -> {}", domain_id, volume_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/disk/{}/unmount", domain_id, volume_id),
            None::<()>,
        ).await
    }

    /// 磁盘扩容
    pub async fn expand_disk(&self, domain_id: &str, volume_id: &str, size_gb: u64) -> Result<()> {
        info!("磁盘扩容: {} -> {} ({}GB)", domain_id, volume_id, size_gb);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/disk/{}/expand", domain_id, volume_id),
            Some(serde_json::json!({ "size": size_gb })),
        ).await
    }

    /// 设置磁盘限速
    pub async fn set_disk_speed_limit(&self, domain_id: &str, volume_id: &str, read_bytes: Option<i64>, write_bytes: Option<i64>) -> Result<()> {
        info!("设置磁盘限速: {} -> {}", domain_id, volume_id);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/domain/{}/disk/{}/speed-limit", domain_id, volume_id),
            Some(serde_json::json!({
                "readBytesSec": read_bytes,
                "writeBytesSec": write_bytes,
            })),
        ).await
    }

    /// 批量增加独立磁盘
    pub async fn batch_add_independent_disk(&self, id_list: Vec<String>, disk_config: serde_json::Value) -> Result<()> {
        info!("批量增加独立磁盘: {} 个", id_list.len());
        let mut req = disk_config;
        req["idList"] = serde_json::json!(id_list);
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/independent-disk",
            Some(req),
        ).await
    }

    // ============================================
    // ISO 管理
    // ============================================

    /// 获取虚拟机 ISO 列表
    pub async fn get_isos(&self, domain_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("获取虚拟机 ISO 列表: {}", domain_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/iso", domain_id),
            None::<()>,
        ).await
    }

    /// 添加 ISO
    pub async fn add_iso(&self, domain_id: &str, iso_path: &str, storage_pool_id: Option<&str>) -> Result<()> {
        info!("添加 ISO 到虚拟机: {} -> {}", domain_id, iso_path);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/iso", domain_id),
            Some(serde_json::json!({
                "isoPath": iso_path,
                "storagePoolId": storage_pool_id,
            })),
        ).await
    }

    /// 删除 ISO
    pub async fn delete_iso(&self, domain_id: &str) -> Result<()> {
        info!("删除虚拟机 ISO: {}", domain_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/iso/delete", domain_id),
            None::<()>,
        ).await
    }

    /// 动态更换 ISO
    pub async fn change_iso(&self, domain_id: &str, iso_path: &str) -> Result<()> {
        info!("更换虚拟机 ISO: {} -> {}", domain_id, iso_path);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/domain/{}/iso", domain_id),
            Some(serde_json::json!({ "isoPath": iso_path })),
        ).await
    }

    // ============================================
    // 网卡管理 (扩展)
    // ============================================

    /// 添加网卡
    pub async fn add_nic(&self, domain_id: &str, req: NetworkConfigRequest) -> Result<()> {
        info!("添加网卡到虚拟机: {}, OVS: {}", domain_id, req.ovs_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/nic", domain_id),
            Some(req),
        ).await
    }

    // ============================================
    // 迁移和监控
    // ============================================

    /// 迁移虚拟机
    pub async fn migrate(&self, domain_id: &str, target_host_id: &str) -> Result<()> {
        info!("迁移虚拟机: {} -> {}", domain_id, target_host_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/migrate", domain_id),
            Some(serde_json::json!({ "hostId": target_host_id })),
        ).await
    }

    /// 请求后台监控
    pub async fn request_monitor(&self, domain_id: &str) -> Result<String> {
        info!("请求后台监控: {}", domain_id);
        let response: serde_json::Value = self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/monitor", domain_id),
            None::<()>,
        ).await?;
        Ok(response["requestId"].as_str().unwrap_or("").to_string())
    }

    /// 查询后台监控状态
    pub async fn get_monitor_status(&self, domain_id: &str, request_id: &str) -> Result<serde_json::Value> {
        info!("查询后台监控状态: {} -> {}", domain_id, request_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/monitor/{}", domain_id, request_id),
            None::<()>,
        ).await
    }

    // ============================================
    // 查询方法
    // ============================================

    /// 获取虚拟机子虚机列表
    pub async fn get_children(&self, domain_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("获取母虚机的子虚机: {}", domain_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/child", domain_id),
            None::<()>,
        ).await
    }

    /// 获取虚拟机 VNC/SPICE 端口
    pub async fn get_ports(&self, domain_id: &str) -> Result<serde_json::Value> {
        info!("获取虚拟机端口: {}", domain_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/port", domain_id),
            None::<()>,
        ).await
    }

    /// 获取 VNC 访问密码
    pub async fn get_vnc_password(&self, domain_id: &str) -> Result<String> {
        info!("获取 VNC 密码: {}", domain_id);
        let response: serde_json::Value = self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/vnc-password", domain_id),
            None::<()>,
        ).await?;
        Ok(response["password"].as_str().unwrap_or("").to_string())
    }

    /// 获取 SPICE Key
    pub async fn get_spice_key(&self, domain_id: &str) -> Result<String> {
        info!("获取 SPICE Key: {}", domain_id);
        let response: serde_json::Value = self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/spice-key", domain_id),
            None::<()>,
        ).await?;
        Ok(response["key"].as_str().unwrap_or("").to_string())
    }

    /// 获取虚拟机 XML
    pub async fn get_xml(&self, domain_id: &str) -> Result<String> {
        info!("获取虚拟机 XML: {}", domain_id);
        let response: serde_json::Value = self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/xml", domain_id),
            None::<()>,
        ).await?;
        Ok(response["xml"].as_str().unwrap_or("").to_string())
    }

    /// 获取虚拟机进程列表
    pub async fn get_task_manager(&self, domain_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("获取虚拟机进程列表: {}", domain_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/task-manager", domain_id),
            None::<()>,
        ).await
    }

    /// 同步虚拟机状态
    pub async fn sync_status(&self, domain_id: &str) -> Result<()> {
        info!("同步虚拟机状态: {}", domain_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/sync-status", domain_id),
            None::<()>,
        ).await
    }

    /// 虚拟机转模板
    pub async fn to_model(&self, domain_id: &str) -> Result<()> {
        info!("虚拟机转模板: {}", domain_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/domain/{}/to-model", domain_id),
            None::<()>,
        ).await
    }

    /// 修改单个虚拟机配置
    pub async fn update(&self, domain_id: &str, config: serde_json::Value) -> Result<()> {
        info!("修改虚拟机配置: {}", domain_id);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/domain/{}", domain_id),
            Some(config),
        ).await
    }

    /// 获取在线用户及其虚机列表
    pub async fn get_active_users(&self) -> Result<Vec<serde_json::Value>> {
        info!("获取在线用户及虚机列表");
        self.client.request(
            Method::GET,
            "/ocloud/v1/domain/active-user",
            None::<()>,
        ).await
    }

    /// 获取虚拟机代理版本列表
    pub async fn get_agent_versions(&self) -> Result<Vec<serde_json::Value>> {
        info!("获取虚拟机代理版本列表");
        self.client.request(
            Method::GET,
            "/ocloud/v1/domain/agent-version",
            None::<()>,
        ).await
    }

    /// 查询虚拟机的存储池交集
    pub async fn get_common_storage_pool(&self, id_list: Vec<String>) -> Result<Vec<serde_json::Value>> {
        info!("查询虚拟机存储池交集: {} 个", id_list.len());
        let ids = id_list.join(",");
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/common-storage-pool?ids={}", ids),
            None::<()>,
        ).await
    }

    /// 获取用户统计数据
    pub async fn get_user_stat(&self) -> Result<serde_json::Value> {
        info!("获取用户统计数据");
        self.client.request(
            Method::GET,
            "/ocloud/v1/domain/user-stat",
            None::<()>,
        ).await
    }

    /// 启动虚拟机之前获取 IP 展示
    pub async fn get_ip_preview(&self, domain_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("获取虚拟机 IP 预览: {}", domain_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/ip-all", domain_id),
            None::<()>,
        ).await
    }

    /// 导入虚拟机
    pub async fn import_domain(&self, config: serde_json::Value) -> Result<()> {
        info!("导入虚拟机");
        self.client.request(
            Method::POST,
            "/ocloud/v1/domain/import",
            Some(config),
        ).await
    }
}


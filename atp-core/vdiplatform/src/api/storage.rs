//! 存储管理 API
//!
//! 提供存储池和存储卷管理功能，包括：
//! - 存储池 CRUD
//! - 存储卷 CRUD
//! - ISO 管理

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;

/// 存储管理 API
pub struct StorageApi<'a> {
    client: &'a VdiClient,
}

impl<'a> StorageApi<'a> {
    /// 创建新的存储 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    // ============================================
    // 存储池管理
    // ============================================

    /// 存储池分页查询
    pub async fn list_pools(&self, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询存储池列表: 第{}页, 每页{}条", page_num, page_size);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/storage-pool?pageNum={}&pageSize={}", page_num, page_size),
            None::<()>,
        ).await
    }

    /// 存储池查询全部
    pub async fn list_all_pools(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询所有存储池");
        self.client.request(
            Method::GET,
            "/ocloud/v1/storage-pool/all",
            None::<()>,
        ).await
    }

    /// 查询存储池详情
    pub async fn get_pool(&self, pool_id: &str) -> Result<serde_json::Value> {
        info!("查询存储池详情: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/storage-pool/{}", pool_id),
            None::<()>,
        ).await
    }

    /// 删除存储池
    pub async fn delete_pool(&self, pool_id: &str) -> Result<()> {
        info!("删除存储池: {}", pool_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/storage-pool/{}", pool_id),
            None::<()>,
        ).await
    }

    /// 存储池重命名
    pub async fn rename_pool(&self, pool_id: &str, name: &str) -> Result<()> {
        info!("存储池重命名: {} -> {}", pool_id, name);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/storage-pool/{}/name", pool_id),
            Some(serde_json::json!({ "name": name })),
        ).await
    }

    /// 查询存储池实际空间
    pub async fn get_pool_usage(&self, pool_id: &str) -> Result<serde_json::Value> {
        info!("查询存储池空间: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/storage-pool/{}/usage", pool_id),
            None::<()>,
        ).await
    }

    /// 绑定存储池到资源池
    pub async fn bind_pool_to_resource(&self, pool_id: &str, resource_pool_id: &str) -> Result<()> {
        info!("绑定存储池到资源池: {} -> {}", pool_id, resource_pool_id);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/storage-pool/{}/bind-pool", pool_id),
            Some(serde_json::json!({ "poolId": resource_pool_id })),
        ).await
    }

    /// 解绑存储池
    pub async fn unbind_pool_from_resource(&self, pool_id: &str) -> Result<()> {
        info!("解绑存储池: {}", pool_id);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/storage-pool/{}/unbind-pool", pool_id),
            None::<()>,
        ).await
    }

    // ============================================
    // 创建存储池
    // ============================================

    /// 创建本地文件存储池
    pub async fn create_dir_pool(&self, config: serde_json::Value) -> Result<serde_json::Value> {
        info!("创建本地文件存储池");
        self.client.request(
            Method::POST,
            "/ocloud/v1/storage-pool/dir",
            Some(config),
        ).await
    }

    /// 创建 NFS 存储池
    pub async fn create_nfs_pool(&self, config: serde_json::Value) -> Result<serde_json::Value> {
        info!("创建 NFS 存储池");
        self.client.request(
            Method::POST,
            "/ocloud/v1/storage-pool/nfs",
            Some(config),
        ).await
    }

    /// 创建 GFS 存储池
    pub async fn create_gfs_pool(&self, config: serde_json::Value) -> Result<serde_json::Value> {
        info!("创建 GFS 存储池");
        self.client.request(
            Method::POST,
            "/ocloud/v1/storage-pool/gfs",
            Some(config),
        ).await
    }

    /// 创建 Gluster 存储池
    pub async fn create_gluster_pool(&self, config: serde_json::Value) -> Result<serde_json::Value> {
        info!("创建 Gluster 存储池");
        self.client.request(
            Method::POST,
            "/ocloud/v1/storage-pool/gluster",
            Some(config),
        ).await
    }

    /// 创建 LVM 存储池
    pub async fn create_lvm_pool(&self, config: serde_json::Value) -> Result<serde_json::Value> {
        info!("创建 LVM 存储池");
        self.client.request(
            Method::POST,
            "/ocloud/v1/storage-pool/lvm",
            Some(config),
        ).await
    }

    /// 创建共享 LVM 存储池
    pub async fn create_share_lvm_pool(&self, config: serde_json::Value) -> Result<serde_json::Value> {
        info!("创建共享 LVM 存储池");
        self.client.request(
            Method::POST,
            "/ocloud/v1/storage-pool/share-lvm",
            Some(config),
        ).await
    }

    /// 查询 LVM VG
    pub async fn list_lvm_vg(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询 LVM VG");
        self.client.request(
            Method::GET,
            "/ocloud/v1/storage-pool/lvm-vg",
            None::<()>,
        ).await
    }

    /// 查询共享 LVM VG
    pub async fn list_share_vg(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询共享 LVM VG");
        self.client.request(
            Method::GET,
            "/ocloud/v1/storage-pool/share-vg",
            None::<()>,
        ).await
    }

    /// 查询 Gluster 存储池特定信息
    pub async fn get_gluster_info(&self, pool_id: &str) -> Result<serde_json::Value> {
        info!("查询 Gluster 存储池信息: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/storage-pool/{}/gluster", pool_id),
            None::<()>,
        ).await
    }

    // ============================================
    // 存储卷管理
    // ============================================

    /// 查询存储卷（分页）
    pub async fn list_volumes(&self, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询存储卷列表: 第{}页, 每页{}条", page_num, page_size);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/storage-volume?pageNum={}&pageSize={}", page_num, page_size),
            None::<()>,
        ).await
    }

    /// 查询存储卷（全部）
    pub async fn list_all_volumes(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询所有存储卷");
        self.client.request(
            Method::GET,
            "/ocloud/v1/storage-volume/all",
            None::<()>,
        ).await
    }

    /// 创建存储卷
    pub async fn create_volume(&self, config: serde_json::Value) -> Result<serde_json::Value> {
        info!("创建存储卷");
        self.client.request(
            Method::POST,
            "/ocloud/v1/storage-volume",
            Some(config),
        ).await
    }

    /// 删除存储卷
    pub async fn delete_volume(&self, volume_id: &str) -> Result<()> {
        info!("删除存储卷: {}", volume_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/storage-volume/{}", volume_id),
            None::<()>,
        ).await
    }

    // ============================================
    // ISO 管理
    // ============================================

    /// 分页查询存储池下的 ISO
    pub async fn list_pool_isos(&self, pool_id: &str, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询存储池 ISO: {}", pool_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/storage-pool/{}/iso?pageNum={}&pageSize={}", pool_id, page_num, page_size),
            None::<()>,
        ).await
    }

    /// 删除 ISO 镜像文件
    pub async fn delete_iso(&self, pool_id: &str, iso_name: &str) -> Result<()> {
        info!("删除 ISO: {} -> {}", pool_id, iso_name);
        self.client.request(
            Method::POST,
            "/ocloud/v1/storage-pool/delete-iso",
            Some(serde_json::json!({
                "poolId": pool_id,
                "isoName": iso_name,
            })),
        ).await
    }

    /// 挂载 ISO 到虚拟机
    pub async fn mount_iso(&self, domain_id: &str, iso_path: &str) -> Result<()> {
        info!("挂载 ISO: {} -> {}", iso_path, domain_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/iso/mount",
            Some(serde_json::json!({
                "domainId": domain_id,
                "isoPath": iso_path,
            })),
        ).await
    }

    /// 从虚拟机卸载 ISO
    pub async fn unmount_iso(&self, domain_id: &str) -> Result<()> {
        info!("卸载 ISO: {}", domain_id);
        self.client.request(
            Method::POST,
            "/ocloud/v1/iso/unmount",
            Some(serde_json::json!({ "domainId": domain_id })),
        ).await
    }

    /// 查询可以挂载或卸载 ISO 的虚拟机
    pub async fn list_mount_domains(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询可挂载 ISO 的虚拟机");
        self.client.request(
            Method::GET,
            "/ocloud/v1/iso/mount-domain",
            None::<()>,
        ).await
    }
}

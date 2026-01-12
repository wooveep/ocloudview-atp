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

    /// 存储池查询全部（自动处理分页）
    pub async fn list_all_pools(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询所有存储池");
        
        let url = "/ocloud/v1/storage-pool?pageNum=1&pageSize=1000";
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

    /// 查询存储卷（全部，遍历所有存储池）
    ///
    /// 注意：VDI API 需要指定 storagePoolId 才能获取存储卷，
    /// 因此此方法会先获取所有存储池，然后遍历每个存储池获取其存储卷
    pub async fn list_all_volumes(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询所有存储卷（遍历所有存储池）");
        
        // 1. 先获取所有存储池
        let storage_pools = self.list_all_pools().await?;
        
        let mut all_volumes = Vec::new();
        let token = self.client.get_token().await?;
        
        // 2. 遍历每个存储池获取其存储卷
        for pool in &storage_pools {
            let pool_id = match pool["id"].as_str() {
                Some(id) => id,
                None => continue,
            };
            
            let pool_name = pool["name"].as_str().unwrap_or("unknown");
            tracing::debug!("获取存储池 {} ({}) 的存储卷", pool_name, pool_id);
            
            // 使用分页获取所有存储卷
            let url = format!(
                "/ocloud/v1/storage-volume?pageNum=1&pageSize=10000&storagePoolId={}&snapshotFree=1",
                pool_id
            );
            
            let response: serde_json::Value = self.client.http_client()
                .get(&format!("{}{}", self.client.base_url(), &url))
                .header("Token", &token)
                .send()
                .await
                .map_err(|e| crate::error::VdiError::HttpError(e.to_string()))?
                .json()
                .await
                .map_err(|e| crate::error::VdiError::ParseError(e.to_string()))?;
            
            if response["status"].as_i64().unwrap_or(-1) != 0 {
                let msg = response["msg"].as_str().unwrap_or("未知错误");
                tracing::warn!("获取存储池 {} 的存储卷失败: {}", pool_id, msg);
                continue;
            }
            
            if let Some(volumes) = response["data"]["list"].as_array() {
                tracing::debug!("存储池 {} 有 {} 个存储卷", pool_name, volumes.len());
                all_volumes.extend(volumes.clone());
            }
        }
        
        info!("共获取 {} 个存储卷（来自 {} 个存储池）", all_volumes.len(), storage_pools.len());
        Ok(all_volumes)
    }

    /// 按存储池查询存储卷（用于通过文件名查找 VM）
    ///
    /// # Arguments
    /// * `storage_pool_id` - 存储池 ID
    /// * `keyword` - 关键字过滤（可选）
    /// * `page_num` - 页码
    /// * `page_size` - 每页数量
    ///
    /// # Returns
    /// 返回分页结果，包含存储卷列表及其关联的虚拟机信息
    ///
    /// # Example Response
    /// ```json
    /// {
    ///   "status": 0,
    ///   "data": {
    ///     "list": [
    ///       {
    ///         "id": "9fed5f84-ccdb-42f6-b3bd-6be6d3930939",
    ///         "name": "disk-1",
    ///         "storagePoolId": "8f9e2b87-3237-4eac-a7d9-8e8e13ba3764",
    ///         "domainId": "cfe73063-3017-4966-bea9-ada24929e30d",
    ///         "domainName": "vm-test-01"
    ///       }
    ///     ]
    ///   }
    /// }
    /// ```
    pub async fn query_volumes_by_pool(
        &self,
        storage_pool_id: &str,
        keyword: Option<&str>,
        page_num: u32,
        page_size: u32,
    ) -> Result<serde_json::Value> {
        let keyword_param = keyword.unwrap_or("");
        info!(
            "按存储池查询存储卷: pool={}, keyword={}, page={}/{}",
            storage_pool_id, keyword_param, page_num, page_size
        );

        let url = format!(
            "/ocloud/v1/storage-volume?pageNum={}&pageSize={}&keyword={}&storagePoolId={}&snapshotFree=1",
            page_num, page_size, keyword_param, storage_pool_id
        );

        self.client.request(Method::GET, &url, None::<()>).await
    }

    /// 按存储池查询所有存储卷（自动分页获取全部）
    ///
    /// # Arguments
    /// * `storage_pool_id` - 存储池 ID
    ///
    /// # Returns
    /// 存储池下的所有存储卷列表
    pub async fn list_volumes_by_pool(&self, storage_pool_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("查询存储池 {} 下的所有存储卷", storage_pool_id);

        let mut all_volumes = Vec::new();
        let mut page_num = 1;
        let page_size = 100;

        loop {
            let response = self
                .query_volumes_by_pool(storage_pool_id, None, page_num, page_size)
                .await?;

            let data = &response["data"];
            let list = data["list"].as_array();

            if let Some(volumes) = list {
                if volumes.is_empty() {
                    break;
                }
                all_volumes.extend(volumes.clone());

                // 检查是否还有更多页
                let total = data["total"].as_u64().unwrap_or(0) as usize;
                if all_volumes.len() >= total {
                    break;
                }
                page_num += 1;
            } else {
                break;
            }
        }

        info!(
            "存储池 {} 共有 {} 个存储卷",
            storage_pool_id,
            all_volumes.len()
        );
        Ok(all_volumes)
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

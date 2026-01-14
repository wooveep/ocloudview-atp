//! 存储操作服务
//!
//! 统一封装磁盘位置查询和脑裂修复的核心业务逻辑：
//! - 磁盘位置查询（disk_location）
//! - Gluster 脑裂修复（heal_splitbrain）
//! - 共享 SSH 连接管理器，实现连接复用

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use atp_gluster::{GlusterClient, GlusterFileLocation, SplitBrainEntry, SplitBrainInfo};
use atp_vdiplatform::{DiskInfo, VdiClient};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::ssh_manager::{SshConnectionManager, SshParams};

// ============================================
// 磁盘位置查询数据结构
// ============================================

/// 磁盘位置查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskLocationResult {
    /// 虚拟机名称
    pub domain_name: String,
    /// 虚拟机 ID
    pub domain_id: String,
    /// 磁盘列表及位置信息
    pub disks: Vec<DiskLocationInfo>,
}

/// 单个磁盘的位置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskLocationInfo {
    /// 磁盘基本信息
    pub disk: DiskInfo,
    /// Gluster 位置（如果是 Gluster 存储且成功查询）
    pub gluster_location: Option<GlusterFileLocation>,
    /// 查询错误（如果失败）
    pub error: Option<String>,
}

// ============================================
// 脑裂修复数据结构
// ============================================

/// 修复策略枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealStrategy {
    /// 预览模式 - 仅显示信息，不执行修复
    DryRun,
    /// 交互模式 - 每个文件询问用户选择
    Interactive,
    /// 自动模式 - 自动选择最佳副本
    Auto,
}

/// 受影响的 VM 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedVm {
    /// VM ID
    pub id: String,
    /// VM 名称
    pub name: String,
    /// VM 状态字符串
    pub status: String,
    /// VM 状态码（1=运行中, 2=关机, 3=暂停等）
    pub status_code: i64,
    /// 所在主机 ID
    pub host_id: String,
    /// 所在主机名称
    pub host_name: String,
    /// 关联的磁盘名称
    pub disk_name: String,
}

/// 副本统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaStat {
    /// 副本索引（1-based）
    pub index: usize,
    /// 主机 IP
    pub host: String,
    /// 文件完整路径
    pub full_path: String,
    /// 文件大小（字节）
    pub size: Option<u64>,
    /// 修改时间
    pub mtime: Option<String>,
}

/// 单个脑裂条目的处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealEntryResult {
    /// 成功修复
    Success {
        path: String,
        discarded_replica: String,
    },
    /// 跳过（用户选择或预览模式）
    Skipped {
        path: String,
        reason: String,
    },
    /// 失败
    Failed {
        path: String,
        error: String,
    },
}

/// 脑裂修复报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealReport {
    /// Gluster 卷名
    pub volume_name: String,
    /// 总条目数
    pub total_entries: usize,
    /// 成功数量
    pub success_count: usize,
    /// 跳过数量
    pub skip_count: usize,
    /// 失败数量
    pub fail_count: usize,
    /// 详细结果列表
    pub results: Vec<HealEntryResult>,
}

impl HealReport {
    /// 创建新的修复报告
    pub fn new(volume_name: impl Into<String>, total_entries: usize) -> Self {
        Self {
            volume_name: volume_name.into(),
            total_entries,
            success_count: 0,
            skip_count: 0,
            fail_count: 0,
            results: Vec::new(),
        }
    }

    /// 添加成功结果
    pub fn add_success(&mut self, path: impl Into<String>, discarded_replica: impl Into<String>) {
        self.success_count += 1;
        self.results.push(HealEntryResult::Success {
            path: path.into(),
            discarded_replica: discarded_replica.into(),
        });
    }

    /// 添加跳过结果
    pub fn add_skipped(&mut self, path: impl Into<String>, reason: impl Into<String>) {
        self.skip_count += 1;
        self.results.push(HealEntryResult::Skipped {
            path: path.into(),
            reason: reason.into(),
        });
    }

    /// 添加失败结果
    pub fn add_failed(&mut self, path: impl Into<String>, error: impl Into<String>) {
        self.fail_count += 1;
        self.results.push(HealEntryResult::Failed {
            path: path.into(),
            error: error.into(),
        });
    }
}

// ============================================
// 副本选择器 Trait
// ============================================

/// 副本选择器 trait
///
/// 用于脑裂修复时选择要舍弃的副本
#[async_trait]
pub trait ReplicaSelector: Send + Sync {
    /// 选择要舍弃的副本
    ///
    /// # Arguments
    /// * `entry` - 脑裂条目
    /// * `replica_stats` - 副本统计信息列表
    /// * `affected_vm` - 受影响的 VM 信息（如果能找到）
    /// * `vm_host_ip` - VM 运行主机的 IP（如果能确定）
    ///
    /// # Returns
    /// 返回要舍弃的副本索引（1-based），返回 None 表示跳过此条目
    async fn select_discard_replica(
        &self,
        entry: &SplitBrainEntry,
        replica_stats: &[ReplicaStat],
        affected_vm: Option<&AffectedVm>,
        vm_host_ip: Option<&str>,
    ) -> Option<usize>;
}

/// 自动选择器：优先舍弃不是 VM 运行主机的副本
pub struct AutoReplicaSelector;

#[async_trait]
impl ReplicaSelector for AutoReplicaSelector {
    async fn select_discard_replica(
        &self,
        _entry: &SplitBrainEntry,
        replica_stats: &[ReplicaStat],
        _affected_vm: Option<&AffectedVm>,
        vm_host_ip: Option<&str>,
    ) -> Option<usize> {
        // 优先选择不是 VM 运行主机的副本
        let choice = replica_stats
            .iter()
            .find(|stat| vm_host_ip.map_or(true, |ip| stat.host != ip))
            .map(|stat| stat.index)
            .unwrap_or(2); // 默认舍弃第二个

        Some(choice)
    }
}

/// 交互式选择器：通过回调函数询问用户
pub struct InteractiveReplicaSelector<F>
where
    F: Fn(&SplitBrainEntry, &[ReplicaStat], Option<&AffectedVm>) -> Option<usize> + Send + Sync,
{
    selector_fn: F,
}

impl<F> InteractiveReplicaSelector<F>
where
    F: Fn(&SplitBrainEntry, &[ReplicaStat], Option<&AffectedVm>) -> Option<usize> + Send + Sync,
{
    /// 创建交互式选择器
    pub fn new(selector_fn: F) -> Self {
        Self { selector_fn }
    }
}

#[async_trait]
impl<F> ReplicaSelector for InteractiveReplicaSelector<F>
where
    F: Fn(&SplitBrainEntry, &[ReplicaStat], Option<&AffectedVm>) -> Option<usize> + Send + Sync,
{
    async fn select_discard_replica(
        &self,
        entry: &SplitBrainEntry,
        replica_stats: &[ReplicaStat],
        affected_vm: Option<&AffectedVm>,
        _vm_host_ip: Option<&str>,
    ) -> Option<usize> {
        (self.selector_fn)(entry, replica_stats, affected_vm)
    }
}

// ============================================
// 存储操作服务
// ============================================

/// 存储操作服务（磁盘位置查询 + 脑裂修复）
///
/// 共享 SSH 连接管理器，实现连接复用
pub struct StorageOpsService {
    vdi_client: Arc<VdiClient>,
    ssh_manager: SshConnectionManager,
}

impl StorageOpsService {
    /// 创建新的存储操作服务
    pub fn new(vdi_client: Arc<VdiClient>, ssh_manager: SshConnectionManager) -> Self {
        Self {
            vdi_client,
            ssh_manager,
        }
    }

    // ==================== 磁盘位置查询 ====================

    /// 查询虚拟机磁盘位置
    pub async fn query_disk_location(
        &mut self,
        vm_id_or_name: &str,
        ssh_params: &SshParams,
    ) -> Result<DiskLocationResult> {
        // 1. 查找虚拟机
        let (domain_id, domain_name, disks) = self.find_domain(vm_id_or_name).await?;

        if disks.is_empty() {
            return Ok(DiskLocationResult {
                domain_name,
                domain_id,
                disks: Vec::new(),
            });
        }

        // 2. 收集所有 Gluster 磁盘的存储池 ID
        let gluster_pool_ids: std::collections::HashSet<String> = disks
            .iter()
            .filter(|d| d.is_gluster())
            .map(|d| d.storage_pool_id.clone())
            .collect();

        // 3. 为每个存储池建立 Gluster 客户端连接
        let mut gluster_clients: HashMap<String, Option<GlusterClient>> = HashMap::new();

        for pool_id in &gluster_pool_ids {
            match self.connect_gluster_for_pool(pool_id, ssh_params).await {
                Ok(client) => {
                    gluster_clients.insert(pool_id.clone(), Some(client));
                }
                Err(e) => {
                    warn!("存储池 {} Gluster 连接失败: {}", pool_id, e);
                    gluster_clients.insert(pool_id.clone(), None);
                }
            }
        }

        // 4. 查询每个磁盘的位置信息
        let mut disk_infos = Vec::new();

        for disk in disks {
            let mut info = DiskLocationInfo {
                disk: disk.clone(),
                gluster_location: None,
                error: None,
            };

            if disk.is_gluster() {
                if let Some(Some(client)) = gluster_clients.get(&disk.storage_pool_id) {
                    match self.query_gluster_location(&disk, client).await {
                        Ok(location) => {
                            info.gluster_location = Some(location);
                        }
                        Err(e) => {
                            info.error = Some(format!("查询 Gluster 位置失败: {}", e));
                        }
                    }
                }
            }

            disk_infos.push(info);
        }

        Ok(DiskLocationResult {
            domain_name,
            domain_id,
            disks: disk_infos,
        })
    }

    /// 查找虚拟机
    async fn find_domain(&self, vm_id_or_name: &str) -> Result<(String, String, Vec<DiskInfo>)> {
        let domains = self.vdi_client.domain().list_all().await?;
        let domain = domains
            .iter()
            .find(|d| {
                d["id"].as_str() == Some(vm_id_or_name)
                    || d["name"].as_str() == Some(vm_id_or_name)
            })
            .with_context(|| format!("未找到虚拟机: {}", vm_id_or_name))?;

        let domain_id = domain["id"].as_str().unwrap_or("").to_string();
        let domain_name = domain["name"].as_str().unwrap_or("").to_string();

        // 获取磁盘信息
        let disk_values = self.vdi_client.domain().get_disks(&domain_id).await?;
        let disks: Vec<DiskInfo> = disk_values
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();

        Ok((domain_id, domain_name, disks))
    }

    /// 获取存储池关联主机 IP
    async fn get_storage_pool_hosts(&self, pool_id: &str) -> Result<Vec<String>> {
        // 查询存储池详情
        let pool_detail = self.vdi_client.storage().get_pool(pool_id).await?;
        let data = &pool_detail["data"];
        let resource_pool_id = data["poolId"].as_str().unwrap_or("").to_string();

        if resource_pool_id.is_empty() {
            bail!("存储池 {} 没有关联资源池", pool_id);
        }

        // 查询关联主机
        let hosts = self
            .vdi_client
            .host()
            .list_by_pool_id(&resource_pool_id)
            .await?;

        let host_ips: Vec<String> = hosts
            .iter()
            .filter_map(|h| h["ip"].as_str().map(|s| s.to_string()))
            .filter(|ip| !ip.is_empty())
            .collect();

        Ok(host_ips)
    }

    /// 为存储池建立 Gluster 客户端连接
    async fn connect_gluster_for_pool(
        &mut self,
        pool_id: &str,
        ssh_params: &SshParams,
    ) -> Result<GlusterClient> {
        let host_ips = self.get_storage_pool_hosts(pool_id).await?;

        if host_ips.is_empty() {
            bail!("存储池 {} 没有关联主机", pool_id);
        }

        // 尝试连接到任意一个主机
        for ip in &host_ips {
            match self.ssh_manager.create_gluster_client(ip, ssh_params).await {
                Ok(client) => {
                    info!("Gluster 连接成功: {} (存储池 {})", ip, pool_id);
                    return Ok(client);
                }
                Err(e) => {
                    warn!("Gluster 连接失败 {}: {}", ip, e);
                }
            }
        }

        bail!("存储池 {} 所有主机连接失败", pool_id)
    }

    /// 查询单个磁盘的 Gluster 位置
    async fn query_gluster_location(
        &self,
        disk: &DiskInfo,
        client: &GlusterClient,
    ) -> Result<GlusterFileLocation> {
        client
            .get_file_location(&disk.vol_full_path)
            .await
            .with_context(|| format!("查询磁盘 {} Gluster 位置失败", disk.name))
    }

    // ==================== 脑裂修复 ====================

    /// 执行脑裂检测和修复
    pub async fn heal_splitbrain<S: ReplicaSelector>(
        &mut self,
        pool_id: &str,
        ssh_params: &SshParams,
        strategy: HealStrategy,
        replica_selector: &S,
    ) -> Result<HealReport> {
        // 1. 获取存储池信息
        let pool_detail = self.vdi_client.storage().get_pool(pool_id).await?;
        let data = &pool_detail["data"];
        let volume_name = data["sourceName"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if volume_name.is_empty() {
            bail!("无法获取存储池 {} 的卷名", pool_id);
        }

        // 2. 获取关联主机并建立连接
        let host_ips = self.get_storage_pool_hosts(pool_id).await?;
        if host_ips.is_empty() {
            bail!("存储池 {} 没有关联主机", pool_id);
        }

        // 使用 connect_gluster_for_pool 获取 primary_client
        let primary_client = self.connect_gluster_for_pool(pool_id, ssh_params).await?;

        // 建立到所有主机的 Gluster 连接（要求全部成功）
        let mut host_clients: HashMap<String, GlusterClient> = HashMap::new();
        for ip in &host_ips {
            let client = self
                .ssh_manager
                .create_gluster_client(ip, ssh_params)
                .await
                .with_context(|| format!("连接主机 {} 失败", ip))?;
            host_clients.insert(ip.clone(), client);
            info!("已连接: {}", ip);
        }

        // 3. 检测脑裂文件
        let split_brain_info = primary_client.check_split_brain(&volume_name).await?;

        if !split_brain_info.has_split_brain() {
            return Ok(HealReport::new(&volume_name, 0));
        }

        // 4. 获取主机映射（用于查找 VM 运行主机）
        let host_id_to_ip = self.build_host_id_to_ip_map().await?;
        let host_id_to_name = self.build_host_id_to_name_map().await?;

        // 5. 获取存储池下的所有存储卷
        let volumes = self
            .vdi_client
            .storage()
            .list_volumes_by_pool(pool_id)
            .await?;

        // 6. 处理每个脑裂条目
        let total_entries = split_brain_info.entry_count();
        let mut report = HealReport::new(&volume_name, total_entries);

        for entry in &split_brain_info.entries {
            let result = self
                .process_splitbrain_entry(
                    entry,
                    &volume_name,
                    &host_clients,
                    &volumes,
                    &host_id_to_name,
                    &host_id_to_ip,
                    strategy,
                    replica_selector,
                )
                .await;

            match result {
                Ok(entry_result) => match entry_result {
                    HealEntryResult::Success { path, discarded_replica } => {
                        report.add_success(path, discarded_replica);
                    }
                    HealEntryResult::Skipped { path, reason } => {
                        report.add_skipped(path, reason);
                    }
                    HealEntryResult::Failed { path, error } => {
                        report.add_failed(path, error);
                    }
                },
                Err(e) => {
                    report.add_failed(entry.effective_path(), e.to_string());
                }
            }
        }

        Ok(report)
    }

    /// 仅检测脑裂状态（不修复）
    pub async fn detect_splitbrain(
        &mut self,
        pool_id: &str,
        ssh_params: &SshParams,
    ) -> Result<SplitBrainInfo> {
        // 获取存储池信息
        let pool_detail = self.vdi_client.storage().get_pool(pool_id).await?;
        let data = &pool_detail["data"];
        let volume_name = data["volumePath"]
            .as_str()
            .unwrap_or("")
            .trim_start_matches('/')
            .to_string();

        if volume_name.is_empty() {
            bail!("无法获取存储池 {} 的卷名", pool_id);
        }

        // 连接到一个主机
        let client = self.connect_gluster_for_pool(pool_id, ssh_params).await?;
        client.check_split_brain(&volume_name).await.map_err(Into::into)
    }

    /// 处理单个脑裂条目
    #[allow(clippy::too_many_arguments)]
    async fn process_splitbrain_entry<S: ReplicaSelector>(
        &self,
        entry: &SplitBrainEntry,
        volume_name: &str,
        host_clients: &HashMap<String, GlusterClient>,
        volumes: &[serde_json::Value],
        host_id_to_name: &HashMap<String, String>,
        host_id_to_ip: &HashMap<String, String>,
        strategy: HealStrategy,
        replica_selector: &S,
    ) -> Result<HealEntryResult> {
        let file_path = entry.effective_path();

        // 1. 提取磁盘名称
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);
        let disk_name = file_name
            .trim_end_matches(".qcow2")
            .trim_end_matches(".raw");

        // 2. 查找受影响的 VM
        let affected_vm = self
            .find_affected_vm(volumes, disk_name, host_id_to_name)
            .await?;

        // 3. 获取副本统计信息
        let replica_stats = self.get_replica_stats(entry, host_clients).await;

        if replica_stats.len() < 2 {
            return Ok(HealEntryResult::Skipped {
                path: file_path.to_string(),
                reason: "副本数量不足".to_string(),
            });
        }

        // 4. 获取 VM 运行主机 IP
        let vm_host_ip = affected_vm
            .as_ref()
            .and_then(|vm| host_id_to_ip.get(&vm.host_id))
            .map(|s| s.as_str());

        // 5. 预览模式直接返回
        if strategy == HealStrategy::DryRun {
            return Ok(HealEntryResult::Skipped {
                path: file_path.to_string(),
                reason: "预览模式".to_string(),
            });
        }

        // 6. 选择要舍弃的副本
        let discard_idx = replica_selector
            .select_discard_replica(entry, &replica_stats, affected_vm.as_ref(), vm_host_ip)
            .await;

        let discard_idx = match discard_idx {
            Some(idx) => idx,
            None => {
                return Ok(HealEntryResult::Skipped {
                    path: file_path.to_string(),
                    reason: "用户跳过".to_string(),
                });
            }
        };

        // 7. 获取要舍弃的副本信息
        let discard_replica = entry
            .brick_locations
            .get(discard_idx - 1)
            .with_context(|| format!("无效的副本索引: {}", discard_idx))?;

        // 8. 执行修复
        self.execute_heal(entry, volume_name, &discard_replica.host, host_clients)
            .await?;

        Ok(HealEntryResult::Success {
            path: file_path.to_string(),
            discarded_replica: discard_replica.full_location(),
        })
    }

    /// 查找受影响的 VM
    async fn find_affected_vm(
        &self,
        volumes: &[serde_json::Value],
        disk_name: &str,
        host_id_to_name: &HashMap<String, String>,
    ) -> Result<Option<AffectedVm>> {
        // 在存储卷中查找匹配的磁盘
        for vol in volumes {
            let vol_name = vol["name"].as_str().unwrap_or("");

            if vol_name == disk_name || vol_name.contains(disk_name) || disk_name.contains(vol_name)
            {
                let domain_id = vol["domainId"].as_str().unwrap_or("");
                if domain_id.is_empty() {
                    continue;
                }

                // 获取 VM 详情
                let all_vms = self.vdi_client.domain().list_all().await?;
                let vm = all_vms.iter().find(|v| v["id"].as_str() == Some(domain_id));

                if let Some(vm) = vm {
                    let host_id = vm["hostId"].as_str().unwrap_or("").to_string();
                    let host_name = host_id_to_name
                        .get(&host_id)
                        .cloned()
                        .unwrap_or_else(|| host_id.clone());

                    let status_code = vm["status"].as_i64().unwrap_or(-1);
                    let status = match status_code {
                        1 => "运行中",
                        2 => "关机",
                        3 => "暂停",
                        4 => "待机",
                        5 => "休眠",
                        _ => "未知",
                    }
                    .to_string();

                    return Ok(Some(AffectedVm {
                        id: domain_id.to_string(),
                        name: vm["name"].as_str().unwrap_or("").to_string(),
                        status,
                        status_code,
                        host_id,
                        host_name,
                        disk_name: vol_name.to_string(),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// 获取副本统计信息
    async fn get_replica_stats(
        &self,
        entry: &SplitBrainEntry,
        host_clients: &HashMap<String, GlusterClient>,
    ) -> Vec<ReplicaStat> {
        let mut stats = Vec::new();

        for (i, loc) in entry.brick_locations.iter().enumerate() {
            let mut stat = ReplicaStat {
                index: i + 1,
                host: loc.host.clone(),
                full_path: loc.full_path.clone(),
                size: None,
                mtime: None,
            };

            // 尝试获取文件统计信息
            if let Some(client) = host_clients.get(&loc.host) {
                if let Ok(file_stat) = client.get_file_stat(&loc.full_path).await {
                    stat.size = Some(file_stat.size);
                    stat.mtime = Some(file_stat.mtime);
                }
            }

            stats.push(stat);
        }

        stats
    }

    /// 执行修复操作
    async fn execute_heal(
        &self,
        entry: &SplitBrainEntry,
        volume_name: &str,
        discard_host: &str,
        host_clients: &HashMap<String, GlusterClient>,
    ) -> Result<()> {
        let discard_loc = entry
            .brick_locations
            .iter()
            .find(|loc| loc.host == discard_host)
            .context("找不到要舍弃的副本")?;

        let gluster_client = host_clients
            .get(discard_host)
            .context(format!("未连接到主机 {}", discard_host))?;

        // 清除 AFR 属性
        let removed_count = gluster_client
            .remove_all_afr_attributes(&discard_loc.full_path)
            .await
            .context("清除 AFR 属性失败")?;

        debug!(
            "已清除 {} 上的 {} 个 AFR 属性",
            discard_host, removed_count
        );

        // 触发卷修复
        let any_client = host_clients.values().next().context("无可用客户端")?;
        any_client
            .trigger_heal(volume_name)
            .await
            .context("触发卷修复失败")?;

        // 等待修复完成
        any_client
            .wait_for_heal(volume_name, 10, 5)
            .await
            .context("等待修复完成失败")?;

        Ok(())
    }

    /// 构建主机 ID 到名称的映射
    async fn build_host_id_to_name_map(&self) -> Result<HashMap<String, String>> {
        let hosts = self.vdi_client.host().list_all().await?;
        let mut map = HashMap::new();

        for host in &hosts {
            if let (Some(id), Some(name)) = (host["id"].as_str(), host["name"].as_str()) {
                map.insert(id.to_string(), name.to_string());
            }
        }

        Ok(map)
    }

    /// 构建主机 ID 到 IP 的映射
    async fn build_host_id_to_ip_map(&self) -> Result<HashMap<String, String>> {
        let hosts = self.vdi_client.host().list_all().await?;
        let mut map = HashMap::new();

        for host in &hosts {
            if let (Some(id), Some(ip)) = (host["id"].as_str(), host["ip"].as_str()) {
                map.insert(id.to_string(), ip.to_string());
            }
        }

        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atp_gluster::BrickLocation;

    #[test]
    fn test_heal_strategy() {
        assert_eq!(HealStrategy::DryRun, HealStrategy::DryRun);
        assert_ne!(HealStrategy::Auto, HealStrategy::Interactive);
    }

    #[test]
    fn test_heal_report() {
        let mut report = HealReport::new("gv0", 3);
        report.add_success("/path/1", "host1:/brick/1");
        report.add_skipped("/path/2", "用户跳过");
        report.add_failed("/path/3", "连接失败");

        assert_eq!(report.success_count, 1);
        assert_eq!(report.skip_count, 1);
        assert_eq!(report.fail_count, 1);
        assert_eq!(report.results.len(), 3);
    }

    #[test]
    fn test_replica_stat() {
        let stat = ReplicaStat {
            index: 1,
            host: "192.168.1.1".to_string(),
            full_path: "/data/brick1/file.qcow2".to_string(),
            size: Some(1024),
            mtime: Some("2024-01-01".to_string()),
        };

        assert_eq!(stat.index, 1);
        assert_eq!(stat.size, Some(1024));
    }

    #[tokio::test]
    async fn test_auto_replica_selector() {
        let selector = AutoReplicaSelector;
        let entry = SplitBrainEntry::regular_file("/test/file.qcow2", vec![
            BrickLocation::new("192.168.1.1", "/brick1", "/brick1/file.qcow2"),
            BrickLocation::new("192.168.1.2", "/brick2", "/brick2/file.qcow2"),
        ]);

        let stats = vec![
            ReplicaStat {
                index: 1,
                host: "192.168.1.1".to_string(),
                full_path: "/brick1/file.qcow2".to_string(),
                size: None,
                mtime: None,
            },
            ReplicaStat {
                index: 2,
                host: "192.168.1.2".to_string(),
                full_path: "/brick2/file.qcow2".to_string(),
                size: None,
                mtime: None,
            },
        ];

        // VM 在 192.168.1.1 上运行，应选择舍弃 192.168.1.2 的副本
        let result = selector
            .select_discard_replica(&entry, &stats, None, Some("192.168.1.1"))
            .await;
        assert_eq!(result, Some(2));
    }
}

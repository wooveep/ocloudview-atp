//! VDI 批量操作核心逻辑
//!
//! 本模块包含 VDI 平台批量操作的核心业务逻辑，与 CLI 层解耦。
//! CLI 层仅负责参数解析、用户交互和输出格式化。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Serialize;
use tracing::{debug, info, warn};
use virt::domain::Domain;

use atp_protocol::qga::QgaProtocol;
use atp_protocol::Protocol;
use atp_storage::Storage;
use atp_transport::{HostInfo, TransportError, TransportManager};
use atp_vdiplatform::{
    AssignmentPlan, BatchTaskRequest, DomainStatus, RenamePlan, VdiClient, VmMatchResult,
};

// ============================================================================
// Traits - 抽象接口定义
// ============================================================================

/// VM 匹配器 trait - 用于匹配虚拟机名称
pub trait VmMatcher: Send + Sync {
    /// 检查名称是否匹配模式
    fn matches(&self, name: &str, pattern: &str) -> bool;
}

/// QGA 验证器 trait - 用于通过 QGA 验证虚拟机状态
#[async_trait]
pub trait QgaVerifier: Send + Sync {
    /// 验证单个虚拟机的 QGA 连接
    async fn verify_single(
        &self,
        domain: &Domain,
        vm_name: &str,
        max_retries: u32,
        retry_delay_secs: u64,
    ) -> Result<()>;
}

/// 批量操作接口 trait
#[async_trait]
pub trait BatchOperations: Send + Sync {
    /// 获取匹配的虚拟机列表
    async fn get_matching_vms(&self, pattern: &str) -> Result<Vec<VmMatchResult>>;

    /// 批量启动虚拟机
    async fn batch_start(&self, vms: &[VmMatchResult], verify: bool) -> Result<BatchStartResult>;

    /// 批量验证虚拟机 QGA 状态
    async fn batch_verify_qga(
        &self,
        vms: &[VmInfo],
        max_retries: u32,
        retry_delay_secs: u64,
    ) -> Vec<QgaVerifyResult>;

    /// 批量分配虚拟机给用户
    async fn batch_assign(&self, plans: &[AssignmentPlan]) -> Result<BatchAssignResult>;

    /// 批量重命名虚拟机为绑定用户名
    async fn batch_rename(&self, plans: &[RenamePlan]) -> Result<BatchRenameResult>;

    /// 批量设置 autoJoinDomain (自动加域)
    async fn batch_set_auto_ad(
        &self,
        vms: &[VmMatchResult],
        enable: bool,
    ) -> Result<BatchAutoAdResult>;
}

// ============================================================================
// 数据结构定义
// ============================================================================

/// 虚拟机信息（用于 QGA 验证）
#[derive(Debug, Clone)]
pub struct VmInfo {
    /// 虚拟机名称
    pub vm_name: String,
    /// 主机名称
    pub host_name: String,
    /// 主机 ID
    pub host_id: String,
    /// 主机 IP
    pub host_ip: String,
}

impl VmInfo {
    pub fn new(vm_name: &str, host_name: &str, host_id: &str, host_ip: &str) -> Self {
        Self {
            vm_name: vm_name.to_string(),
            host_name: host_name.to_string(),
            host_id: host_id.to_string(),
            host_ip: host_ip.to_string(),
        }
    }
}

/// 批量启动单个 VM 错误
#[derive(Debug, Clone)]
pub struct BatchStartError {
    /// 虚拟机 ID
    pub vm_id: String,
    /// 虚拟机名称
    pub vm_name: String,
    /// 错误信息
    pub error: String,
}

/// 批量启动结果
#[derive(Debug, Clone)]
pub struct BatchStartResult {
    /// 成功启动的数量
    pub success_count: usize,
    /// 失败的虚拟机列表
    pub failed_vms: Vec<BatchStartError>,
    /// QGA 验证结果（如果启用了验证）
    pub verification_results: Option<Vec<QgaVerifyResult>>,
}

impl BatchStartResult {
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failed_vms: Vec::new(),
            verification_results: None,
        }
    }
}

impl Default for BatchStartResult {
    fn default() -> Self {
        Self::new()
    }
}

/// QGA 验证结果
#[derive(Debug, Clone)]
pub struct QgaVerifyResult {
    /// 虚拟机名称
    pub vm_name: String,
    /// 主机名称
    pub host_name: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果失败）
    pub error_msg: Option<String>,
}

impl QgaVerifyResult {
    pub fn success(vm_name: &str, host_name: &str) -> Self {
        Self {
            vm_name: vm_name.to_string(),
            host_name: host_name.to_string(),
            success: true,
            error_msg: None,
        }
    }

    pub fn failure(vm_name: &str, host_name: &str, error: &str) -> Self {
        Self {
            vm_name: vm_name.to_string(),
            host_name: host_name.to_string(),
            success: false,
            error_msg: Some(error.to_string()),
        }
    }
}

/// Libvirt 虚拟机状态信息
#[derive(Debug, Clone, Serialize)]
pub struct LibvirtDomainState {
    /// 虚拟机名称
    pub name: String,
    /// 状态字符串 (e.g., "Running", "Shutoff", "Paused")
    pub state: String,
    /// CPU 核数
    pub cpu: u32,
    /// 内存大小 (MB)
    pub memory_mb: u64,
}

/// VDI 与 libvirt 一致性比对结果
#[derive(Debug, Clone, Serialize)]
pub struct CompareResult {
    /// 虚拟机名称
    pub vm_name: String,
    /// VDI 平台状态
    pub vdi_status: String,
    /// libvirt 状态
    pub libvirt_status: String,
    /// 是否一致
    pub consistent: bool,
    /// 所在主机
    pub host: String,
}

/// 一致性验证结果汇总
#[derive(Debug, Clone, Serialize)]
pub struct VerifyResult {
    /// 总虚拟机数
    pub total_vms: usize,
    /// 一致的虚拟机数
    pub consistent_vms: usize,
    /// 不一致的虚拟机数
    pub inconsistent_vms: usize,
    /// 详细比对结果
    pub results: Vec<CompareResult>,
}

/// 通用批量操作错误
#[derive(Debug, Clone, Serialize)]
pub struct BatchOpError {
    /// 虚拟机 ID
    pub vm_id: String,
    /// 虚拟机名称
    pub vm_name: String,
    /// 错误信息
    pub error: String,
}

/// 批量分配结果
#[derive(Debug, Clone, Serialize)]
pub struct BatchAssignResult {
    /// 成功数量
    pub success_count: usize,
    /// 错误数量
    pub error_count: usize,
    /// 错误详情
    pub errors: Vec<BatchOpError>,
}

impl Default for BatchAssignResult {
    fn default() -> Self {
        Self {
            success_count: 0,
            error_count: 0,
            errors: Vec::new(),
        }
    }
}

/// 批量重命名结果
#[derive(Debug, Clone, Serialize)]
pub struct BatchRenameResult {
    /// 成功数量
    pub success_count: usize,
    /// 错误数量
    pub error_count: usize,
    /// 错误详情
    pub errors: Vec<BatchOpError>,
}

impl Default for BatchRenameResult {
    fn default() -> Self {
        Self {
            success_count: 0,
            error_count: 0,
            errors: Vec::new(),
        }
    }
}

/// 批量设置 AutoAD 结果
#[derive(Debug, Clone, Serialize)]
pub struct BatchAutoAdResult {
    /// 成功数量
    pub success_count: usize,
    /// 错误数量
    pub error_count: usize,
    /// 错误详情
    pub errors: Vec<BatchOpError>,
}

impl Default for BatchAutoAdResult {
    fn default() -> Self {
        Self {
            success_count: 0,
            error_count: 0,
            errors: Vec::new(),
        }
    }
}

// ============================================================================
// 公共工具函数 - libvirt 状态获取
// ============================================================================

/// 注册主机到 TransportManager
///
/// 尝试将主机添加到传输管理器并等待连接建立。
/// 如果主机已注册，会静默跳过。
pub async fn ensure_host_registered(
    transport_manager: &TransportManager,
    host_id: &str,
    host_ip: &str,
) -> Result<bool> {
    let host_info = HostInfo {
        id: host_id.to_string(),
        host: host_ip.to_string(),
        uri: format!("qemu+tcp://{}/system", host_ip),
        tags: vec![],
        metadata: std::collections::HashMap::new(),
    };

    match transport_manager.add_host(host_info).await {
        Ok(_) => {
            info!("✅ 已注册主机 {} ({})", host_id, host_ip);

            // 等待连接建立（给 libvirt 连接充足的时间）
            let max_wait = Duration::from_secs(35);
            let start = Instant::now();
            let mut connected = false;
            let hid = host_id.to_string();
            let mut attempt = 0;
            let max_attempts = (max_wait.as_secs() * 2) as usize; // 每 500ms 一次尝试

            while start.elapsed() < max_wait {
                attempt += 1;

                match transport_manager
                    .execute_on_host(&hid, |conn| async move {
                        // 尝试获取连接对象并验证
                        let conn_mutex = conn.get_connection().await?;
                        let conn_guard = conn_mutex.lock().await;
                        let conn_ref = conn_guard
                            .as_ref()
                            .ok_or_else(|| TransportError::Disconnected)?;

                        // 执行简单的 libvirt 操作验证连接可用性
                        let is_alive = tokio::task::spawn_blocking({
                            let conn_clone = conn_ref.clone();
                            move || conn_clone.is_alive().unwrap_or(false)
                        })
                        .await
                        .unwrap_or(false);

                        if !is_alive {
                            return Err(TransportError::Disconnected);
                        }

                        Ok(())
                    })
                    .await
                {
                    Ok(_) => {
                        connected = true;
                        debug!(
                            "✅ 主机 {} 连接验证成功 (耗时 {:?})",
                            host_id,
                            start.elapsed()
                        );
                        break;
                    }
                    Err(e) => {
                        debug!(
                            "等待主机 {} 连接建立... (尝试 {}/{}, 错误: {})",
                            host_id, attempt, max_attempts, e
                        );
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }

            if !connected {
                warn!("⚠️  主机 {} ({}) 连接超时", host_id, host_ip);
            }

            Ok(connected)
        }
        Err(e) => {
            // 可能已经注册了，尝试连接
            let hid = host_id.to_string();
            match transport_manager
                .execute_on_host(&hid, |_conn| async move { Ok(()) })
                .await
            {
                Ok(_) => Ok(true),
                Err(_) => {
                    warn!("⚠️  无法注册主机 {} ({}): {}", host_id, host_ip, e);
                    Ok(false)
                }
            }
        }
    }
}

/// 获取单个虚拟机的 libvirt 状态
pub async fn get_domain_libvirt_state(
    transport_manager: &TransportManager,
    host_id: &str,
    domain_name: &str,
) -> Result<LibvirtDomainState> {
    let hid = host_id.to_string();
    let dname = domain_name.to_string();

    let state = transport_manager
        .execute_on_host(&hid, |conn| {
            let name = dname.clone();
            async move {
                let domain = conn.get_domain(&name).await?;
                let (st, _) = domain
                    .get_state()
                    .map_err(|e| TransportError::LibvirtError(format!("获取状态失败: {}", e)))?;
                let state_str = format!("{:?}", st);

                let (cpu, memory) = match domain.get_info() {
                    Ok(info) => (info.nr_virt_cpu, info.memory / 1024),
                    Err(_) => (0, 0),
                };

                Ok(LibvirtDomainState {
                    name,
                    state: state_str,
                    cpu,
                    memory_mb: memory,
                })
            }
        })
        .await
        .context(format!("获取虚拟机 {} 状态失败", domain_name))?;

    Ok(state)
}

/// 获取主机上所有虚拟机的 libvirt 状态
pub async fn list_host_domains_state(
    transport_manager: &TransportManager,
    host_id: &str,
) -> Result<HashMap<String, LibvirtDomainState>> {
    let hid = host_id.to_string();

    let domains = transport_manager
        .execute_on_host(&hid, |conn| async move {
            // 获取所有域（包括关闭状态的）
            // flags: VIR_CONNECT_LIST_DOMAINS_ACTIVE | VIR_CONNECT_LIST_DOMAINS_INACTIVE = 3
            let conn_mutex = conn.get_connection().await?;
            let conn_guard = conn_mutex.lock().await;
            let conn_ref = conn_guard
                .as_ref()
                .ok_or_else(|| TransportError::Disconnected)?;

            let domains = conn_ref
                .list_all_domains(3)
                .map_err(|e| TransportError::LibvirtError(format!("列出虚拟机失败: {}", e)))?;

            let mut result = HashMap::new();
            for domain in &domains {
                if let Ok(name) = domain.get_name() {
                    let state = if let Ok((st, _)) = domain.get_state() {
                        format!("{:?}", st)
                    } else {
                        "Unknown".to_string()
                    };

                    let (cpu, memory) = if let Ok(info) = domain.get_info() {
                        (info.nr_virt_cpu, info.memory / 1024)
                    } else {
                        (0, 0)
                    };

                    result.insert(
                        name.clone(),
                        LibvirtDomainState {
                            name,
                            state,
                            cpu,
                            memory_mb: memory,
                        },
                    );
                }
            }

            Ok(result)
        })
        .await
        .context(format!("获取主机 {} 虚拟机列表失败", host_id))?;

    Ok(domains)
}

// ============================================================================
// 默认实现
// ============================================================================

/// 默认 VM 匹配器实现
/// 支持: * (全部), prefix* (前缀), *suffix (后缀), *middle* (包含), exact (精确)
pub struct DefaultVmMatcher;

impl VmMatcher for DefaultVmMatcher {
    fn matches(&self, name: &str, pattern: &str) -> bool {
        matches_pattern(name, pattern)
    }
}

/// 默认 QGA 验证器实现
pub struct DefaultQgaVerifier;

#[async_trait]
impl QgaVerifier for DefaultQgaVerifier {
    async fn verify_single(
        &self,
        domain: &Domain,
        vm_name: &str,
        max_retries: u32,
        retry_delay_secs: u64,
    ) -> Result<()> {
        verify_single_vm_qga(domain, vm_name, max_retries, retry_delay_secs).await
    }
}

// ============================================================================
// VdiBatchOps - 核心批量操作实现
// ============================================================================

/// VDI 批量操作器
///
/// 封装 VDI 平台批量操作的核心逻辑，包括：
/// - 虚拟机列表获取和筛选
/// - 批量启动操作
/// - QGA 验证
pub struct VdiBatchOps {
    /// 传输管理器
    transport_manager: Arc<TransportManager>,
    /// VDI 平台客户端
    vdi_client: Arc<VdiClient>,
    /// 存储（可选，用于数据库操作）
    storage: Option<Arc<Storage>>,
    /// VM 匹配器
    vm_matcher: Box<dyn VmMatcher>,
    /// QGA 验证器
    qga_verifier: Box<dyn QgaVerifier>,
}

impl VdiBatchOps {
    /// 创建批量操作器
    pub fn new(transport_manager: Arc<TransportManager>, vdi_client: Arc<VdiClient>) -> Self {
        Self {
            transport_manager,
            vdi_client,
            storage: None,
            vm_matcher: Box::new(DefaultVmMatcher),
            qga_verifier: Box::new(DefaultQgaVerifier),
        }
    }

    /// 设置存储
    pub fn with_storage(mut self, storage: Arc<Storage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// 设置自定义 VM 匹配器
    pub fn with_vm_matcher(mut self, matcher: Box<dyn VmMatcher>) -> Self {
        self.vm_matcher = matcher;
        self
    }

    /// 设置自定义 QGA 验证器
    pub fn with_qga_verifier(mut self, verifier: Box<dyn QgaVerifier>) -> Self {
        self.qga_verifier = verifier;
        self
    }

    /// 获取主机 ID 到名称的映射
    pub async fn build_host_id_to_name_map(&self) -> Result<HashMap<String, String>> {
        let hosts = self
            .vdi_client
            .host()
            .list_all()
            .await
            .context("获取主机列表失败")?;

        let map: HashMap<String, String> = hosts
            .iter()
            .filter_map(|h| {
                let id = h["id"].as_str()?.to_string();
                let name = h["name"].as_str()?.to_string();
                Some((id, name))
            })
            .collect();

        Ok(map)
    }

    /// 获取主机 ID 到 IP 的映射
    pub async fn build_host_id_to_ip_map(&self) -> Result<HashMap<String, String>> {
        let hosts = self
            .vdi_client
            .host()
            .list_all()
            .await
            .context("获取主机列表失败")?;

        let map: HashMap<String, String> = hosts
            .iter()
            .filter_map(|h| {
                let id = h["id"].as_str()?.to_string();
                let ip = h["ip"].as_str()?.to_string();
                Some((id, ip))
            })
            .collect();

        Ok(map)
    }

    /// 注册主机到 TransportManager (委托给公共函数)
    async fn ensure_host_registered(&self, host_id: &str, host_ip: &str) -> Result<bool> {
        ensure_host_registered(&self.transport_manager, host_id, host_ip).await
    }
}

#[async_trait]
impl BatchOperations for VdiBatchOps {
    /// 获取匹配模式的虚拟机列表
    async fn get_matching_vms(&self, pattern: &str) -> Result<Vec<VmMatchResult>> {
        let host_id_to_name = self.build_host_id_to_name_map().await?;
        let domains = self
            .vdi_client
            .domain()
            .list_all()
            .await
            .context("获取虚拟机列表失败")?;

        let mut results = Vec::new();
        for domain in &domains {
            let name = domain["name"].as_str().unwrap_or("").to_string();
            if !self.vm_matcher.matches(&name, pattern) {
                continue;
            }

            let id = domain["id"].as_str().unwrap_or("").to_string();
            let status_code = domain["status"].as_i64().unwrap_or(-1);
            let status = DomainStatus::from_code(status_code)
                .display_name()
                .to_string();
            let host_id = domain["hostId"].as_str().unwrap_or("").to_string();
            let host_name = host_id_to_name.get(&host_id).cloned().unwrap_or_default();

            // 获取绑定用户信息
            let bound_user_id = domain["userId"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let bound_user = bound_user_id.clone();
            let ip = domain["ip"].as_str().map(|s| s.to_string());
            let cpu = domain["cpuNum"].as_i64();
            let memory = domain["memory"].as_i64();

            results.push(VmMatchResult {
                id,
                name,
                status,
                status_code,
                bound_user,
                bound_user_id,
                host_id,
                host_name,
                ip,
                cpu,
                memory,
            });
        }

        Ok(results)
    }

    /// 批量启动虚拟机
    async fn batch_start(&self, vms: &[VmMatchResult], verify: bool) -> Result<BatchStartResult> {
        let mut result = BatchStartResult::new();

        if vms.is_empty() {
            return Ok(result);
        }

        // 执行批量启动
        let vm_ids: Vec<String> = vms.iter().map(|vm| vm.id.clone()).collect();
        let request = BatchTaskRequest::new(vm_ids);
        let response = self
            .vdi_client
            .domain()
            .batch_start(request)
            .await
            .context("批量启动请求失败")?;

        // 统计结果
        result.success_count = vms.len() - response.error_list.len();
        for err in response.error_list {
            result.failed_vms.push(BatchStartError {
                vm_id: err.id.unwrap_or_default(),
                vm_name: String::new(), // API 不返回名称
                error: err.error.unwrap_or_else(|| "未知错误".to_string()),
            });
        }

        // QGA 验证
        if verify {
            // 等待虚拟机启动
            info!("⏳ 等待虚拟机启动 (30秒)...");
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

            let host_id_to_ip = self.build_host_id_to_ip_map().await?;

            let vms_for_verify: Vec<VmInfo> = vms
                .iter()
                .map(|vm| {
                    let host_ip = host_id_to_ip.get(&vm.host_id).cloned().unwrap_or_default();
                    VmInfo::new(&vm.name, &vm.host_name, &vm.host_id, &host_ip)
                })
                .collect();

            let verify_results = self.batch_verify_qga(&vms_for_verify, 3, 20).await;
            result.verification_results = Some(verify_results);
        }

        Ok(result)
    }

    /// 批量验证虚拟机 QGA 状态
    async fn batch_verify_qga(
        &self,
        vms: &[VmInfo],
        max_retries: u32,
        retry_delay_secs: u64,
    ) -> Vec<QgaVerifyResult> {
        // 收集唯一主机并注册到 TransportManager
        let mut registered_hosts: HashMap<String, bool> = HashMap::new();

        for vm in vms {
            if registered_hosts.contains_key(&vm.host_id) {
                continue;
            }

            let connected = self
                .ensure_host_registered(&vm.host_id, &vm.host_ip)
                .await
                .unwrap_or(false);
            registered_hosts.insert(vm.host_id.clone(), connected);
        }

        // 验证每个 VM
        let mut results = Vec::new();

        for vm in vms {
            // 检查主机是否已连接
            let host_connected = registered_hosts.get(&vm.host_id).copied().unwrap_or(false);
            if !host_connected {
                results.push(QgaVerifyResult::failure(
                    &vm.vm_name,
                    &vm.host_name,
                    &format!("主机 {} 连接失败", vm.host_id),
                ));
                continue;
            }

            // 通过 TransportManager 获取 Domain 并验证
            let tm = Arc::clone(&self.transport_manager);
            let vm_name_clone = vm.vm_name.clone();
            let host_id_clone = vm.host_id.clone();

            let domain_result = tm
                .execute_on_host(&host_id_clone, |conn| {
                    let vm_name_inner = vm_name_clone.clone();
                    async move { conn.get_domain(&vm_name_inner).await }
                })
                .await;

            match domain_result {
                Ok(domain) => {
                    // 执行 QGA 验证
                    let verify_result = self
                        .qga_verifier
                        .verify_single(&domain, &vm.vm_name, max_retries, retry_delay_secs)
                        .await;

                    match verify_result {
                        Ok(()) => {
                            results.push(QgaVerifyResult::success(&vm.vm_name, &vm.host_name))
                        }
                        Err(e) => results.push(QgaVerifyResult::failure(
                            &vm.vm_name,
                            &vm.host_name,
                            &e.to_string(),
                        )),
                    }
                }
                Err(e) => {
                    results.push(QgaVerifyResult::failure(
                        &vm.vm_name,
                        &vm.host_name,
                        &format!("获取虚拟机失败: {}", e),
                    ));
                }
            }
        }

        results
    }

    /// 批量分配虚拟机给用户
    async fn batch_assign(&self, plans: &[AssignmentPlan]) -> Result<BatchAssignResult> {
        let mut result = BatchAssignResult::default();

        for plan in plans {
            // 如果是重新分配，先解绑现有用户
            if plan.is_reassignment {
                if let Err(e) = self.vdi_client.domain().unbind_user(&plan.vm_id).await {
                    result.error_count += 1;
                    result.errors.push(BatchOpError {
                        vm_id: plan.vm_id.clone(),
                        vm_name: plan.vm_name.clone(),
                        error: format!("解绑失败: {}", e),
                    });
                    continue;
                }
            }

            // 绑定新用户
            match self
                .vdi_client
                .domain()
                .bind_user(&plan.vm_id, &plan.username)
                .await
            {
                Ok(_) => {
                    let action = if plan.is_reassignment {
                        "重新分配"
                    } else {
                        "分配"
                    };
                    info!("✅ {} {} -> {}", action, plan.vm_name, plan.username);
                    result.success_count += 1;
                }
                Err(e) => {
                    result.error_count += 1;
                    result.errors.push(BatchOpError {
                        vm_id: plan.vm_id.clone(),
                        vm_name: plan.vm_name.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(result)
    }

    /// 批量重命名虚拟机为绑定用户名
    async fn batch_rename(&self, plans: &[RenamePlan]) -> Result<BatchRenameResult> {
        let mut result = BatchRenameResult::default();

        for plan in plans {
            match self
                .vdi_client
                .domain()
                .rename(&plan.vm_id, &plan.new_name)
                .await
            {
                Ok(_) => {
                    info!("✅ {} -> {}", plan.old_name, plan.new_name);
                    result.success_count += 1;
                }
                Err(e) => {
                    result.error_count += 1;
                    result.errors.push(BatchOpError {
                        vm_id: plan.vm_id.clone(),
                        vm_name: plan.old_name.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(result)
    }

    /// 批量设置 autoJoinDomain (自动加域)
    async fn batch_set_auto_ad(
        &self,
        vms: &[VmMatchResult],
        enable: bool,
    ) -> Result<BatchAutoAdResult> {
        let mut result = BatchAutoAdResult::default();
        let target_value = if enable { 1 } else { 0 };
        let action_name = if enable { "启用" } else { "禁用" };

        for vm in vms {
            match self
                .vdi_client
                .domain()
                .set_auto_join_domain(&vm.id, target_value)
                .await
            {
                Ok(_) => {
                    info!("✅ {} - {} 自动加域", vm.name, action_name);
                    result.success_count += 1;
                }
                Err(e) => {
                    result.error_count += 1;
                    result.errors.push(BatchOpError {
                        vm_id: vm.id.clone(),
                        vm_name: vm.name.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(result)
    }
}

// ============================================================================
// 工具函数
// ============================================================================

/// 模式匹配函数
/// 支持: * (全部), prefix* (前缀), *suffix (后缀), *middle* (包含), exact (精确)
pub fn matches_pattern(name: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let starts_with_star = pattern.starts_with('*');
    let ends_with_star = pattern.ends_with('*');

    match (starts_with_star, ends_with_star) {
        (true, true) if pattern.len() > 2 => {
            // *middle* - 包含匹配
            let middle = &pattern[1..pattern.len() - 1];
            name.contains(middle)
        }
        (true, true) => {
            // 只有 ** 或 * 的情况
            true
        }
        (true, false) => {
            // *suffix - 后缀匹配
            let suffix = &pattern[1..];
            name.ends_with(suffix)
        }
        (false, true) => {
            // prefix* - 前缀匹配
            let prefix = &pattern[..pattern.len() - 1];
            name.starts_with(prefix)
        }
        (false, false) => {
            // exact - 精确匹配
            name == pattern
        }
    }
}

/// 验证单个虚拟机的 QGA 连接
async fn verify_single_vm_qga(
    domain: &Domain,
    vm_name: &str,
    max_retries: u32,
    retry_delay_secs: u64,
) -> Result<()> {
    info!("验证虚拟机 {} QGA 连接", vm_name);

    // 重试 QGA ping
    for attempt in 1..=max_retries {
        debug!("QGA ping 尝试 {}/{} - {}", attempt, max_retries, vm_name);

        let mut qga = QgaProtocol::new().with_timeout(10);
        match qga.connect(domain).await {
            Ok(()) => {
                // connect 成功意味着 ping 也成功了
                info!("✅ {} - QGA 验证成功", vm_name);
                return Ok(());
            }
            Err(e) => {
                warn!(
                    "⚠️  {} - QGA 连接失败 (尝试 {}/{}): {}",
                    vm_name, attempt, max_retries, e
                );
                if attempt < max_retries {
                    debug!("等待 {} 秒后重试...", retry_delay_secs);
                    tokio::time::sleep(tokio::time::Duration::from_secs(retry_delay_secs)).await;
                }
            }
        }
    }

    anyhow::bail!("QGA 验证失败 (已重试 {} 次)", max_retries)
}

// ============================================================================
// VdiVerifyOps - VDI 与 libvirt 一致性验证
// ============================================================================

/// VDI 与 libvirt 一致性验证器
///
/// 封装验证 VDI 平台与 libvirt 虚拟机状态一致性的核心逻辑
pub struct VdiVerifyOps {
    /// 传输管理器
    transport_manager: Arc<TransportManager>,
    /// VDI 平台客户端
    vdi_client: Arc<VdiClient>,
}

impl VdiVerifyOps {
    /// 创建验证器
    pub fn new(transport_manager: Arc<TransportManager>, vdi_client: Arc<VdiClient>) -> Self {
        Self {
            transport_manager,
            vdi_client,
        }
    }

    /// 获取主机 ID 到名称的映射
    async fn build_host_maps(&self) -> Result<(HashMap<String, String>, HashMap<String, String>)> {
        let hosts = self
            .vdi_client
            .host()
            .list_all()
            .await
            .context("获取主机列表失败")?;

        let mut id_to_name = HashMap::new();
        let mut id_to_ip = HashMap::new();

        for host in &hosts {
            if let (Some(id), Some(name), Some(ip)) = (
                host["id"].as_str(),
                host["name"].as_str(),
                host["ip"].as_str(),
            ) {
                id_to_name.insert(id.to_string(), name.to_string());
                id_to_ip.insert(id.to_string(), ip.to_string());
            }
        }

        Ok((id_to_name, id_to_ip))
    }

    /// 执行一致性验证
    ///
    /// 比对 VDI 平台与 libvirt 的虚拟机状态
    pub async fn verify_consistency(&self) -> Result<VerifyResult> {
        info!("开始 VDI 与 libvirt 一致性验证");

        // 1. 获取 VDI 主机和虚拟机信息
        // let (host_id_to_name, host_id_to_ip) = self.build_host_maps().await?;
        let hosts = self.vdi_client.host().list_all().await?;
        let vdi_domains = self.vdi_client.domain().list_all().await?;

        // 2. 构建 VDI 虚拟机状态映射
        let mut vdi_vms: HashMap<String, (String, String)> = HashMap::new(); // name -> (status, host_id)
        for domain in &vdi_domains {
            let name = domain["name"].as_str().unwrap_or("").to_string();
            let status_code = domain["status"].as_i64().unwrap_or(-1);
            let status = DomainStatus::from_code(status_code)
                .display_name()
                .to_string();
            let host_id = domain["hostId"].as_str().unwrap_or("").to_string();

            if !name.is_empty() {
                vdi_vms.insert(name, (status, host_id));
            }
        }

        // 3. 遍历主机，连接 libvirt 获取状态并比对
        let mut results = Vec::new();
        let mut total_vms = 0;
        let mut consistent_vms = 0;
        let mut inconsistent_vms = 0;

        for host in &hosts {
            let host_id = host["id"].as_str().unwrap_or("");
            let host_name = host["name"].as_str().unwrap_or("");
            let host_ip = host["ip"].as_str().unwrap_or("");
            let host_status = host["status"].as_i64().unwrap_or(-1);

            // 跳过离线主机 (status != 1)
            if host_status != 1 {
                debug!("跳过离线主机: {} (status={})", host_name, host_status);
                continue;
            }

            // 注册主机到 TransportManager
            let connected =
                ensure_host_registered(&self.transport_manager, host_id, host_ip).await?;
            if !connected {
                warn!("无法连接主机 {}, 跳过", host_name);
                continue;
            }

            // 获取 libvirt 虚拟机状态
            let libvirt_vms = match list_host_domains_state(&self.transport_manager, host_id).await
            {
                Ok(vms) => vms,
                Err(e) => {
                    warn!("获取主机 {} libvirt 状态失败: {}", host_name, e);
                    continue;
                }
            };

            // 比对状态
            for (vm_name, libvirt_state) in &libvirt_vms {
                total_vms += 1;

                let (vdi_status, consistent) = if let Some((status, _)) = vdi_vms.get(vm_name) {
                    let is_consistent = Self::compare_status(status, &libvirt_state.state);
                    (status.clone(), is_consistent)
                } else {
                    ("不存在".to_string(), false)
                };

                if consistent {
                    consistent_vms += 1;
                } else {
                    inconsistent_vms += 1;
                }

                results.push(CompareResult {
                    vm_name: vm_name.clone(),
                    vdi_status,
                    libvirt_status: libvirt_state.state.clone(),
                    consistent,
                    host: host_name.to_string(),
                });
            }
        }

        info!(
            "验证完成: 总数={}, 一致={}, 不一致={}",
            total_vms, consistent_vms, inconsistent_vms
        );

        Ok(VerifyResult {
            total_vms,
            consistent_vms,
            inconsistent_vms,
            results,
        })
    }

    /// 比较 VDI 状态与 libvirt 状态是否一致
    fn compare_status(vdi_status: &str, libvirt_state: &str) -> bool {
        match (vdi_status, libvirt_state) {
            ("运行中", "Running") | ("运行中", "1") => true,
            ("挂起", "Paused") | ("挂起", "3") => true,
            ("关机", "Shutoff") | ("关机", "5") => true,
            _ => false,
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern_exact() {
        assert!(matches_pattern("test-vm-01", "test-vm-01"));
        assert!(!matches_pattern("test-vm-01", "test-vm-02"));
    }

    #[test]
    fn test_matches_pattern_prefix() {
        assert!(matches_pattern("test-vm-01", "test*"));
        assert!(matches_pattern("test-vm-02", "test*"));
        assert!(!matches_pattern("prod-vm-01", "test*"));
    }

    #[test]
    fn test_matches_pattern_suffix() {
        assert!(matches_pattern("vm-01", "*01"));
        assert!(!matches_pattern("vm-02", "*01"));
    }

    #[test]
    fn test_matches_pattern_contains() {
        assert!(matches_pattern("test-vm-01", "*vm*"));
        assert!(matches_pattern("prod-vm-02", "*vm*"));
        assert!(!matches_pattern("test-host-01", "*vm*"));
    }

    #[test]
    fn test_matches_pattern_all() {
        assert!(matches_pattern("anything", "*"));
        assert!(matches_pattern("", "*"));
    }

    #[test]
    fn test_qga_verify_result() {
        let success = QgaVerifyResult::success("vm-01", "host-01");
        assert!(success.success);
        assert!(success.error_msg.is_none());

        let failure = QgaVerifyResult::failure("vm-02", "host-02", "连接超时");
        assert!(!failure.success);
        assert_eq!(failure.error_msg.as_deref(), Some("连接超时"));
    }

    #[test]
    fn test_batch_start_result_default() {
        let result = BatchStartResult::default();
        assert_eq!(result.success_count, 0);
        assert!(result.failed_vms.is_empty());
        assert!(result.verification_results.is_none());
    }
}

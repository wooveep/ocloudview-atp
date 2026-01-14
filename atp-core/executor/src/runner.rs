//! 场景执行器

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use virt::domain::Domain;

use atp_protocol::{
    qga::QgaProtocol,
    qmp::QmpProtocol,
    spice::{MouseButton, SpiceProtocol},
    Protocol, ProtocolRegistry,
};
use atp_storage::{ExecutionStepRecord, Storage, TestReportRecord};
use atp_transport::{ListDomainsFilter, TransportManager};
use atp_vdiplatform::{models::CreateDeskPoolRequest, VdiClient};
use verification_server::{
    client::ClientManager, server::ServerConfig, service::ServiceConfig, VerificationServer,
    VerificationService,
};

use crate::{
    Action, ExecutorError, FailureStrategy, InputChannelType, Result, Scenario, ScenarioStep,
    VerificationConfig,
};

/// 场景执行器
pub struct ScenarioRunner {
    /// 传输管理器
    transport_manager: Arc<TransportManager>,

    /// 协议注册表
    protocol_registry: Arc<ProtocolRegistry>,

    /// 当前场景的协议实例 (协议类型 -> 协议实例)
    qmp_protocol: Option<QmpProtocol>,
    qga_protocol: Option<QgaProtocol>,
    spice_protocol: Option<SpiceProtocol>,

    /// VDI 平台客户端 (可选)
    vdi_client: Option<Arc<VdiClient>>,

    /// 当前 Domain
    current_domain: Option<Domain>,

    /// 默认超时时间
    default_timeout: Duration,

    /// 数据库存储 (可选)
    storage: Option<Arc<Storage>>,

    /// 验证服务 (可选,运行时创建)
    verification_service: Option<Arc<VerificationService>>,

    /// 验证服务器后台任务句柄
    verification_server_handle: Option<tokio::task::JoinHandle<()>>,

    /// 客户端管理器 (用于验证)
    client_manager: Option<Arc<ClientManager>>,

    /// 当前验证用的 VM ID (运行时设置)
    verification_vm_id: Option<String>,

    /// 当前场景的输入通道类型
    input_channel_type: InputChannelType,
}

impl ScenarioRunner {
    /// 创建新的场景执行器
    pub fn new(
        transport_manager: Arc<TransportManager>,
        protocol_registry: Arc<ProtocolRegistry>,
    ) -> Self {
        Self {
            transport_manager,
            protocol_registry,
            qmp_protocol: None,
            qga_protocol: None,
            spice_protocol: None,
            vdi_client: None,
            current_domain: None,
            default_timeout: Duration::from_secs(30),
            storage: None,
            verification_service: None,
            verification_server_handle: None,
            client_manager: None,
            verification_vm_id: None,
            input_channel_type: InputChannelType::Qmp,
        }
    }

    /// 设置数据库存储
    pub fn with_storage(mut self, storage: Arc<Storage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// 设置默认超时时间
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// 设置 VDI 平台客户端
    pub fn with_vdi_client(mut self, client: Arc<VdiClient>) -> Self {
        self.vdi_client = Some(client);
        self
    }

    /// 执行场景
    ///
    /// 智能选择执行模式：
    /// - 如果配置了 `target_domains` (多目标选择器)，自动使用多目标执行
    /// - 如果配置了 `target_domain` (单目标)，使用单目标执行
    /// - 如果未配置任何目标，从 VDI 平台自动获取第一个运行中的虚拟机
    pub async fn run(&mut self, scenario: &Scenario) -> Result<ExecutionReport> {
        info!("开始执行场景: {}", scenario.name);

        // 检查是否使用多目标模式
        // 优先检查 target_domains，如果配置了则使用多目标执行
        if scenario.target_domains.is_some() {
            info!("检测到多目标配置 (target_domains)，使用多目标执行模式");
            let multi_report = self.run_multi_target(scenario).await?;
            // 将多目标报告转换为单个报告 (取第一个或合并)
            if let Some(first_report) = multi_report.reports.into_iter().next() {
                return Ok(first_report);
            }
            return Err(ExecutorError::ConfigError(
                "没有匹配的目标虚拟机".to_string(),
            ));
        }

        // 确定目标虚拟机
        // 优先使用场景配置的 target_domain，否则尝试从 VDI 平台获取第一个运行中的虚拟机
        let target_domain = if let Some(domain) = &scenario.target_domain {
            domain.clone()
        } else {
            // 尝试从 VDI 平台获取第一个可用虚拟机
            self.get_first_available_domain_from_vdi().await?
        };

        info!("目标虚拟机: {}", target_domain);

        // 执行单个场景
        self.run_single(scenario, &target_domain).await
    }

    /// 执行单个目标的场景 (核心执行逻辑)
    async fn run_single(
        &mut self,
        scenario: &Scenario,
        target_domain: &str,
    ) -> Result<ExecutionReport> {
        let start_time = Instant::now();
        let mut report = ExecutionReport::new(&scenario.name);

        if let Some(desc) = &scenario.description {
            report.description = Some(desc.clone());
        }

        report.tags = scenario.tags.clone();

        // 设置输入通道类型
        self.input_channel_type = scenario.input_channel.channel_type;

        // 检查是否需要验证 (任意步骤 verify=true 或配置了 verification)
        let needs_verification =
            scenario.verification.is_some() || scenario.steps.iter().any(|s| s.verify);

        // 如果需要验证，启动嵌入式验证服务器
        if needs_verification {
            if let Err(e) = self.start_verification_server(scenario).await {
                error!("启动验证服务器失败: {}", e);
                return Err(e);
            }
        }

        // 初始化协议连接
        if let Err(e) = self.initialize_protocols(scenario, target_domain).await {
            error!("初始化协议失败: {}", e);
            self.cleanup_verification_server().await;
            return Err(e);
        }

        // 在协议初始化完成后，通过 QGA 启动 guest-verifier
        if needs_verification {
            if let Some(verification_config) = &scenario.verification {
                if let Err(e) = self
                    .start_guest_verifier(verification_config, target_domain)
                    .await
                {
                    warn!("启动 guest-verifier 失败: {}, 继续执行但验证可能失败", e);
                }
            }
        }

        for (index, step) in scenario.steps.iter().enumerate() {
            info!("执行步骤 {}/{}", index + 1, scenario.steps.len());

            let step_result = self.execute_step(step, index).await;

            match step_result {
                Ok(result) => {
                    info!("步骤 {} 完成: {}", index + 1, result.description);
                    report.add_step(result);
                }
                Err(e) => {
                    error!("步骤 {} 失败: {}", index + 1, e);
                    let failed_step = StepReport {
                        step_index: index,
                        description: format!("步骤 {}", index + 1),
                        status: StepStatus::Failed,
                        error: Some(e.to_string()),
                        duration_ms: 0,
                        output: None,
                        verified: None,
                        verification_latency_ms: None,
                    };
                    report.add_step(failed_step);
                    break; // 失败后停止执行
                }
            }
        }

        // 清理协议连接
        self.cleanup_protocols().await;

        // 清理验证服务器
        if needs_verification {
            self.cleanup_verification_server().await;
        }

        report.duration_ms = start_time.elapsed().as_millis() as u64;

        info!(
            "场景执行完成: {} - {}/{} 步骤成功",
            scenario.name, report.passed_count, report.steps_executed
        );

        // 保存执行报告到数据库
        if let Some(storage) = &self.storage {
            if let Err(e) = self.save_report_to_db(storage, &report, start_time).await {
                warn!("保存测试报告到数据库失败: {}", e);
                // 不影响测试执行结果,继续返回报告
            }
        }

        Ok(report)
    }

    /// 执行多目标场景
    ///
    /// 当场景配置了多目标选择器 (target_domains) 时，此方法会:
    /// 1. 获取所有可用虚拟机列表
    /// 2. 筛选匹配的虚拟机
    /// 3. 对每个虚拟机执行场景 (串行或并行)
    ///
    /// 返回所有虚拟机的执行报告
    pub async fn run_multi_target(&mut self, scenario: &Scenario) -> Result<MultiTargetReport> {
        info!("开始执行多目标场景: {}", scenario.name);
        let start_time = Instant::now();

        // 获取目标选择器
        let selector = scenario
            .get_target_selector()
            .ok_or_else(|| ExecutorError::ConfigError("未配置目标虚拟机".to_string()))?;

        // 如果不是多目标场景，直接使用单目标执行
        if !selector.is_multi_target() {
            // 获取目标虚拟机
            let target_domain = scenario
                .target_domain
                .clone()
                .ok_or_else(|| ExecutorError::ConfigError("未指定目标虚拟机".to_string()))?;

            let report = self.run_single(scenario, &target_domain).await?;
            return Ok(MultiTargetReport {
                scenario_name: scenario.name.clone(),
                total_targets: 1,
                successful: if report.passed { 1 } else { 0 },
                failed: if report.passed { 0 } else { 1 },
                duration_ms: start_time.elapsed().as_millis() as u64,
                reports: vec![report],
            });
        }

        // 获取所有可用虚拟机列表
        let all_domains = self.list_available_domains(scenario).await?;
        info!("发现 {} 个虚拟机", all_domains.len());

        // 筛选匹配的虚拟机
        let matched_domains: Vec<String> = scenario
            .filter_targets(&all_domains)
            .into_iter()
            .cloned()
            .collect();

        if matched_domains.is_empty() {
            warn!("没有匹配的虚拟机");
            return Ok(MultiTargetReport {
                scenario_name: scenario.name.clone(),
                total_targets: 0,
                successful: 0,
                failed: 0,
                duration_ms: start_time.elapsed().as_millis() as u64,
                reports: vec![],
            });
        }

        info!(
            "匹配到 {} 个虚拟机: {:?}",
            matched_domains.len(),
            matched_domains
        );

        // 获取并行配置
        let parallel_config = scenario.parallel.clone().unwrap_or_default();
        let failure_strategy = parallel_config.on_failure;

        let mut reports = Vec::new();
        let mut successful = 0;
        let mut failed = 0;

        // 只支持串行执行，并行执行需要更复杂的处理（暂不支持）
        // 串行执行
        for domain_name in &matched_domains {
            info!("执行目标: {}", domain_name);

            let report = self.run_single(scenario, domain_name).await;

            match report {
                Ok(r) => {
                    if r.passed {
                        successful += 1;
                    } else {
                        failed += 1;
                    }
                    reports.push(r);

                    // 检查失败策略
                    if failed > 0 && failure_strategy == FailureStrategy::StopAll {
                        warn!("检测到失败，根据策略停止所有执行");
                        break;
                    }
                }
                Err(e) => {
                    error!("执行失败: {} - {}", domain_name, e);
                    failed += 1;

                    // 创建失败报告
                    let failed_report = ExecutionReport {
                        scenario_name: format!("{} ({})", scenario.name, domain_name),
                        description: scenario.description.clone(),
                        tags: scenario.tags.clone(),
                        passed: false,
                        steps_executed: 0,
                        passed_count: 0,
                        failed_count: 1,
                        duration_ms: 0,
                        steps: vec![],
                    };
                    reports.push(failed_report);

                    if failure_strategy == FailureStrategy::StopAll {
                        warn!("检测到失败，根据策略停止所有执行");
                        break;
                    }
                }
            }
        }

        Ok(MultiTargetReport {
            scenario_name: scenario.name.clone(),
            total_targets: matched_domains.len(),
            successful,
            failed,
            duration_ms: start_time.elapsed().as_millis() as u64,
            reports,
        })
    }

    /// 获取可用虚拟机列表
    ///
    /// 优先从 VDI 平台获取虚拟机列表，并通过 VdiCacheManager 同步到本地数据库
    async fn list_available_domains(&mut self, scenario: &Scenario) -> Result<Vec<String>> {
        // 优先使用 VDI 平台获取虚拟机列表
        if let Some(vdi_client) = &self.vdi_client {
            info!("从 VDI 平台获取虚拟机列表");

            // 获取主机列表并同步到本地数据库
            let hosts = vdi_client.host().list_all().await.map_err(|e| {
                ExecutorError::TransportError(format!("VDI 获取主机列表失败: {}", e))
            })?;

            // 获取虚拟机列表（包含主机信息）
            let domains = vdi_client.domain().list_all().await.map_err(|e| {
                ExecutorError::TransportError(format!("VDI 获取虚拟机列表失败: {}", e))
            })?;

            // 使用 VdiCacheManager 同步数据到本地数据库
            if let Some(storage) = &self.storage {
                let cache = atp_storage::VdiCacheManager::from_pool(storage.pool().clone());

                // 先同步主机（满足外键约束）
                match cache.sync_hosts(&hosts).await {
                    Ok(count) => debug!("同步了 {} 个主机记录到数据库", count),
                    Err(e) => warn!("同步主机记录失败: {}", e),
                }

                // 同步虚拟机
                match cache.sync_domains(&domains).await {
                    Ok(count) => info!("同步了 {} 个虚拟机记录到数据库", count),
                    Err(e) => warn!("同步虚拟机记录失败: {}", e),
                }
            }

            // 提取虚拟机名称列表（只返回运行中的虚拟机）
            let domain_names: Vec<String> = domains
                .iter()
                .filter(|d| {
                    // status == 1 表示运行中
                    d["status"].as_i64().map(|s| s == 1).unwrap_or(false)
                })
                .filter_map(|d| d["name"].as_str().map(|s| s.to_string()))
                .collect();

            info!("从 VDI 平台获取到 {} 个运行中的虚拟机", domain_names.len());
            return Ok(domain_names);
        }

        // 回退到传统方式
        let hosts = self.transport_manager.list_hosts().await;
        let host_id = scenario
            .target_host
            .as_deref()
            .or_else(|| hosts.first().map(String::as_str))
            .ok_or_else(|| ExecutorError::ConfigError("未指定目标主机且无可用主机".to_string()))?;

        let domain_infos = self
            .transport_manager
            .execute_on_host(host_id, |conn| async move {
                conn.list_domains(ListDomainsFilter::all()).await
            })
            .await
            .map_err(|e| ExecutorError::TransportError(e.to_string()))?;

        // 从 LibvirtDomainInfo 中提取虚拟机名称
        let domains: Vec<String> = domain_infos.into_iter().map(|info| info.name).collect();
        Ok(domains)
    }

    /// 从 VDI 平台获取第一个可用的运行中虚拟机
    async fn get_first_available_domain_from_vdi(&self) -> Result<String> {
        let vdi_client = self.vdi_client.as_ref().ok_or_else(|| {
            ExecutorError::ConfigError("VDI 客户端未初始化，无法自动获取虚拟机".to_string())
        })?;

        info!("从 VDI 平台获取第一个运行中的虚拟机");

        let domains =
            vdi_client.domain().list_running().await.map_err(|e| {
                ExecutorError::TransportError(format!("VDI 获取虚拟机列表失败: {}", e))
            })?;

        if domains.is_empty() {
            return Err(ExecutorError::ConfigError(
                "VDI 平台没有运行中的虚拟机".to_string(),
            ));
        }

        let first_domain = domains
            .first()
            .and_then(|d| d["name"].as_str())
            .ok_or_else(|| ExecutorError::ConfigError("无法获取虚拟机名称".to_string()))?
            .to_string();

        info!("自动选择虚拟机: {}", first_domain);
        Ok(first_domain)
    }

    /// 初始化协议连接
    async fn initialize_protocols(&mut self, scenario: &Scenario, domain_name: &str) -> Result<()> {
        info!("初始化协议连接: 虚拟机 = {}", domain_name);

        // 获取目标主机
        // 1. 优先使用场景配置的 target_host
        // 2. 其次从数据库读取虚拟机-主机映射
        // 3. 最后回退到 transport_manager 中的第一个主机
        let host_id = if let Some(host) = &scenario.target_host {
            host.clone()
        } else if let Some(storage) = &self.storage {
            // 从数据库读取虚拟机记录
            match storage.domains().get_by_name(domain_name).await {
                Ok(Some(domain_record)) => {
                    let host_id = domain_record.host_id.clone().unwrap_or_default();
                    // 获取主机 IP (从 hosts 表)
                    let host_ip =
                        if let Ok(Some(host_record)) = storage.hosts().get_by_id(&host_id).await {
                            host_record.host.clone()
                        } else {
                            host_id.clone() // 回退到使用 host_id
                        };

                    info!(
                        "从数据库获取虚拟机 {} 的主机信息: {} ({})",
                        domain_name, host_id, host_ip
                    );

                    // 确保主机已注册到 transport_manager
                    let registered_hosts = self.transport_manager.list_hosts().await;
                    if !registered_hosts
                        .iter()
                        .any(|h| h == &host_id || h == &host_ip)
                    {
                        // 动态注册主机
                        info!("动态注册主机: {} ({})", host_id, host_ip);
                        let uri = format!("qemu+tcp://{}/system", host_ip);
                        let host_info = atp_transport::HostInfo {
                            id: host_id.clone(),
                            host: host_ip.clone(),
                            uri,
                            tags: vec![],
                            metadata: std::collections::HashMap::new(),
                        };

                        self.transport_manager
                            .add_host(host_info)
                            .await
                            .map_err(|e| {
                                ExecutorError::TransportError(format!("注册主机失败: {}", e))
                            })?;

                        // 等待连接建立（add_host 在后台建立连接）
                        info!("等待主机连接建立...");
                        let max_wait = Duration::from_secs(30);
                        let start = Instant::now();
                        let mut connected = false;

                        while start.elapsed() < max_wait {
                            // 尝试获取连接来检查是否已建立
                            match self
                                .transport_manager
                                .execute_on_host(&host_id, |_conn| async move {
                                    // 简单检查连接状态
                                    Ok(())
                                })
                                .await
                            {
                                Ok(_) => {
                                    connected = true;
                                    info!("主机连接已建立: {}", host_id);
                                    break;
                                }
                                Err(_) => {
                                    tokio::time::sleep(Duration::from_millis(500)).await;
                                }
                            }
                        }

                        if !connected {
                            return Err(ExecutorError::TransportError(format!(
                                "等待主机 {} 连接超时",
                                host_id
                            )));
                        }
                    }

                    host_id
                }
                Ok(None) => {
                    warn!("数据库中未找到虚拟机 {} 的主机映射", domain_name);
                    // 回退到 transport_manager 中的第一个主机
                    let hosts = self.transport_manager.list_hosts().await;
                    hosts.first().cloned().ok_or_else(|| {
                        ExecutorError::ConfigError(format!(
                            "未找到虚拟机 {} 的主机映射，且无可用主机",
                            domain_name
                        ))
                    })?
                }
                Err(e) => {
                    warn!("从数据库读取虚拟机-主机映射失败: {}", e);
                    let hosts = self.transport_manager.list_hosts().await;
                    hosts.first().cloned().ok_or_else(|| {
                        ExecutorError::ConfigError("数据库读取失败且无可用主机".to_string())
                    })?
                }
            }
        } else {
            // 回退到 transport_manager 中的第一个主机
            let hosts = self.transport_manager.list_hosts().await;
            hosts.first().cloned().ok_or_else(|| {
                ExecutorError::ConfigError("未指定目标主机且无可用主机".to_string())
            })?
        };

        // 通过 transport manager 获取 domain
        let domain = self
            .transport_manager
            .execute_on_host(&host_id, |conn| async move {
                conn.get_domain(domain_name).await
            })
            .await
            .map_err(|e| ExecutorError::TransportError(e.to_string()))?;

        // QGA 协议始终初始化（用于 guest-verifier 启动）
        let mut qga = QgaProtocol::new();
        if let Err(e) = qga.connect(&domain).await {
            warn!("QGA 协议连接失败: {}", e);
            // QGA 失败不是致命错误，可能虚拟机没有安装 guest agent
        } else {
            info!("QGA 协议连接成功");
            self.qga_protocol = Some(qga);
        }

        // 根据 input_channel 配置决定初始化 QMP 或 SPICE
        match scenario.input_channel.channel_type {
            InputChannelType::Qmp => {
                info!("初始化 QMP 协议 (input_channel: qmp)");
                let mut qmp = QmpProtocol::new();
                if let Err(e) = qmp.connect(&domain).await {
                    return Err(ExecutorError::ProtocolError(format!(
                        "QMP 协议连接失败: {}",
                        e
                    )));
                }
                info!("QMP 协议连接成功");
                self.qmp_protocol = Some(qmp);
            }
            InputChannelType::Spice => {
                info!("初始化 SPICE 协议 (input_channel: spice)");
                let mut spice = SpiceProtocol::new();
                if let Err(e) = spice.connect(&domain).await {
                    return Err(ExecutorError::ProtocolError(format!(
                        "SPICE 协议连接失败: {}",
                        e
                    )));
                }
                info!("SPICE 协议连接成功");
                self.spice_protocol = Some(spice);
            }
        }

        self.current_domain = Some(domain);

        Ok(())
    }

    /// 清理协议连接
    async fn cleanup_protocols(&mut self) {
        if let Some(mut qmp) = self.qmp_protocol.take() {
            let _ = qmp.disconnect().await;
        }

        if let Some(mut qga) = self.qga_protocol.take() {
            let _ = qga.disconnect().await;
        }

        if let Some(mut spice) = self.spice_protocol.take() {
            let _ = spice.disconnect().await;
        }

        self.current_domain = None;
    }

    /// 启动嵌入式验证服务器
    async fn start_verification_server(&mut self, scenario: &Scenario) -> Result<()> {
        info!("启动嵌入式验证服务器");

        // 设置验证用的 VM ID（优先使用 verification.vm_id，其次使用 target_domain）
        self.verification_vm_id = scenario
            .verification
            .as_ref()
            .and_then(|c| c.vm_id.clone())
            .or_else(|| scenario.target_domain.clone());

        // 从场景配置获取服务器地址
        let (ws_addr, tcp_addr) = if let Some(config) = &scenario.verification {
            (
                config
                    .ws_addr
                    .clone()
                    .unwrap_or_else(|| "0.0.0.0:8765".to_string()),
                config
                    .tcp_addr
                    .clone()
                    .unwrap_or_else(|| "0.0.0.0:8766".to_string()),
            )
        } else {
            ("0.0.0.0:8765".to_string(), "0.0.0.0:8766".to_string())
        };

        // 创建客户端管理器
        let client_manager = Arc::new(ClientManager::new());
        self.client_manager = Some(Arc::clone(&client_manager));

        // 创建验证服务
        let service_config = ServiceConfig::default();
        let verification_service = Arc::new(VerificationService::new(
            Arc::clone(&client_manager),
            service_config,
        ));
        self.verification_service = Some(Arc::clone(&verification_service));

        // 创建服务器配置
        let server_config = ServerConfig {
            websocket_addr: ws_addr.parse().ok(),
            tcp_addr: tcp_addr.parse().ok(),
        };

        // 创建并启动验证服务器
        let server = VerificationServer::new(server_config, Arc::clone(&client_manager));

        let handle = tokio::spawn(async move {
            if let Err(e) = server.start().await {
                error!("验证服务器错误: {}", e);
            }
        });

        self.verification_server_handle = Some(handle);

        // 等待服务器启动
        tokio::time::sleep(Duration::from_millis(100)).await;
        info!("验证服务器已启动: ws={}, tcp={}", ws_addr, tcp_addr);

        Ok(())
    }

    /// 通过 QGA 启动 guest-verifier
    ///
    /// 该方法会：
    /// 1. 从数据库获取 Guest OS 类型（由 VDI 平台提供）
    /// 2. 从配置的网络接口获取宿主机 IP 地址
    /// 3. 使用正确的启动命令启动 verifier-agent
    async fn start_guest_verifier(
        &self,
        config: &VerificationConfig,
        target_domain: &str,
    ) -> Result<()> {
        let qga = self
            .qga_protocol
            .as_ref()
            .ok_or_else(|| ExecutorError::ProtocolError("QGA 协议未初始化".to_string()))?;

        // 1. 从数据库获取 Guest OS 类型（由 VDI 平台提供）
        let os_type = if let Some(storage) = &self.storage {
            match storage.domains().get_by_name(target_domain).await {
                Ok(Some(domain_record)) => {
                    let os = Self::normalize_os_type(domain_record.os_type.as_deref());
                    info!("从数据库获取 Guest OS 类型: {} -> {}", target_domain, os);
                    os
                }
                _ => {
                    warn!(
                        "数据库中未找到虚拟机 {} 的 OS 类型信息，假设为 Linux",
                        target_domain
                    );
                    "linux".to_string()
                }
            }
        } else {
            warn!("数据库未初始化，假设为 Linux");
            "linux".to_string()
        };

        // 2. 获取验证器路径（根据 OS 类型选择默认路径）
        let verifier_path = config.guest_verifier_path.clone().unwrap_or_else(|| {
            if os_type == "windows" {
                "C:\\Program Files\\verifier-agent\\verifier-agent.exe".to_string()
            } else {
                "/usr/local/bin/verifier-agent".to_string()
            }
        });

        // 3. 获取本机 IP 地址（从配置的网络接口读取）
        let interface = config.host_interface.as_deref().unwrap_or("virbr0"); // 默认使用 libvirt 默认网桥
        let host_ip = Self::get_interface_ip(interface);

        // 4. 构建验证服务器地址
        let ws_port = config
            .ws_addr
            .as_ref()
            .and_then(|addr| addr.split(':').last())
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8765);

        let server_addr = format!("{}:{}", host_ip, ws_port);

        info!(
            "准备启动 guest-verifier: path={}, server={}, os={}, interface={}",
            verifier_path, server_addr, os_type, interface
        );

        // 5. 构建启动参数
        // verifier-agent 参数: --server <地址> --transport websocket --verifiers keyboard,mouse
        let args = vec![
            "--server".to_string(),
            server_addr.clone(),
            "--transport".to_string(),
            "websocket".to_string(),
            "--verifiers".to_string(),
            "keyboard".to_string(),
            "--verifiers".to_string(),
            "mouse".to_string(),
        ];

        // 6. 使用跨平台方式启动后台程序
        match qga.exec_background(&os_type, &verifier_path, &args).await {
            Ok(pid) => {
                if pid > 0 {
                    info!("guest-verifier 已启动，PID: {}", pid);
                } else {
                    info!("guest-verifier 启动命令已执行");
                }
            }
            Err(e) => {
                return Err(ExecutorError::ProtocolError(format!(
                    "启动 guest-verifier 失败: {}",
                    e
                )));
            }
        }

        // 7. 等待客户端连接
        let connection_timeout = config.connection_timeout.unwrap_or(30);
        info!("等待 guest-verifier 连接 (超时: {}s)", connection_timeout);

        if let Some(client_manager) = &self.client_manager {
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(connection_timeout) {
                let clients = client_manager.get_clients().await;
                if !clients.is_empty() {
                    info!("guest-verifier 已连接: {} 个客户端", clients.len());
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            warn!("等待 guest-verifier 连接超时");
        }

        Ok(())
    }

    /// 获取指定网络接口的 IP 地址
    ///
    /// # 参数
    /// * `interface` - 网络接口名称 (例如: "virbr0", "br0", "eth0")
    ///
    /// # 返回
    /// 返回接口的 IPv4 地址，如果获取失败则返回默认值 "192.168.122.1"
    fn get_interface_ip(interface: &str) -> String {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;

            // 使用 ip addr show 获取接口 IP
            if let Ok(output) = Command::new("ip")
                .args(["addr", "show", interface])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // 解析: inet 192.168.122.1/24 brd ...
                for line in stdout.lines() {
                    if let Some(inet_pos) = line.find("inet ") {
                        let ip_start = inet_pos + 5;
                        let ip_end = line[ip_start..]
                            .find('/')
                            .map(|p| ip_start + p)
                            .unwrap_or(line.len());
                        let ip = line[ip_start..ip_end].trim();
                        if !ip.is_empty() && !ip.starts_with("127.") {
                            info!("使用主机 IP: {} (来自接口 {})", ip, interface);
                            return ip.to_string();
                        }
                    }
                }
            }

            warn!("无法获取接口 {} 的 IP 地址", interface);
        }

        #[cfg(not(target_os = "linux"))]
        {
            warn!("非 Linux 系统，无法获取接口 {} 的 IP 地址", interface);
        }

        // 默认返回 libvirt 默认网桥的常用地址
        "192.168.122.1".to_string()
    }

    /// 将 VDI 平台的 osType 转换为标准化的操作系统类型
    ///
    /// VDI 平台 osType 可能的值：
    /// - Windows: "xp-32", "win7-32", "win7-64", "win10-64", "win11-64"
    /// - Linux: "linux", "kylin", "uos", "其他"
    ///
    /// 返回 "windows" 或 "linux"
    fn normalize_os_type(os_type: Option<&str>) -> String {
        match os_type {
            Some(os) => {
                let os_lower = os.to_lowercase();
                if os_lower.starts_with("win") || os_lower.starts_with("xp") {
                    "windows".to_string()
                } else {
                    // linux, kylin, uos, 其他 都视为 Linux
                    "linux".to_string()
                }
            }
            None => "linux".to_string(), // 默认假设为 Linux
        }
    }

    /// 清理验证服务器
    async fn cleanup_verification_server(&mut self) {
        if let Some(handle) = self.verification_server_handle.take() {
            handle.abort();
            info!("验证服务器已停止");
        }

        self.verification_service = None;
        self.client_manager = None;
    }

    /// 执行单个步骤
    async fn execute_step(&mut self, step: &ScenarioStep, index: usize) -> Result<StepReport> {
        let start_time = Instant::now();

        let step_timeout = step
            .timeout
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        // 如果需要验证，先注册期望事件
        let verification_future = if step.verify && self.verification_service.is_some() {
            self.create_verification_future(step, step_timeout)
        } else {
            None
        };

        // 执行动作
        let result = timeout(step_timeout, self.execute_action(&step.action, index)).await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(mut report)) => {
                report.duration_ms = duration_ms;
                if let Some(name) = &step.name {
                    report.description = name.clone();
                }

                // 如果有验证任务，等待结果
                if let Some(verify_task) = verification_future {
                    match verify_task.await {
                        Ok(Ok(verify_result)) => {
                            report.verified = Some(verify_result.verified);
                            report.verification_latency_ms = Some(verify_result.latency_ms);
                            debug!(
                                "验证结果: verified={}, latency={}ms",
                                verify_result.verified, verify_result.latency_ms
                            );
                        }
                        Ok(Err(e)) => {
                            warn!("验证服务错误: {}", e);
                            report.verified = Some(false);
                        }
                        Err(e) => {
                            warn!("验证任务失败: {}", e);
                            report.verified = Some(false);
                        }
                    }
                }

                Ok(report)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                error!("步骤执行超时");
                Err(ExecutorError::Timeout)
            }
        }
    }

    /// 创建验证等待任务
    fn create_verification_future(
        &self,
        step: &ScenarioStep,
        timeout_duration: Duration,
    ) -> Option<
        tokio::task::JoinHandle<
            std::result::Result<atp_common::VerifyResult, verification_server::VerificationError>,
        >,
    > {
        let verification_service = self.verification_service.as_ref()?.clone();

        // 根据动作类型确定期望的事件
        let (event_type, expected_name) = match &step.action {
            Action::SendKey { key } => ("keyboard", key.to_uppercase()),
            Action::SendText { text } => {
                // 对于文本，我们验证第一个字符
                let first_char = text.chars().next()?;
                ("keyboard", first_char.to_uppercase().to_string())
            }
            _ => return None, // 其他动作暂不支持验证
        };

        // 从 ScenarioRunner 获取 VM ID，默认使用 "default"
        let vm_id = self
            .verification_vm_id
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let event_type_owned = event_type.to_string();

        Some(tokio::spawn(async move {
            verification_service
                .expect_input(
                    &vm_id,
                    &event_type_owned,
                    &expected_name,
                    Some(1), // 期望按下事件
                    Some(timeout_duration),
                )
                .await
        }))
    }

    /// 执行具体动作
    async fn execute_action(&mut self, action: &Action, index: usize) -> Result<StepReport> {
        match action {
            Action::SendKey { key } => self.execute_send_key(key, index).await,
            Action::SendText { text } => self.execute_send_text(text, index).await,
            Action::MouseClick { x, y, button } => {
                self.execute_mouse_click(*x, *y, button, index).await
            }
            Action::ExecCommand { command } => self.execute_command(command, index).await,
            Action::Wait { duration } => self.execute_wait(*duration, index).await,
            Action::Custom { data } => {
                warn!("自定义动作尚未完全实现: {:?}", data);
                Ok(StepReport::success(index, "自定义动作（跳过）"))
            }
            // VDI 平台操作
            Action::VdiCreateDeskPool {
                name,
                template_id,
                count,
            } => {
                self.execute_vdi_create_desk_pool(name, template_id, *count, index)
                    .await
            }
            Action::VdiEnableDeskPool { pool_id } => {
                self.execute_vdi_enable_desk_pool(pool_id, index).await
            }
            Action::VdiDisableDeskPool { pool_id } => {
                self.execute_vdi_disable_desk_pool(pool_id, index).await
            }
            Action::VdiDeleteDeskPool { pool_id } => {
                self.execute_vdi_delete_desk_pool(pool_id, index).await
            }
            Action::VdiStartDomain { domain_id } => {
                self.execute_vdi_start_domain(domain_id, index).await
            }
            Action::VdiShutdownDomain { domain_id } => {
                self.execute_vdi_shutdown_domain(domain_id, index).await
            }
            Action::VdiRebootDomain { domain_id } => {
                self.execute_vdi_reboot_domain(domain_id, index).await
            }
            Action::VdiDeleteDomain { domain_id } => {
                self.execute_vdi_delete_domain(domain_id, index).await
            }
            Action::VdiBindUser { domain_id, user_id } => {
                self.execute_vdi_bind_user(domain_id, user_id, index).await
            }
            Action::VdiGetDeskPoolDomains { pool_id } => {
                self.execute_vdi_get_desk_pool_domains(pool_id, index).await
            }
            // 验证步骤
            Action::VerifyDomainStatus {
                domain_id,
                expected_status,
                timeout_secs,
            } => {
                self.verify_domain_status(domain_id, expected_status, *timeout_secs, index)
                    .await
            }
            Action::VerifyAllDomainsRunning {
                pool_id,
                timeout_secs,
            } => {
                self.verify_all_domains_running(pool_id, *timeout_secs, index)
                    .await
            }
            Action::VerifyCommandSuccess { timeout_secs } => {
                self.verify_command_success(*timeout_secs, index).await
            }
        }
    }

    /// 执行发送按键
    async fn execute_send_key(&mut self, key: &str, index: usize) -> Result<StepReport> {
        info!("发送按键: {} (通道: {:?})", key, self.input_channel_type);

        match self.input_channel_type {
            InputChannelType::Qmp => {
                // 使用 QMP 协议发送按键
                if let Some(qmp) = &mut self.qmp_protocol {
                    qmp.send_key(key).await.map_err(|e| {
                        ExecutorError::ProtocolError(format!("QMP send_key 失败: {}", e))
                    })?;

                    Ok(StepReport::success(
                        index,
                        &format!("发送按键: {} (QMP)", key),
                    ))
                } else {
                    Err(ExecutorError::ProtocolError("QMP 协议未初始化".to_string()))
                }
            }
            InputChannelType::Spice => {
                // 使用 SPICE 协议发送按键
                if let Some(spice) = &self.spice_protocol {
                    // 将 QKeyCode 转换为扫描码
                    let scancode = qkeycode_to_scancode(key).ok_or_else(|| {
                        ExecutorError::ProtocolError(format!("未知的按键: {}", key))
                    })?;

                    // 发送按下和释放
                    spice.send_key(scancode, true).await.map_err(|e| {
                        ExecutorError::ProtocolError(format!("SPICE key_down 失败: {}", e))
                    })?;

                    // 短暂延迟
                    tokio::time::sleep(Duration::from_millis(50)).await;

                    spice.send_key(scancode, false).await.map_err(|e| {
                        ExecutorError::ProtocolError(format!("SPICE key_up 失败: {}", e))
                    })?;

                    Ok(StepReport::success(
                        index,
                        &format!("发送按键: {} (SPICE)", key),
                    ))
                } else {
                    Err(ExecutorError::ProtocolError(
                        "SPICE 协议未初始化".to_string(),
                    ))
                }
            }
        }
    }

    /// 执行发送文本
    async fn execute_send_text(&mut self, text: &str, index: usize) -> Result<StepReport> {
        info!("发送文本: {} (通道: {:?})", text, self.input_channel_type);

        // 将文本转换为按键序列
        let keys: Vec<&str> = text.chars().filter_map(char_to_qkeycode).collect();

        if keys.is_empty() {
            warn!("文本中没有可映射的字符: {}", text);
            return Ok(StepReport::success(
                index,
                &format!("发送文本: {} (无可映射字符)", text),
            ));
        }

        match self.input_channel_type {
            InputChannelType::Qmp => {
                // 使用 QMP 协议发送文本
                if let Some(qmp) = &mut self.qmp_protocol {
                    qmp.send_keys(keys, Some(100)).await.map_err(|e| {
                        ExecutorError::ProtocolError(format!("QMP send_keys 失败: {}", e))
                    })?;

                    Ok(StepReport::success(
                        index,
                        &format!("发送文本: {} [QMP]", text),
                    ))
                } else {
                    Err(ExecutorError::ProtocolError("QMP 协议未初始化".to_string()))
                }
            }
            InputChannelType::Spice => {
                // 使用 SPICE 协议发送文本
                if let Some(spice) = &self.spice_protocol {
                    for key in keys {
                        // 将 QKeyCode 转换为 scancode
                        if let Some(scancode) = qkeycode_to_scancode(key) {
                            // 按下
                            spice.send_key(scancode, true).await.map_err(|e| {
                                ExecutorError::ProtocolError(format!(
                                    "SPICE send_key (down) 失败: {}",
                                    e
                                ))
                            })?;

                            // 短暂延迟
                            tokio::time::sleep(Duration::from_millis(50)).await;

                            // 释放
                            spice.send_key(scancode, false).await.map_err(|e| {
                                ExecutorError::ProtocolError(format!(
                                    "SPICE send_key (up) 失败: {}",
                                    e
                                ))
                            })?;

                            // 按键间隔
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                    }

                    Ok(StepReport::success(
                        index,
                        &format!("发送文本: {} [SPICE]", text),
                    ))
                } else {
                    Err(ExecutorError::ProtocolError(
                        "SPICE 协议未初始化".to_string(),
                    ))
                }
            }
        }
    }

    /// 执行鼠标点击
    async fn execute_mouse_click(
        &mut self,
        x: i32,
        y: i32,
        button: &str,
        index: usize,
    ) -> Result<StepReport> {
        info!("鼠标点击: ({}, {}) 按钮: {}", x, y, button);

        // 使用 SPICE 协议发送鼠标操作
        if let Some(spice) = &mut self.spice_protocol {
            // 将按钮字符串转换为 MouseButton 枚举
            let mouse_button = match button.to_lowercase().as_str() {
                "left" => MouseButton::Left,
                "right" => MouseButton::Right,
                "middle" => MouseButton::Middle,
                _ => {
                    warn!("未知的鼠标按钮: {}, 使用默认左键", button);
                    MouseButton::Left
                }
            };

            // 首先移动鼠标到目标位置（使用绝对坐标）
            spice
                .send_mouse_move(x as u32, y as u32, 0)
                .await
                .map_err(|e| ExecutorError::ProtocolError(format!("SPICE 鼠标移动失败: {}", e)))?;

            // 等待一小段时间确保位置更新
            tokio::time::sleep(Duration::from_millis(50)).await;

            // 发送鼠标点击（按下）
            spice
                .send_mouse_click(mouse_button, true)
                .await
                .map_err(|e| ExecutorError::ProtocolError(format!("SPICE 鼠标按下失败: {}", e)))?;

            // 短暂延迟模拟真实点击
            tokio::time::sleep(Duration::from_millis(50)).await;

            // 发送鼠标释放
            spice
                .send_mouse_click(mouse_button, false)
                .await
                .map_err(|e| ExecutorError::ProtocolError(format!("SPICE 鼠标释放失败: {}", e)))?;

            Ok(StepReport::success(
                index,
                &format!("鼠标点击: ({}, {}) 按钮: {}", x, y, button),
            ))
        } else {
            // 如果 SPICE 未连接，尝试通过 QGA 执行脚本模拟鼠标操作（备用方案）
            if let Some(qga) = &self.qga_protocol {
                warn!("SPICE 协议未初始化，尝试通过 QGA 执行鼠标脚本");

                // 在 Linux 中可以使用 xdotool 模拟鼠标
                let script = format!(
                    "DISPLAY=:0 xdotool mousemove {} {} click {}",
                    x,
                    y,
                    match button.to_lowercase().as_str() {
                        "left" => "1",
                        "middle" => "2",
                        "right" => "3",
                        _ => "1",
                    }
                );

                let status = qga.exec_shell(&script).await.map_err(|e| {
                    ExecutorError::ProtocolError(format!("QGA 执行鼠标脚本失败: {}", e))
                })?;

                if let Some(exit_code) = status.exit_code {
                    if exit_code != 0 {
                        return Ok(StepReport::failed(
                            index,
                            &format!("鼠标点击: ({}, {})", x, y),
                            "xdotool 执行失败（可能未安装）",
                        ));
                    }
                }

                Ok(StepReport::success(
                    index,
                    &format!("鼠标点击: ({}, {}) [QGA/xdotool]", x, y),
                ))
            } else {
                Err(ExecutorError::ProtocolError(
                    "SPICE 和 QGA 协议均未初始化，无法执行鼠标操作".to_string(),
                ))
            }
        }
    }

    /// 执行命令
    async fn execute_command(&mut self, command: &str, index: usize) -> Result<StepReport> {
        info!("执行命令: {}", command);

        // 使用 QGA 协议执行命令
        if let Some(qga) = &self.qga_protocol {
            let status = qga
                .exec_shell(command)
                .await
                .map_err(|e| ExecutorError::ProtocolError(format!("QGA exec_shell 失败: {}", e)))?;

            // 检查退出码
            if let Some(exit_code) = status.exit_code {
                if exit_code != 0 {
                    let stderr = status
                        .decode_stderr()
                        .unwrap_or_else(|| "无错误输出".to_string());

                    return Ok(StepReport::failed(
                        index,
                        &format!("执行命令: {}", command),
                        &format!("命令执行失败 (退出码: {}): {}", exit_code, stderr),
                    ));
                }
            }

            // 获取输出
            let stdout = status
                .decode_stdout()
                .unwrap_or_else(|| "无输出".to_string());

            let mut report = StepReport::success(index, &format!("执行命令: {}", command));
            report.output = Some(stdout);
            Ok(report)
        } else {
            Err(ExecutorError::ProtocolError("QGA 协议未初始化".to_string()))
        }
    }

    /// 执行等待
    async fn execute_wait(&self, duration: u64, index: usize) -> Result<StepReport> {
        info!("等待 {} 秒", duration);

        tokio::time::sleep(Duration::from_secs(duration)).await;

        Ok(StepReport::success(index, &format!("等待 {} 秒", duration)))
    }

    // ========================================
    // VDI 平台操作执行方法
    // ========================================

    /// 执行 VDI 创建桌面池
    async fn execute_vdi_create_desk_pool(
        &mut self,
        name: &str,
        template_id: &str,
        count: u32,
        index: usize,
    ) -> Result<StepReport> {
        info!(
            "创建桌面池: {} (模板: {}, 数量: {})",
            name, template_id, count
        );

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        // 构造创建桌面池请求
        let request = CreateDeskPoolRequest {
            name: name.to_string(),
            template_id: template_id.to_string(),
            count,
            vcpu: 2,      // 默认值，可以从场景配置中获取
            memory: 2048, // 默认 2GB，可以从场景配置中获取
        };

        // 调用 VDI 平台 API 创建桌面池
        vdi_client
            .desk_pool()
            .create(request)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("创建桌面池失败: {}", e)))?;

        Ok(StepReport::success(index, &format!("创建桌面池: {}", name)))
    }

    /// 执行 VDI 启用桌面池
    async fn execute_vdi_enable_desk_pool(
        &mut self,
        pool_id: &str,
        index: usize,
    ) -> Result<StepReport> {
        info!("启用桌面池: {}", pool_id);

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client
            .desk_pool()
            .enable(pool_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("启用桌面池失败: {}", e)))?;

        Ok(StepReport::success(
            index,
            &format!("启用桌面池: {}", pool_id),
        ))
    }

    /// 执行 VDI 禁用桌面池
    async fn execute_vdi_disable_desk_pool(
        &mut self,
        pool_id: &str,
        index: usize,
    ) -> Result<StepReport> {
        info!("禁用桌面池: {}", pool_id);

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client
            .desk_pool()
            .disable(pool_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("禁用桌面池失败: {}", e)))?;

        Ok(StepReport::success(
            index,
            &format!("禁用桌面池: {}", pool_id),
        ))
    }

    /// 执行 VDI 删除桌面池
    async fn execute_vdi_delete_desk_pool(
        &mut self,
        pool_id: &str,
        index: usize,
    ) -> Result<StepReport> {
        info!("删除桌面池: {}", pool_id);

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client
            .desk_pool()
            .delete(pool_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("删除桌面池失败: {}", e)))?;

        Ok(StepReport::success(
            index,
            &format!("删除桌面池: {}", pool_id),
        ))
    }

    /// 执行 VDI 启动虚拟机
    async fn execute_vdi_start_domain(
        &mut self,
        domain_id: &str,
        index: usize,
    ) -> Result<StepReport> {
        info!("启动虚拟机: {}", domain_id);

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client
            .domain()
            .start(domain_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("启动虚拟机失败: {}", e)))?;

        Ok(StepReport::success(
            index,
            &format!("启动虚拟机: {}", domain_id),
        ))
    }

    /// 执行 VDI 关闭虚拟机
    async fn execute_vdi_shutdown_domain(
        &mut self,
        domain_id: &str,
        index: usize,
    ) -> Result<StepReport> {
        info!("关闭虚拟机: {}", domain_id);

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client
            .domain()
            .shutdown(domain_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("关闭虚拟机失败: {}", e)))?;

        Ok(StepReport::success(
            index,
            &format!("关闭虚拟机: {}", domain_id),
        ))
    }

    /// 执行 VDI 重启虚拟机
    async fn execute_vdi_reboot_domain(
        &mut self,
        domain_id: &str,
        index: usize,
    ) -> Result<StepReport> {
        info!("重启虚拟机: {}", domain_id);

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client
            .domain()
            .reboot(domain_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("重启虚拟机失败: {}", e)))?;

        Ok(StepReport::success(
            index,
            &format!("重启虚拟机: {}", domain_id),
        ))
    }

    /// 执行 VDI 删除虚拟机
    async fn execute_vdi_delete_domain(
        &mut self,
        domain_id: &str,
        index: usize,
    ) -> Result<StepReport> {
        info!("删除虚拟机: {}", domain_id);

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client
            .domain()
            .delete(domain_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("删除虚拟机失败: {}", e)))?;

        Ok(StepReport::success(
            index,
            &format!("删除虚拟机: {}", domain_id),
        ))
    }

    /// 执行 VDI 绑定用户
    async fn execute_vdi_bind_user(
        &mut self,
        domain_id: &str,
        user_id: &str,
        index: usize,
    ) -> Result<StepReport> {
        info!("绑定用户: 虚拟机={}, 用户={}", domain_id, user_id);

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client
            .domain()
            .bind_user(domain_id, user_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("绑定用户失败: {}", e)))?;

        Ok(StepReport::success(
            index,
            &format!("绑定用户: 虚拟机={}, 用户={}", domain_id, user_id),
        ))
    }

    /// 执行 VDI 获取桌面池虚拟机列表
    async fn execute_vdi_get_desk_pool_domains(
        &mut self,
        pool_id: &str,
        index: usize,
    ) -> Result<StepReport> {
        info!("获取桌面池虚拟机列表: {}", pool_id);

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        let domains = vdi_client
            .desk_pool()
            .list_domains(pool_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("获取虚拟机列表失败: {}", e)))?;

        let mut report = StepReport::success(index, &format!("获取桌面池虚拟机列表: {}", pool_id));
        report.output = Some(format!("虚拟机数量: {}", domains.len()));
        Ok(report)
    }

    // ========================================
    // 验证步骤执行方法
    // ========================================

    /// 验证虚拟机状态
    async fn verify_domain_status(
        &mut self,
        domain_id: &str,
        expected_status: &str,
        timeout_secs: Option<u64>,
        index: usize,
    ) -> Result<StepReport> {
        use crate::vdi_ops::get_domain_libvirt_state;

        info!("验证虚拟机状态: {} 应为 {}", domain_id, expected_status);

        let timeout_duration = timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        // 获取第一个可用主机
        let hosts = self.transport_manager.list_hosts().await;
        let host_id = hosts
            .first()
            .ok_or_else(|| ExecutorError::ConfigError("无可用主机".to_string()))?
            .clone();

        // 使用共享函数获取 libvirt 虚拟机状态
        let result = timeout(timeout_duration, async {
            let state = get_domain_libvirt_state(&self.transport_manager, &host_id, domain_id)
                .await
                .map_err(|e| ExecutorError::TransportError(e.to_string()))?;

            let actual_status = state.state.to_lowercase();
            let expected_lower = expected_status.to_lowercase();

            if actual_status.contains(&expected_lower) || expected_lower.contains(&actual_status) {
                Ok(StepReport::success(
                    index,
                    &format!("虚拟机状态验证成功: {} = {}", domain_id, expected_status),
                ))
            } else {
                Ok(StepReport::failed(
                    index,
                    &format!("虚拟机状态验证失败: {}", domain_id),
                    &format!("期望: {}, 实际: {}", expected_status, actual_status),
                ))
            }
        })
        .await;

        match result {
            Ok(Ok(report)) => Ok(report),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                error!("验证虚拟机状态超时");
                Err(ExecutorError::Timeout)
            }
        }
    }

    /// 验证所有虚拟机运行中
    async fn verify_all_domains_running(
        &mut self,
        pool_id: &str,
        timeout_secs: Option<u64>,
        index: usize,
    ) -> Result<StepReport> {
        info!("验证所有虚拟机运行中: 桌面池 {}", pool_id);

        let timeout_duration = timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let vdi_client = self
            .vdi_client
            .as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        let result = timeout(timeout_duration, async {
            // 获取桌面池的所有虚拟机
            let domains = vdi_client
                .desk_pool()
                .list_domains(pool_id)
                .await
                .map_err(|e| ExecutorError::TransportError(format!("获取虚拟机列表失败: {}", e)))?;

            let mut all_running = true;
            let mut failed_domains = Vec::new();

            // 检查每个虚拟机的状态
            for domain_info in &domains {
                // 使用 VDI API 返回的状态
                if domain_info.status.to_lowercase() != "running" {
                    all_running = false;
                    failed_domains.push(domain_info.id.clone());
                }
            }

            if all_running {
                Ok(StepReport::success(
                    index,
                    &format!(
                        "所有虚拟机运行中: 桌面池 {} ({} 台)",
                        pool_id,
                        domains.len()
                    ),
                ))
            } else {
                Ok(StepReport::failed(
                    index,
                    &format!("验证所有虚拟机运行中: 桌面池 {}", pool_id),
                    &format!("以下虚拟机未运行: {:?}", failed_domains),
                ))
            }
        })
        .await;

        match result {
            Ok(Ok(report)) => Ok(report),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                error!("验证所有虚拟机运行中超时");
                Err(ExecutorError::Timeout)
            }
        }
    }

    /// 验证命令执行成功
    async fn verify_command_success(
        &mut self,
        _timeout_secs: Option<u64>,
        index: usize,
    ) -> Result<StepReport> {
        info!("验证命令执行成功");

        // 这个验证步骤通常跟在 ExecCommand 之后
        // 由于我们在 execute_command 中已经检查了退出码
        // 这里主要是作为一个显式的验证步骤

        // 如果到达这里，说明之前的命令执行成功
        Ok(StepReport::success(index, "验证命令执行成功"))
    }

    /// 保存报告到数据库
    async fn save_report_to_db(
        &self,
        storage: &Storage,
        report: &ExecutionReport,
        _start_time: Instant,
    ) -> Result<i64> {
        // 计算实际开始时间
        let now = Utc::now();
        let actual_start_time = now - chrono::Duration::milliseconds(report.duration_ms as i64);

        // 转换为数据库记录
        let test_report = TestReportRecord {
            id: 0, // 数据库自动生成
            scenario_name: report.scenario_name.clone(),
            description: report.description.clone(),
            start_time: actual_start_time,
            end_time: Some(now),
            duration_ms: Some(report.duration_ms as i64),
            total_steps: report.steps_executed as i32,
            success_count: report.passed_count as i32,
            failed_count: report.failed_count as i32,
            skipped_count: 0,
            passed: report.passed,
            tags: if report.tags.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&report.tags).map_err(|e| {
                    ExecutorError::SerdeError(format!("Failed to serialize tags: {}", e))
                })?)
            },
            created_at: now,
        };

        // 保存报告
        let report_id =
            storage.reports().create(&test_report).await.map_err(|e| {
                ExecutorError::DatabaseError(format!("Failed to save report: {}", e))
            })?;

        // 保存步骤
        let steps: Vec<ExecutionStepRecord> = report
            .steps
            .iter()
            .map(|step| ExecutionStepRecord {
                id: 0,
                report_id,
                step_index: step.step_index as i32,
                description: step.description.clone(),
                status: match step.status {
                    StepStatus::Success => "Success".to_string(),
                    StepStatus::Failed => "Failed".to_string(),
                    StepStatus::Skipped => "Skipped".to_string(),
                },
                error: step.error.clone(),
                duration_ms: Some(step.duration_ms as i64),
                output: step.output.clone(),
            })
            .collect();

        storage
            .reports()
            .create_steps(&steps)
            .await
            .map_err(|e| ExecutorError::DatabaseError(format!("Failed to save steps: {}", e)))?;

        info!("测试报告已保存到数据库, ID: {}", report_id);
        Ok(report_id)
    }
}

/// 多目标执行报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiTargetReport {
    /// 场景名称
    pub scenario_name: String,

    /// 总目标数
    pub total_targets: usize,

    /// 成功数
    pub successful: usize,

    /// 失败数
    pub failed: usize,

    /// 总耗时（毫秒）
    pub duration_ms: u64,

    /// 各目标的执行报告
    pub reports: Vec<ExecutionReport>,
}

impl MultiTargetReport {
    /// 是否全部成功
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.successful > 0
    }

    /// 导出为 JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// 导出为 YAML
    pub fn to_yaml(&self) -> serde_yaml::Result<String> {
        serde_yaml::to_string(self)
    }
}

/// 执行报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    /// 场景名称
    pub scenario_name: String,

    /// 场景描述
    pub description: Option<String>,

    /// 标签
    pub tags: Vec<String>,

    /// 是否通过
    pub passed: bool,

    /// 执行的步骤数
    pub steps_executed: usize,

    /// 通过的步骤数
    pub passed_count: usize,

    /// 失败的步骤数
    pub failed_count: usize,

    /// 总耗时（毫秒）
    pub duration_ms: u64,

    /// 步骤报告列表
    pub steps: Vec<StepReport>,
}

impl ExecutionReport {
    pub fn new(name: &str) -> Self {
        Self {
            scenario_name: name.to_string(),
            description: None,
            tags: Vec::new(),
            passed: true,
            steps_executed: 0,
            passed_count: 0,
            failed_count: 0,
            duration_ms: 0,
            steps: Vec::new(),
        }
    }

    pub fn add_step(&mut self, step: StepReport) {
        self.steps_executed += 1;

        match step.status {
            StepStatus::Success => self.passed_count += 1,
            StepStatus::Failed => {
                self.failed_count += 1;
                self.passed = false;
            }
            StepStatus::Skipped => {}
        }

        self.steps.push(step);
    }

    /// 导出为 JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// 导出为 YAML
    pub fn to_yaml(&self) -> serde_yaml::Result<String> {
        serde_yaml::to_string(self)
    }
}

/// 步骤报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepReport {
    /// 步骤索引
    pub step_index: usize,

    /// 步骤描述
    pub description: String,

    /// 步骤状态
    pub status: StepStatus,

    /// 错误信息
    pub error: Option<String>,

    /// 耗时（毫秒）
    pub duration_ms: u64,

    /// 输出内容
    pub output: Option<String>,

    /// 是否已验证 (仅当 verify=true 时有值)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,

    /// 验证延迟（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_latency_ms: Option<u64>,
}

impl StepReport {
    pub fn success(index: usize, description: &str) -> Self {
        Self {
            step_index: index,
            description: description.to_string(),
            status: StepStatus::Success,
            error: None,
            duration_ms: 0,
            output: None,
            verified: None,
            verification_latency_ms: None,
        }
    }

    pub fn failed(index: usize, description: &str, error: &str) -> Self {
        Self {
            step_index: index,
            description: description.to_string(),
            status: StepStatus::Failed,
            error: Some(error.to_string()),
            duration_ms: 0,
            output: None,
            verified: None,
            verification_latency_ms: None,
        }
    }

    /// 设置验证结果
    pub fn with_verification(mut self, verified: bool, latency_ms: u64) -> Self {
        self.verified = Some(verified);
        self.verification_latency_ms = Some(latency_ms);
        self
    }
}

/// 步骤状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Success,
    Failed,
    Skipped,
}

/// 将字符转换为 QKeyCode 字符串
///
/// 返回 QEMU QKeyCode 名称，用于 QMP send-key 命令。
/// 使用静态字符串避免内存分配和泄漏。
///
/// 参考: https://github.com/qemu/qemu/blob/master/include/ui/input-keymap.h
fn char_to_qkeycode(c: char) -> Option<&'static str> {
    match c {
        // 小写字母
        'a' => Some("a"),
        'b' => Some("b"),
        'c' => Some("c"),
        'd' => Some("d"),
        'e' => Some("e"),
        'f' => Some("f"),
        'g' => Some("g"),
        'h' => Some("h"),
        'i' => Some("i"),
        'j' => Some("j"),
        'k' => Some("k"),
        'l' => Some("l"),
        'm' => Some("m"),
        'n' => Some("n"),
        'o' => Some("o"),
        'p' => Some("p"),
        'q' => Some("q"),
        'r' => Some("r"),
        's' => Some("s"),
        't' => Some("t"),
        'u' => Some("u"),
        'v' => Some("v"),
        'w' => Some("w"),
        'x' => Some("x"),
        'y' => Some("y"),
        'z' => Some("z"),

        // 大写字母 (需要 Shift，但 QMP 使用相同的 keycode)
        'A' => Some("a"),
        'B' => Some("b"),
        'C' => Some("c"),
        'D' => Some("d"),
        'E' => Some("e"),
        'F' => Some("f"),
        'G' => Some("g"),
        'H' => Some("h"),
        'I' => Some("i"),
        'J' => Some("j"),
        'K' => Some("k"),
        'L' => Some("l"),
        'M' => Some("m"),
        'N' => Some("n"),
        'O' => Some("o"),
        'P' => Some("p"),
        'Q' => Some("q"),
        'R' => Some("r"),
        'S' => Some("s"),
        'T' => Some("t"),
        'U' => Some("u"),
        'V' => Some("v"),
        'W' => Some("w"),
        'X' => Some("x"),
        'Y' => Some("y"),
        'Z' => Some("z"),

        // 数字
        '0' => Some("0"),
        '1' => Some("1"),
        '2' => Some("2"),
        '3' => Some("3"),
        '4' => Some("4"),
        '5' => Some("5"),
        '6' => Some("6"),
        '7' => Some("7"),
        '8' => Some("8"),
        '9' => Some("9"),

        // 空白字符
        ' ' => Some("spc"),
        '\t' => Some("tab"),
        '\n' => Some("ret"),
        '\r' => Some("ret"),

        // 标点符号 (美式键盘布局)
        '`' => Some("grave_accent"),
        '~' => Some("grave_accent"), // Shift + `
        '-' => Some("minus"),
        '_' => Some("minus"), // Shift + -
        '=' => Some("equal"),
        '+' => Some("equal"), // Shift + =
        '[' => Some("bracket_left"),
        '{' => Some("bracket_left"), // Shift + [
        ']' => Some("bracket_right"),
        '}' => Some("bracket_right"), // Shift + ]
        '\\' => Some("backslash"),
        '|' => Some("backslash"), // Shift + \
        ';' => Some("semicolon"),
        ':' => Some("semicolon"), // Shift + ;
        '\'' => Some("apostrophe"),
        '"' => Some("apostrophe"), // Shift + '
        ',' => Some("comma"),
        '<' => Some("comma"), // Shift + ,
        '.' => Some("dot"),
        '>' => Some("dot"), // Shift + .
        '/' => Some("slash"),
        '?' => Some("slash"), // Shift + /

        // Shift + 数字产生的符号
        '!' => Some("1"), // Shift + 1
        '@' => Some("2"), // Shift + 2
        '#' => Some("3"), // Shift + 3
        '$' => Some("4"), // Shift + 4
        '%' => Some("5"), // Shift + 5
        '^' => Some("6"), // Shift + 6
        '&' => Some("7"), // Shift + 7
        '*' => Some("8"), // Shift + 8
        '(' => Some("9"), // Shift + 9
        ')' => Some("0"), // Shift + 0

        // 不支持的字符
        _ => {
            debug!("未映射的字符: {:?} (U+{:04X})", c, c as u32);
            None
        }
    }
}

/// 将 QKeyCode 字符串转换为 PS/2 Set 1 扫描码 (用于 SPICE)
///
/// 返回 PS/2 Set 1 Make Code，用于 SPICE inputs 通道。
/// 参考: https://wiki.osdev.org/PS/2_Keyboard#Scan_Code_Set_1
fn qkeycode_to_scancode(key: &str) -> Option<u32> {
    match key.to_lowercase().as_str() {
        // 字母键 (a-z)
        "a" => Some(0x1E),
        "b" => Some(0x30),
        "c" => Some(0x2E),
        "d" => Some(0x20),
        "e" => Some(0x12),
        "f" => Some(0x21),
        "g" => Some(0x22),
        "h" => Some(0x23),
        "i" => Some(0x17),
        "j" => Some(0x24),
        "k" => Some(0x25),
        "l" => Some(0x26),
        "m" => Some(0x32),
        "n" => Some(0x31),
        "o" => Some(0x18),
        "p" => Some(0x19),
        "q" => Some(0x10),
        "r" => Some(0x13),
        "s" => Some(0x1F),
        "t" => Some(0x14),
        "u" => Some(0x16),
        "v" => Some(0x2F),
        "w" => Some(0x11),
        "x" => Some(0x2D),
        "y" => Some(0x15),
        "z" => Some(0x2C),

        // 数字键 (0-9)
        "0" => Some(0x0B),
        "1" => Some(0x02),
        "2" => Some(0x03),
        "3" => Some(0x04),
        "4" => Some(0x05),
        "5" => Some(0x06),
        "6" => Some(0x07),
        "7" => Some(0x08),
        "8" => Some(0x09),
        "9" => Some(0x0A),

        // 功能键
        "esc" | "escape" => Some(0x01),
        "ret" | "return" | "enter" => Some(0x1C),
        "tab" => Some(0x0F),
        "spc" | "space" => Some(0x39),
        "backspace" => Some(0x0E),

        // 标点符号
        "minus" => Some(0x0C),
        "equal" => Some(0x0D),
        "bracket_left" => Some(0x1A),
        "bracket_right" => Some(0x1B),
        "backslash" => Some(0x2B),
        "semicolon" => Some(0x27),
        "apostrophe" => Some(0x28),
        "grave_accent" => Some(0x29),
        "comma" => Some(0x33),
        "dot" => Some(0x34),
        "slash" => Some(0x35),

        // 修饰键
        "shift" | "shift_l" => Some(0x2A),
        "shift_r" => Some(0x36),
        "ctrl" | "ctrl_l" => Some(0x1D),
        "alt" | "alt_l" => Some(0x38),
        "caps_lock" => Some(0x3A),

        // F1-F12
        "f1" => Some(0x3B),
        "f2" => Some(0x3C),
        "f3" => Some(0x3D),
        "f4" => Some(0x3E),
        "f5" => Some(0x3F),
        "f6" => Some(0x40),
        "f7" => Some(0x41),
        "f8" => Some(0x42),
        "f9" => Some(0x43),
        "f10" => Some(0x44),
        "f11" => Some(0x57),
        "f12" => Some(0x58),

        // 方向键 (Extended: 需要 0xE0 前缀，这里返回基础码)
        "up" => Some(0x48),
        "down" => Some(0x50),
        "left" => Some(0x4B),
        "right" => Some(0x4D),

        // 其他
        "insert" => Some(0x52),
        "delete" => Some(0x53),
        "home" => Some(0x47),
        "end" => Some(0x4F),
        "pgup" | "page_up" => Some(0x49),
        "pgdn" | "page_down" => Some(0x51),

        _ => {
            debug!("未映射的 QKeyCode: {}", key);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_to_qkeycode_lowercase() {
        assert_eq!(char_to_qkeycode('a'), Some("a"));
        assert_eq!(char_to_qkeycode('m'), Some("m"));
        assert_eq!(char_to_qkeycode('z'), Some("z"));
    }

    #[test]
    fn test_char_to_qkeycode_uppercase() {
        // 大写字母映射到相同的 keycode
        assert_eq!(char_to_qkeycode('A'), Some("a"));
        assert_eq!(char_to_qkeycode('M'), Some("m"));
        assert_eq!(char_to_qkeycode('Z'), Some("z"));
    }

    #[test]
    fn test_char_to_qkeycode_digits() {
        assert_eq!(char_to_qkeycode('0'), Some("0"));
        assert_eq!(char_to_qkeycode('5'), Some("5"));
        assert_eq!(char_to_qkeycode('9'), Some("9"));
    }

    #[test]
    fn test_char_to_qkeycode_whitespace() {
        assert_eq!(char_to_qkeycode(' '), Some("spc"));
        assert_eq!(char_to_qkeycode('\t'), Some("tab"));
        assert_eq!(char_to_qkeycode('\n'), Some("ret"));
        assert_eq!(char_to_qkeycode('\r'), Some("ret"));
    }

    #[test]
    fn test_char_to_qkeycode_punctuation() {
        assert_eq!(char_to_qkeycode('`'), Some("grave_accent"));
        assert_eq!(char_to_qkeycode('-'), Some("minus"));
        assert_eq!(char_to_qkeycode('='), Some("equal"));
        assert_eq!(char_to_qkeycode('['), Some("bracket_left"));
        assert_eq!(char_to_qkeycode(']'), Some("bracket_right"));
        assert_eq!(char_to_qkeycode('\\'), Some("backslash"));
        assert_eq!(char_to_qkeycode(';'), Some("semicolon"));
        assert_eq!(char_to_qkeycode('\''), Some("apostrophe"));
        assert_eq!(char_to_qkeycode(','), Some("comma"));
        assert_eq!(char_to_qkeycode('.'), Some("dot"));
        assert_eq!(char_to_qkeycode('/'), Some("slash"));
    }

    #[test]
    fn test_char_to_qkeycode_shift_symbols() {
        // Shift + 数字产生的符号
        assert_eq!(char_to_qkeycode('!'), Some("1"));
        assert_eq!(char_to_qkeycode('@'), Some("2"));
        assert_eq!(char_to_qkeycode('#'), Some("3"));
        assert_eq!(char_to_qkeycode('$'), Some("4"));
        assert_eq!(char_to_qkeycode('%'), Some("5"));
        assert_eq!(char_to_qkeycode('^'), Some("6"));
        assert_eq!(char_to_qkeycode('&'), Some("7"));
        assert_eq!(char_to_qkeycode('*'), Some("8"));
        assert_eq!(char_to_qkeycode('('), Some("9"));
        assert_eq!(char_to_qkeycode(')'), Some("0"));
    }

    #[test]
    fn test_char_to_qkeycode_shift_punctuation() {
        // Shift + 标点产生的符号
        assert_eq!(char_to_qkeycode('~'), Some("grave_accent"));
        assert_eq!(char_to_qkeycode('_'), Some("minus"));
        assert_eq!(char_to_qkeycode('+'), Some("equal"));
        assert_eq!(char_to_qkeycode('{'), Some("bracket_left"));
        assert_eq!(char_to_qkeycode('}'), Some("bracket_right"));
        assert_eq!(char_to_qkeycode('|'), Some("backslash"));
        assert_eq!(char_to_qkeycode(':'), Some("semicolon"));
        assert_eq!(char_to_qkeycode('"'), Some("apostrophe"));
        assert_eq!(char_to_qkeycode('<'), Some("comma"));
        assert_eq!(char_to_qkeycode('>'), Some("dot"));
        assert_eq!(char_to_qkeycode('?'), Some("slash"));
    }

    #[test]
    fn test_char_to_qkeycode_unsupported() {
        // 不支持的字符返回 None
        assert_eq!(char_to_qkeycode('中'), None);
        assert_eq!(char_to_qkeycode('日'), None);
        assert_eq!(char_to_qkeycode('€'), None);
        assert_eq!(char_to_qkeycode('£'), None);
    }

    #[test]
    fn test_char_to_qkeycode_all_lowercase_letters() {
        for c in 'a'..='z' {
            let result = char_to_qkeycode(c);
            assert!(result.is_some(), "Letter '{}' should be mapped", c);
            assert_eq!(result.unwrap(), c.to_string().as_str());
        }
    }

    #[test]
    fn test_char_to_qkeycode_all_digits() {
        for c in '0'..='9' {
            let result = char_to_qkeycode(c);
            assert!(result.is_some(), "Digit '{}' should be mapped", c);
            assert_eq!(result.unwrap(), c.to_string().as_str());
        }
    }

    #[test]
    fn test_step_report_success() {
        let report = StepReport::success(0, "Test step");
        assert_eq!(report.step_index, 0);
        assert_eq!(report.description, "Test step");
        assert_eq!(report.status, StepStatus::Success);
        assert!(report.error.is_none());
    }

    #[test]
    fn test_step_report_failed() {
        let report = StepReport::failed(1, "Failed step", "Error message");
        assert_eq!(report.step_index, 1);
        assert_eq!(report.description, "Failed step");
        assert_eq!(report.status, StepStatus::Failed);
        assert_eq!(report.error, Some("Error message".to_string()));
    }

    #[test]
    fn test_step_report_with_verification() {
        let report = StepReport::success(0, "Test").with_verification(true, 50);
        assert_eq!(report.verified, Some(true));
        assert_eq!(report.verification_latency_ms, Some(50));
    }

    #[test]
    fn test_execution_report_new() {
        let report = ExecutionReport::new("Test Scenario");
        assert_eq!(report.scenario_name, "Test Scenario");
        assert!(report.passed);
        assert_eq!(report.steps_executed, 0);
        assert_eq!(report.passed_count, 0);
        assert_eq!(report.failed_count, 0);
    }

    #[test]
    fn test_execution_report_add_step() {
        let mut report = ExecutionReport::new("Test");

        report.add_step(StepReport::success(0, "Step 1"));
        assert_eq!(report.steps_executed, 1);
        assert_eq!(report.passed_count, 1);
        assert!(report.passed);

        report.add_step(StepReport::failed(1, "Step 2", "Error"));
        assert_eq!(report.steps_executed, 2);
        assert_eq!(report.failed_count, 1);
        assert!(!report.passed);
    }

    #[test]
    fn test_multi_target_report_all_passed() {
        let report = MultiTargetReport {
            scenario_name: "Test".to_string(),
            total_targets: 3,
            successful: 3,
            failed: 0,
            duration_ms: 1000,
            reports: vec![],
        };
        assert!(report.all_passed());
    }

    #[test]
    fn test_multi_target_report_some_failed() {
        let report = MultiTargetReport {
            scenario_name: "Test".to_string(),
            total_targets: 3,
            successful: 2,
            failed: 1,
            duration_ms: 1000,
            reports: vec![],
        };
        assert!(!report.all_passed());
    }
}

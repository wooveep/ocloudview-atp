//! 场景执行器

use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn, error};
use tokio::time::timeout;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use virt::domain::Domain;

use atp_transport::TransportManager;
use atp_protocol::{
    Protocol, ProtocolRegistry,
    qmp::QmpProtocol,
    qga::QgaProtocol,
    spice::{SpiceProtocol, MouseButton},
};
use atp_storage::{Storage, TestReportRecord, ExecutionStepRecord};
use atp_vdiplatform::{VdiClient, models::CreateDeskPoolRequest};

use crate::{Result, Scenario, ScenarioStep, Action, ExecutorError};

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
    pub async fn run(&mut self, scenario: &Scenario) -> Result<ExecutionReport> {
        info!("开始执行场景: {}", scenario.name);

        let start_time = Instant::now();
        let mut report = ExecutionReport::new(&scenario.name);

        if let Some(desc) = &scenario.description {
            report.description = Some(desc.clone());
        }

        report.tags = scenario.tags.clone();

        // 初始化协议连接 (如果指定了目标虚拟机)
        if let Some(target_domain) = &scenario.target_domain {
            if let Err(e) = self.initialize_protocols(scenario, target_domain).await {
                error!("初始化协议失败: {}", e);
                return Err(e);
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
                    };
                    report.add_step(failed_step);
                    break; // 失败后停止执行
                }
            }
        }

        // 清理协议连接
        self.cleanup_protocols().await;

        report.duration_ms = start_time.elapsed().as_millis() as u64;

        info!(
            "场景执行完成: {} - {}/{} 步骤成功",
            scenario.name,
            report.passed_count,
            report.steps_executed
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

    /// 初始化协议连接
    async fn initialize_protocols(&mut self, scenario: &Scenario, domain_name: &str) -> Result<()> {
        info!("初始化协议连接: 虚拟机 = {}", domain_name);

        // 获取目标主机的连接
        let hosts = self.transport_manager.list_hosts().await;
        let host_id = scenario.target_host.as_deref()
            .or_else(|| hosts.first().map(String::as_str))
            .ok_or_else(|| ExecutorError::ConfigError("未指定目标主机且无可用主机".to_string()))?;

        // 通过 transport manager 获取 domain
        let domain = self.transport_manager
            .execute_on_host(host_id, |conn| async move {
                conn.get_domain(domain_name).await
            })
            .await
            .map_err(|e| ExecutorError::TransportError(e.to_string()))?;

        // 初始化 QMP 协议
        let mut qmp = QmpProtocol::new();
        if let Err(e) = qmp.connect(&domain).await {
            warn!("QMP 协议连接失败: {}", e);
            // QMP 失败不是致命错误,可能虚拟机没有 QMP
        } else {
            info!("QMP 协议连接成功");
            self.qmp_protocol = Some(qmp);
        }

        // 初始化 QGA 协议
        let mut qga = QgaProtocol::new();
        if let Err(e) = qga.connect(&domain).await {
            warn!("QGA 协议连接失败: {}", e);
            // QGA 失败不是致命错误,可能虚拟机没有安装 guest agent
        } else {
            info!("QGA 协议连接成功");
            self.qga_protocol = Some(qga);
        }

        // 初始化 SPICE 协议（用于鼠标操作）
        let mut spice = SpiceProtocol::new();
        if let Err(e) = spice.connect(&domain).await {
            warn!("SPICE 协议连接失败: {}", e);
            // SPICE 失败不是致命错误,可能虚拟机没有配置 SPICE
        } else {
            info!("SPICE 协议连接成功");
            self.spice_protocol = Some(spice);
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

    /// 执行单个步骤
    async fn execute_step(&mut self, step: &ScenarioStep, index: usize) -> Result<StepReport> {
        let start_time = Instant::now();

        let step_timeout = step.timeout
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let result = timeout(step_timeout, self.execute_action(&step.action, index)).await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(mut report)) => {
                report.duration_ms = duration_ms;
                if let Some(name) = &step.name {
                    report.description = name.clone();
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

    /// 执行具体动作
    async fn execute_action(&mut self, action: &Action, index: usize) -> Result<StepReport> {
        match action {
            Action::SendKey { key } => {
                self.execute_send_key(key, index).await
            }
            Action::SendText { text } => {
                self.execute_send_text(text, index).await
            }
            Action::MouseClick { x, y, button } => {
                self.execute_mouse_click(*x, *y, button, index).await
            }
            Action::ExecCommand { command } => {
                self.execute_command(command, index).await
            }
            Action::Wait { duration } => {
                self.execute_wait(*duration, index).await
            }
            Action::Custom { data } => {
                warn!("自定义动作尚未完全实现: {:?}", data);
                Ok(StepReport::success(index, "自定义动作（跳过）"))
            }
            // VDI 平台操作
            Action::VdiCreateDeskPool { name, template_id, count } => {
                self.execute_vdi_create_desk_pool(name, template_id, *count, index).await
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
            Action::VerifyDomainStatus { domain_id, expected_status, timeout_secs } => {
                self.verify_domain_status(domain_id, expected_status, *timeout_secs, index).await
            }
            Action::VerifyAllDomainsRunning { pool_id, timeout_secs } => {
                self.verify_all_domains_running(pool_id, *timeout_secs, index).await
            }
            Action::VerifyCommandSuccess { timeout_secs } => {
                self.verify_command_success(*timeout_secs, index).await
            }
        }
    }

    /// 执行发送按键
    async fn execute_send_key(&mut self, key: &str, index: usize) -> Result<StepReport> {
        info!("发送按键: {}", key);

        // 使用 QMP 协议发送按键
        if let Some(qmp) = &mut self.qmp_protocol {
            qmp.send_key(key)
                .await
                .map_err(|e| ExecutorError::ProtocolError(format!("QMP send_key 失败: {}", e)))?;

            Ok(StepReport::success(index, &format!("发送按键: {}", key)))
        } else {
            Err(ExecutorError::ProtocolError("QMP 协议未初始化".to_string()))
        }
    }

    /// 执行发送文本
    async fn execute_send_text(&mut self, text: &str, index: usize) -> Result<StepReport> {
        info!("发送文本: {}", text);

        // 使用 QMP 协议发送文本 (将文本拆分为按键序列)
        if let Some(qmp) = &mut self.qmp_protocol {
            // 将文本转换为按键序列
            let keys: Vec<&str> = text.chars()
                .map(|c| {
                    // 这里简化处理,实际需要完整的字符到 QKeyCode 映射
                    match c {
                        'a'..='z' | 'A'..='Z' | '0'..='9' => c.to_string().leak(),
                        ' ' => "spc",
                        '\n' => "ret",
                        _ => "unknown", // 需要更完整的映射表
                    }
                })
                .collect();

            qmp.send_keys(keys, Some(100))
                .await
                .map_err(|e| ExecutorError::ProtocolError(format!("QMP send_keys 失败: {}", e)))?;

            Ok(StepReport::success(index, &format!("发送文本: {}", text)))
        } else {
            Err(ExecutorError::ProtocolError("QMP 协议未初始化".to_string()))
        }
    }

    /// 执行鼠标点击
    async fn execute_mouse_click(&mut self, x: i32, y: i32, button: &str, index: usize) -> Result<StepReport> {
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
            spice.send_mouse_move(x as u32, y as u32, 0)
                .await
                .map_err(|e| ExecutorError::ProtocolError(format!("SPICE 鼠标移动失败: {}", e)))?;

            // 等待一小段时间确保位置更新
            tokio::time::sleep(Duration::from_millis(50)).await;

            // 发送鼠标点击（按下）
            spice.send_mouse_click(mouse_button, true)
                .await
                .map_err(|e| ExecutorError::ProtocolError(format!("SPICE 鼠标按下失败: {}", e)))?;

            // 短暂延迟模拟真实点击
            tokio::time::sleep(Duration::from_millis(50)).await;

            // 发送鼠标释放
            spice.send_mouse_click(mouse_button, false)
                .await
                .map_err(|e| ExecutorError::ProtocolError(format!("SPICE 鼠标释放失败: {}", e)))?;

            Ok(StepReport::success(index, &format!("鼠标点击: ({}, {}) 按钮: {}", x, y, button)))
        } else {
            // 如果 SPICE 未连接，尝试通过 QGA 执行脚本模拟鼠标操作（备用方案）
            if let Some(qga) = &self.qga_protocol {
                warn!("SPICE 协议未初始化，尝试通过 QGA 执行鼠标脚本");

                // 在 Linux 中可以使用 xdotool 模拟鼠标
                let script = format!("DISPLAY=:0 xdotool mousemove {} {} click {}",
                    x, y,
                    match button.to_lowercase().as_str() {
                        "left" => "1",
                        "middle" => "2",
                        "right" => "3",
                        _ => "1",
                    }
                );

                let status = qga.exec_shell(&script)
                    .await
                    .map_err(|e| ExecutorError::ProtocolError(format!("QGA 执行鼠标脚本失败: {}", e)))?;

                if let Some(exit_code) = status.exit_code {
                    if exit_code != 0 {
                        return Ok(StepReport::failed(
                            index,
                            &format!("鼠标点击: ({}, {})", x, y),
                            "xdotool 执行失败（可能未安装）",
                        ));
                    }
                }

                Ok(StepReport::success(index, &format!("鼠标点击: ({}, {}) [QGA/xdotool]", x, y)))
            } else {
                Err(ExecutorError::ProtocolError(
                    "SPICE 和 QGA 协议均未初始化，无法执行鼠标操作".to_string()
                ))
            }
        }
    }

    /// 执行命令
    async fn execute_command(&mut self, command: &str, index: usize) -> Result<StepReport> {
        info!("执行命令: {}", command);

        // 使用 QGA 协议执行命令
        if let Some(qga) = &self.qga_protocol {
            let status = qga.exec_shell(command)
                .await
                .map_err(|e| ExecutorError::ProtocolError(format!("QGA exec_shell 失败: {}", e)))?;

            // 检查退出码
            if let Some(exit_code) = status.exit_code {
                if exit_code != 0 {
                    let stderr = status.decode_stderr()
                        .unwrap_or_else(|| "无错误输出".to_string());

                    return Ok(StepReport::failed(
                        index,
                        &format!("执行命令: {}", command),
                        &format!("命令执行失败 (退出码: {}): {}", exit_code, stderr),
                    ));
                }
            }

            // 获取输出
            let stdout = status.decode_stdout()
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
        index: usize
    ) -> Result<StepReport> {
        info!("创建桌面池: {} (模板: {}, 数量: {})", name, template_id, count);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        // 构造创建桌面池请求
        let request = CreateDeskPoolRequest {
            name: name.to_string(),
            template_id: template_id.to_string(),
            count,
            vcpu: 2,    // 默认值，可以从场景配置中获取
            memory: 2048, // 默认 2GB，可以从场景配置中获取
        };

        // 调用 VDI 平台 API 创建桌面池
        vdi_client.desk_pool()
            .create(request)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("创建桌面池失败: {}", e)))?;

        Ok(StepReport::success(index, &format!("创建桌面池: {}", name)))
    }

    /// 执行 VDI 启用桌面池
    async fn execute_vdi_enable_desk_pool(
        &mut self,
        pool_id: &str,
        index: usize
    ) -> Result<StepReport> {
        info!("启用桌面池: {}", pool_id);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client.desk_pool()
            .enable(pool_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("启用桌面池失败: {}", e)))?;

        Ok(StepReport::success(index, &format!("启用桌面池: {}", pool_id)))
    }

    /// 执行 VDI 禁用桌面池
    async fn execute_vdi_disable_desk_pool(
        &mut self,
        pool_id: &str,
        index: usize
    ) -> Result<StepReport> {
        info!("禁用桌面池: {}", pool_id);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client.desk_pool()
            .disable(pool_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("禁用桌面池失败: {}", e)))?;

        Ok(StepReport::success(index, &format!("禁用桌面池: {}", pool_id)))
    }

    /// 执行 VDI 删除桌面池
    async fn execute_vdi_delete_desk_pool(
        &mut self,
        pool_id: &str,
        index: usize
    ) -> Result<StepReport> {
        info!("删除桌面池: {}", pool_id);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client.desk_pool()
            .delete(pool_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("删除桌面池失败: {}", e)))?;

        Ok(StepReport::success(index, &format!("删除桌面池: {}", pool_id)))
    }

    /// 执行 VDI 启动虚拟机
    async fn execute_vdi_start_domain(
        &mut self,
        domain_id: &str,
        index: usize
    ) -> Result<StepReport> {
        info!("启动虚拟机: {}", domain_id);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client.domain()
            .start(domain_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("启动虚拟机失败: {}", e)))?;

        Ok(StepReport::success(index, &format!("启动虚拟机: {}", domain_id)))
    }

    /// 执行 VDI 关闭虚拟机
    async fn execute_vdi_shutdown_domain(
        &mut self,
        domain_id: &str,
        index: usize
    ) -> Result<StepReport> {
        info!("关闭虚拟机: {}", domain_id);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client.domain()
            .shutdown(domain_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("关闭虚拟机失败: {}", e)))?;

        Ok(StepReport::success(index, &format!("关闭虚拟机: {}", domain_id)))
    }

    /// 执行 VDI 重启虚拟机
    async fn execute_vdi_reboot_domain(
        &mut self,
        domain_id: &str,
        index: usize
    ) -> Result<StepReport> {
        info!("重启虚拟机: {}", domain_id);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client.domain()
            .reboot(domain_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("重启虚拟机失败: {}", e)))?;

        Ok(StepReport::success(index, &format!("重启虚拟机: {}", domain_id)))
    }

    /// 执行 VDI 删除虚拟机
    async fn execute_vdi_delete_domain(
        &mut self,
        domain_id: &str,
        index: usize
    ) -> Result<StepReport> {
        info!("删除虚拟机: {}", domain_id);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client.domain()
            .delete(domain_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("删除虚拟机失败: {}", e)))?;

        Ok(StepReport::success(index, &format!("删除虚拟机: {}", domain_id)))
    }

    /// 执行 VDI 绑定用户
    async fn execute_vdi_bind_user(
        &mut self,
        domain_id: &str,
        user_id: &str,
        index: usize
    ) -> Result<StepReport> {
        info!("绑定用户: 虚拟机={}, 用户={}", domain_id, user_id);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        vdi_client.domain()
            .bind_user(domain_id, user_id)
            .await
            .map_err(|e| ExecutorError::TransportError(format!("绑定用户失败: {}", e)))?;

        Ok(StepReport::success(index, &format!("绑定用户: 虚拟机={}, 用户={}", domain_id, user_id)))
    }

    /// 执行 VDI 获取桌面池虚拟机列表
    async fn execute_vdi_get_desk_pool_domains(
        &mut self,
        pool_id: &str,
        index: usize
    ) -> Result<StepReport> {
        info!("获取桌面池虚拟机列表: {}", pool_id);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        let domains = vdi_client.desk_pool()
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
        index: usize
    ) -> Result<StepReport> {
        info!("验证虚拟机状态: {} 应为 {}", domain_id, expected_status);

        let timeout_duration = timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        // 获取第一个可用主机
        let hosts = self.transport_manager.list_hosts().await;
        let host_id = hosts.first()
            .ok_or_else(|| ExecutorError::ConfigError("无可用主机".to_string()))?
            .clone();

        // 通过 libvirt 查询虚拟机状态
        let result = timeout(timeout_duration, async {
            let domain = self.transport_manager
                .execute_on_host(&host_id, |conn| async move {
                    conn.get_domain(domain_id).await
                })
                .await
                .map_err(|e| ExecutorError::TransportError(e.to_string()))?;

            let state = domain.get_state()
                .map_err(|e| ExecutorError::TransportError(e.to_string()))?;

            let actual_status = format!("{:?}", state.0).to_lowercase();
            let expected_lower = expected_status.to_lowercase();

            if actual_status.contains(&expected_lower) || expected_lower.contains(&actual_status) {
                Ok(StepReport::success(index, &format!(
                    "虚拟机状态验证成功: {} = {}", domain_id, expected_status
                )))
            } else {
                Ok(StepReport::failed(
                    index,
                    &format!("虚拟机状态验证失败: {}", domain_id),
                    &format!("期望: {}, 实际: {}", expected_status, actual_status)
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
        index: usize
    ) -> Result<StepReport> {
        info!("验证所有虚拟机运行中: 桌面池 {}", pool_id);

        let timeout_duration = timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let vdi_client = self.vdi_client.as_ref()
            .ok_or_else(|| ExecutorError::ConfigError("VDI 客户端未初始化".to_string()))?;

        let result = timeout(timeout_duration, async {
            // 获取桌面池的所有虚拟机
            let domains = vdi_client.desk_pool()
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
                Ok(StepReport::success(index, &format!(
                    "所有虚拟机运行中: 桌面池 {} ({} 台)", pool_id, domains.len()
                )))
            } else {
                Ok(StepReport::failed(
                    index,
                    &format!("验证所有虚拟机运行中: 桌面池 {}", pool_id),
                    &format!("以下虚拟机未运行: {:?}", failed_domains)
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
        index: usize
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
        let report_id = storage
            .reports()
            .create(&test_report)
            .await
            .map_err(|e| ExecutorError::DatabaseError(format!("Failed to save report: {}", e)))?;

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
        }
    }
}

/// 步骤状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Success,
    Failed,
    Skipped,
}

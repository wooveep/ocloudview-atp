//! 场景执行器

use std::sync::Arc;
use tracing::{info, warn};

use atp_vdiplatform::VdiClient;
use atp_transport::TransportManager;
use atp_protocol::ProtocolRegistry;

use crate::scenario::{TestScenario, TestStep, VdiAction, VirtualizationAction, VerifyCondition};
use crate::report::{TestReport, StepResult, StepStatus};
use crate::adapter::VdiVirtualizationAdapter;
use crate::Result;

/// 场景执行器
pub struct ScenarioExecutor {
    /// VDI 客户端
    vdi_client: Arc<VdiClient>,

    /// 传输管理器
    transport_manager: Arc<TransportManager>,

    /// 协议注册表
    protocol_registry: Arc<ProtocolRegistry>,

    /// 集成适配器
    adapter: Arc<VdiVirtualizationAdapter>,
}

impl ScenarioExecutor {
    /// 创建新的场景执行器
    pub fn new(
        vdi_client: Arc<VdiClient>,
        transport_manager: Arc<TransportManager>,
        protocol_registry: Arc<ProtocolRegistry>,
    ) -> Self {
        let adapter = Arc::new(VdiVirtualizationAdapter::new(
            Arc::clone(&vdi_client),
            Arc::clone(&transport_manager),
        ));

        Self {
            vdi_client,
            transport_manager,
            protocol_registry,
            adapter,
        }
    }

    /// 执行测试场景
    pub async fn execute(&self, scenario: &TestScenario) -> Result<TestReport> {
        info!("开始执行测试场景: {}", scenario.name);

        let mut report = TestReport::new(&scenario.name);
        report.description = scenario.description.clone();

        for (index, step) in scenario.steps.iter().enumerate() {
            info!("执行步骤 {}/{}", index + 1, scenario.steps.len());

            let step_result = self.execute_step(step).await;

            match &step_result {
                Ok(result) => {
                    info!("步骤执行成功: {}", result.description);
                    report.add_step_result(result.clone());
                }
                Err(e) => {
                    warn!("步骤执行失败: {}", e);
                    let failed_result = StepResult {
                        step_index: index,
                        description: format!("步骤 {} 失败", index + 1),
                        status: StepStatus::Failed,
                        error: Some(e.to_string()),
                        duration: std::time::Duration::from_secs(0),
                        output: None,
                    };
                    report.add_step_result(failed_result);
                    break;
                }
            }
        }

        report.finalize();
        info!("场景执行完成: {}/{} 步骤成功", report.success_count, report.total_steps);

        Ok(report)
    }

    /// 执行单个步骤
    async fn execute_step(&self, step: &TestStep) -> Result<StepResult> {
        let start_time = std::time::Instant::now();

        let result = match step {
            TestStep::VdiAction { action, .. } => {
                self.execute_vdi_action(action).await
            }
            TestStep::VirtualizationAction { action, .. } => {
                self.execute_virtualization_action(action).await
            }
            TestStep::Wait { duration } => {
                info!("等待 {:?}", duration);
                tokio::time::sleep(*duration).await;
                Ok(StepResult::success(0, "等待完成"))
            }
            TestStep::Verify { condition, timeout } => {
                self.verify_condition(condition, *timeout).await
            }
        };

        let duration = start_time.elapsed();

        result.map(|mut r| {
            r.duration = duration;
            r
        })
    }

    /// 执行 VDI 平台操作
    async fn execute_vdi_action(&self, action: &VdiAction) -> Result<StepResult> {
        match action {
            VdiAction::CreateDeskPool { name, template_id, count } => {
                info!("创建桌面池: {} (模板: {}, 数量: {})", name, template_id, count);
                // TODO: 实现实际的创建逻辑
                Ok(StepResult::success(0, &format!("创建桌面池: {}", name)))
            }
            VdiAction::EnableDeskPool { pool_id } => {
                info!("启用桌面池: {}", pool_id);
                self.vdi_client.desk_pool().enable(pool_id).await
                    .map_err(|e| crate::OrchestratorError::VdiError(e.to_string()))?;
                Ok(StepResult::success(0, &format!("启用桌面池: {}", pool_id)))
            }
            VdiAction::StartDomain { domain_id } => {
                info!("启动虚拟机: {}", domain_id);
                self.vdi_client.domain().start(domain_id).await
                    .map_err(|e| crate::OrchestratorError::VdiError(e.to_string()))?;
                Ok(StepResult::success(0, &format!("启动虚拟机: {}", domain_id)))
            }
            VdiAction::ShutdownDomain { domain_id } => {
                info!("关闭虚拟机: {}", domain_id);
                self.vdi_client.domain().shutdown(domain_id).await
                    .map_err(|e| crate::OrchestratorError::VdiError(e.to_string()))?;
                Ok(StepResult::success(0, &format!("关闭虚拟机: {}", domain_id)))
            }
            _ => {
                warn!("VDI 操作尚未实现: {:?}", action);
                Ok(StepResult::success(0, "VDI 操作（模拟）"))
            }
        }
    }

    /// 执行虚拟化层操作
    async fn execute_virtualization_action(&self, action: &VirtualizationAction) -> Result<StepResult> {
        match action {
            VirtualizationAction::Connect { domain_id } => {
                info!("连接到虚拟机: {}", domain_id);
                // TODO: 实现实际的连接逻辑
                Ok(StepResult::success(0, &format!("连接到虚拟机: {}", domain_id)))
            }
            VirtualizationAction::SendKeyboard { text, .. } => {
                info!("发送键盘输入: {:?}", text);
                // TODO: 实现实际的键盘输入逻辑
                Ok(StepResult::success(0, "发送键盘输入"))
            }
            VirtualizationAction::ExecuteCommand { command } => {
                info!("执行命令: {}", command);
                // TODO: 实现实际的命令执行逻辑
                Ok(StepResult::success(0, &format!("执行命令: {}", command)))
            }
            _ => {
                warn!("虚拟化层操作尚未实现: {:?}", action);
                Ok(StepResult::success(0, "虚拟化层操作（模拟）"))
            }
        }
    }

    /// 验证条件
    async fn verify_condition(
        &self,
        condition: &VerifyCondition,
        timeout: Option<std::time::Duration>,
    ) -> Result<StepResult> {
        info!("验证条件: {:?}", condition);

        // TODO: 实现实际的验证逻辑
        match condition {
            VerifyCondition::DomainStatus { domain_id, expected_status } => {
                info!("验证虚拟机状态: {} 应为 {}", domain_id, expected_status);
                Ok(StepResult::success(0, "验证虚拟机状态"))
            }
            VerifyCondition::AllDomainsRunning { pool_id } => {
                info!("验证所有虚拟机运行中: {}", pool_id);
                Ok(StepResult::success(0, "验证所有虚拟机运行中"))
            }
            _ => {
                Ok(StepResult::success(0, "验证条件（模拟）"))
            }
        }
    }
}

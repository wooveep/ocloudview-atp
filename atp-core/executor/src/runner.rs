//! 场景执行器

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{info, warn, error};
use tokio::time::timeout;
use serde::{Deserialize, Serialize};
use chrono::Utc;

use atp_transport::TransportManager;
use atp_protocol::{Protocol, ProtocolRegistry};
use atp_storage::{Storage, TestReportRecord, ExecutionStepRecord};

use crate::{Result, Scenario, ScenarioStep, Action, ExecutorError};

/// 场景执行器
pub struct ScenarioRunner {
    /// 传输管理器
    transport_manager: Arc<TransportManager>,

    /// 协议注册表
    protocol_registry: Arc<ProtocolRegistry>,

    /// 协议实例缓存（domain_id -> protocol）
    protocol_cache: HashMap<String, Box<dyn Protocol>>,

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
            protocol_cache: HashMap::new(),
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

    /// 执行场景
    pub async fn run(&mut self, scenario: &Scenario) -> Result<ExecutionReport> {
        info!("开始执行场景: {}", scenario.name);

        let start_time = Instant::now();
        let mut report = ExecutionReport::new(&scenario.name);

        if let Some(desc) = &scenario.description {
            report.description = Some(desc.clone());
        }

        report.tags = scenario.tags.clone();

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
        }
    }

    /// 执行发送按键
    async fn execute_send_key(&mut self, key: &str, index: usize) -> Result<StepReport> {
        info!("发送按键: {}", key);

        // TODO: 获取当前连接的协议实例
        // 这里需要从场景上下文中获取当前要操作的虚拟机
        // 暂时返回模拟结果

        Ok(StepReport::success(index, &format!("发送按键: {}", key)))
    }

    /// 执行发送文本
    async fn execute_send_text(&mut self, text: &str, index: usize) -> Result<StepReport> {
        info!("发送文本: {}", text);

        // TODO: 实现实际的文本发送

        Ok(StepReport::success(index, &format!("发送文本: {}", text)))
    }

    /// 执行鼠标点击
    async fn execute_mouse_click(&mut self, x: i32, y: i32, button: &str, index: usize) -> Result<StepReport> {
        info!("鼠标点击: ({}, {}) 按钮: {}", x, y, button);

        // TODO: 实现实际的鼠标点击

        Ok(StepReport::success(index, &format!("鼠标点击: ({}, {})", x, y)))
    }

    /// 执行命令
    async fn execute_command(&mut self, command: &str, index: usize) -> Result<StepReport> {
        info!("执行命令: {}", command);

        // TODO: 使用 QGA 协议执行命令

        Ok(StepReport::success(index, &format!("执行命令: {}", command)))
    }

    /// 执行等待
    async fn execute_wait(&self, duration: u64, index: usize) -> Result<StepReport> {
        info!("等待 {} 秒", duration);

        tokio::time::sleep(Duration::from_secs(duration)).await;

        Ok(StepReport::success(index, &format!("等待 {} 秒", duration)))
    }

    /// 保存报告到数据库
    async fn save_report_to_db(
        &self,
        storage: &Storage,
        report: &ExecutionReport,
        start_time: Instant,
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

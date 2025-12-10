//! 测试报告

use serde::{Deserialize, Serialize};
use std::time::Duration;
use chrono::{DateTime, Utc};

/// 测试报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReport {
    /// 场景名称
    pub name: String,

    /// 场景描述
    pub description: Option<String>,

    /// 开始时间
    pub start_time: DateTime<Utc>,

    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,

    /// 总耗时
    #[serde(skip)]
    pub duration: Duration,

    /// 总步骤数
    pub total_steps: usize,

    /// 成功步骤数
    pub success_count: usize,

    /// 失败步骤数
    pub failed_count: usize,

    /// 跳过步骤数
    pub skipped_count: usize,

    /// 步骤结果列表
    pub steps: Vec<StepResult>,
}

impl TestReport {
    /// 创建新的测试报告
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            start_time: Utc::now(),
            end_time: None,
            duration: Duration::from_secs(0),
            total_steps: 0,
            success_count: 0,
            failed_count: 0,
            skipped_count: 0,
            steps: Vec::new(),
        }
    }

    /// 添加步骤结果
    pub fn add_step_result(&mut self, result: StepResult) {
        match result.status {
            StepStatus::Success => self.success_count += 1,
            StepStatus::Failed => self.failed_count += 1,
            StepStatus::Skipped => self.skipped_count += 1,
        }
        self.total_steps += 1;
        self.steps.push(result);
    }

    /// 完成报告
    pub fn finalize(&mut self) {
        self.end_time = Some(Utc::now());
        if let Some(end_time) = self.end_time {
            self.duration = (end_time - self.start_time)
                .to_std()
                .unwrap_or(Duration::from_secs(0));
        }
    }

    /// 测试是否成功
    pub fn is_success(&self) -> bool {
        self.failed_count == 0 && self.total_steps > 0
    }

    /// 导出为 JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 导出为 YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

/// 步骤结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// 步骤索引
    pub step_index: usize,

    /// 步骤描述
    pub description: String,

    /// 步骤状态
    pub status: StepStatus,

    /// 错误信息
    pub error: Option<String>,

    /// 耗时
    #[serde(skip)]
    pub duration: Duration,

    /// 输出
    pub output: Option<String>,
}

impl StepResult {
    /// 创建成功的步骤结果
    pub fn success(step_index: usize, description: &str) -> Self {
        Self {
            step_index,
            description: description.to_string(),
            status: StepStatus::Success,
            error: None,
            duration: Duration::from_secs(0),
            output: None,
        }
    }

    /// 创建失败的步骤结果
    pub fn failed(step_index: usize, description: &str, error: &str) -> Self {
        Self {
            step_index,
            description: description.to_string(),
            status: StepStatus::Failed,
            error: Some(error.to_string()),
            duration: Duration::from_secs(0),
            output: None,
        }
    }

    /// 创建跳过的步骤结果
    pub fn skipped(step_index: usize, description: &str) -> Self {
        Self {
            step_index,
            description: description.to_string(),
            status: StepStatus::Skipped,
            error: None,
            duration: Duration::from_secs(0),
            output: None,
        }
    }
}

/// 步骤状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// 成功
    Success,

    /// 失败
    Failed,

    /// 跳过
    Skipped,
}

//! Orchestrator 模块测试

use atp_orchestrator::*;
use std::time::Duration;

#[test]
fn test_orchestrator_error_variants() {
    let err1 = OrchestratorError::VdiError("vdi error".to_string());
    assert!(matches!(err1, OrchestratorError::VdiError(_)));

    let err2 = OrchestratorError::VirtualizationError("virt error".to_string());
    assert!(matches!(err2, OrchestratorError::VirtualizationError(_)));

    let err3 = OrchestratorError::ScenarioParseError("parse error".to_string());
    assert!(matches!(err3, OrchestratorError::ScenarioParseError(_)));

    let err4 = OrchestratorError::VerificationFailed("verification error".to_string());
    assert!(matches!(err4, OrchestratorError::VerificationFailed(_)));

    let err5 = OrchestratorError::Timeout("timeout".to_string());
    assert!(matches!(err5, OrchestratorError::Timeout(_)));

    let err6 = OrchestratorError::ResourceNotFound("resource".to_string());
    assert!(matches!(err6, OrchestratorError::ResourceNotFound(_)));

    let err7 = OrchestratorError::Unknown("unknown".to_string());
    assert!(matches!(err7, OrchestratorError::Unknown(_)));
}

#[test]
fn test_orchestrator_error_display() {
    let err = OrchestratorError::VdiError("connection failed".to_string());
    let err_str = format!("{}", err);
    assert!(err_str.contains("VDI 平台错误"));
    assert!(err_str.contains("connection failed"));

    let err = OrchestratorError::VerificationFailed("test failed".to_string());
    let err_str = format!("{}", err);
    assert!(err_str.contains("验证失败"));
    assert!(err_str.contains("test failed"));

    let err = OrchestratorError::Timeout("operation timeout".to_string());
    let err_str = format!("{}", err);
    assert!(err_str.contains("超时"));
}

#[test]
fn test_test_scenario_creation() {
    let scenario = TestScenario {
        name: "test-scenario".to_string(),
        description: Some("A test scenario".to_string()),
        steps: vec![],
        tags: vec!["test".to_string(), "integration".to_string()],
        timeout: Some(3600),
    };

    assert_eq!(scenario.name, "test-scenario");
    assert_eq!(scenario.description, Some("A test scenario".to_string()));
    assert_eq!(scenario.steps.len(), 0);
    assert_eq!(scenario.tags.len(), 2);
    assert_eq!(scenario.timeout, Some(3600));
}

#[test]
fn test_test_scenario_clone() {
    let original = TestScenario {
        name: "clone-test".to_string(),
        description: None,
        steps: vec![],
        tags: vec![],
        timeout: None,
    };

    let cloned = original.clone();
    assert_eq!(cloned.name, original.name);
    assert_eq!(cloned.description, original.description);
}

#[test]
fn test_step_result_success() {
    let result = StepResult::success(0, "test-step");

    assert_eq!(result.step_index, 0);
    assert_eq!(result.description, "test-step");
    assert_eq!(result.status, StepStatus::Success);
    assert!(result.error.is_none());
}

#[test]
fn test_step_result_failed() {
    let result = StepResult::failed(1, "failed-step", "Connection timeout");

    assert_eq!(result.step_index, 1);
    assert_eq!(result.description, "failed-step");
    assert_eq!(result.status, StepStatus::Failed);
    assert!(result.error.is_some());
    assert_eq!(result.error.unwrap(), "Connection timeout");
}

#[test]
fn test_step_result_skipped() {
    let result = StepResult::skipped(2, "skipped-step");

    assert_eq!(result.step_index, 2);
    assert_eq!(result.description, "skipped-step");
    assert_eq!(result.status, StepStatus::Skipped);
    assert!(result.error.is_none());
}

#[test]
fn test_step_status_equality() {
    assert_eq!(StepStatus::Success, StepStatus::Success);
    assert_eq!(StepStatus::Failed, StepStatus::Failed);
    assert_eq!(StepStatus::Skipped, StepStatus::Skipped);

    assert_ne!(StepStatus::Success, StepStatus::Failed);
    assert_ne!(StepStatus::Failed, StepStatus::Skipped);
}

#[test]
fn test_test_report_new() {
    let report = TestReport::new("test-scenario");

    assert_eq!(report.name, "test-scenario");
    assert!(report.description.is_none());
    assert!(report.end_time.is_none());
    assert_eq!(report.total_steps, 0);
    assert_eq!(report.success_count, 0);
    assert_eq!(report.failed_count, 0);
    assert_eq!(report.skipped_count, 0);
    assert_eq!(report.steps.len(), 0);
}

#[test]
fn test_test_report_add_step_result() {
    let mut report = TestReport::new("test-scenario");

    report.add_step_result(StepResult::success(0, "step1"));
    assert_eq!(report.total_steps, 1);
    assert_eq!(report.success_count, 1);
    assert_eq!(report.steps.len(), 1);

    report.add_step_result(StepResult::failed(1, "step2", "error"));
    assert_eq!(report.total_steps, 2);
    assert_eq!(report.failed_count, 1);
    assert_eq!(report.steps.len(), 2);

    report.add_step_result(StepResult::skipped(2, "step3"));
    assert_eq!(report.total_steps, 3);
    assert_eq!(report.skipped_count, 1);
    assert_eq!(report.steps.len(), 3);
}

#[test]
fn test_test_report_is_success() {
    let mut report = TestReport::new("test");

    // 空报告不算成功
    assert!(!report.is_success());

    // 只有成功步骤算成功
    report.add_step_result(StepResult::success(0, "step1"));
    assert!(report.is_success());

    // 有失败步骤不算成功
    report.add_step_result(StepResult::failed(1, "step2", "error"));
    assert!(!report.is_success());
}

#[test]
fn test_test_report_finalize() {
    let mut report = TestReport::new("test");

    assert!(report.end_time.is_none());

    report.finalize();

    assert!(report.end_time.is_some());
    assert!(report.duration > Duration::from_secs(0) || report.duration == Duration::from_secs(0));
}

#[test]
fn test_test_report_to_json() {
    let report = TestReport::new("test-scenario");
    let json = report.to_json().unwrap();

    assert!(!json.is_empty());
    assert!(json.contains("test-scenario"));
}

#[test]
fn test_test_report_to_yaml() {
    let report = TestReport::new("test-scenario");
    let yaml = report.to_yaml().unwrap();

    assert!(!yaml.is_empty());
    assert!(yaml.contains("test-scenario"));
}

#[test]
fn test_step_result_clone() {
    let original = StepResult::success(0, "test-step");
    let cloned = original.clone();

    assert_eq!(cloned.step_index, original.step_index);
    assert_eq!(cloned.description, original.description);
    assert_eq!(cloned.status, original.status);
}

#[test]
fn test_step_result_with_output() {
    let mut result = StepResult::success(0, "test-step");
    result.output = Some("command output".to_string());

    assert!(result.output.is_some());
    assert_eq!(result.output.unwrap(), "command output");
}

#[test]
fn test_step_result_with_duration() {
    let mut result = StepResult::success(0, "test-step");
    result.duration = Duration::from_secs(5);

    assert_eq!(result.duration, Duration::from_secs(5));
}

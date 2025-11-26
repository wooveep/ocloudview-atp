//! Executor 模块测试

use atp_executor::*;

#[test]
fn test_scenario_creation() {
    let scenario = Scenario {
        name: "test-scenario".to_string(),
        description: Some("A test scenario".to_string()),
        steps: vec![],
        tags: vec!["test".to_string()],
    };

    assert_eq!(scenario.name, "test-scenario");
    assert_eq!(scenario.description, Some("A test scenario".to_string()));
    assert_eq!(scenario.steps.len(), 0);
    assert_eq!(scenario.tags.len(), 1);
}

#[test]
fn test_scenario_step_creation() {
    let step = ScenarioStep {
        name: Some("send key".to_string()),
        action: Action::SendKey { key: "enter".to_string() },
        verify: true,
        timeout: Some(30),
    };

    assert!(step.name.is_some());
    assert!(matches!(step.action, Action::SendKey { .. }));
    assert_eq!(step.verify, true);
    assert_eq!(step.timeout, Some(30));
}

#[test]
fn test_action_variants() {
    let action1 = Action::SendKey { key: "a".to_string() };
    assert!(matches!(action1, Action::SendKey { .. }));

    let action2 = Action::SendText { text: "hello".to_string() };
    assert!(matches!(action2, Action::SendText { .. }));

    let action3 = Action::MouseClick { x: 100, y: 200, button: "left".to_string() };
    assert!(matches!(action3, Action::MouseClick { .. }));

    let action4 = Action::ExecCommand { command: "ls -la".to_string() };
    assert!(matches!(action4, Action::ExecCommand { .. }));

    let action5 = Action::Wait { duration: 5 };
    assert!(matches!(action5, Action::Wait { .. }));

    let action6 = Action::Custom { data: serde_json::json!({"key": "value"}) };
    assert!(matches!(action6, Action::Custom { .. }));
}

#[test]
fn test_scenario_json_serialization() {
    let scenario = Scenario {
        name: "json-test".to_string(),
        description: None,
        steps: vec![
            ScenarioStep {
                name: Some("step1".to_string()),
                action: Action::Wait { duration: 1 },
                verify: false,
                timeout: None,
            }
        ],
        tags: vec![],
    };

    let json = scenario.to_json().unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("json-test"));
    assert!(json.contains("wait"));

    // 反序列化
    let deserialized = Scenario::from_json_str(&json).unwrap();
    assert_eq!(deserialized.name, scenario.name);
    assert_eq!(deserialized.steps.len(), scenario.steps.len());
}

#[test]
fn test_scenario_yaml_serialization() {
    let scenario = Scenario {
        name: "yaml-test".to_string(),
        description: Some("YAML test scenario".to_string()),
        steps: vec![
            ScenarioStep {
                name: None,
                action: Action::SendText { text: "hello world".to_string() },
                verify: true,
                timeout: Some(10),
            }
        ],
        tags: vec!["yaml".to_string(), "test".to_string()],
    };

    let yaml = scenario.to_yaml().unwrap();
    assert!(!yaml.is_empty());
    assert!(yaml.contains("yaml-test"));

    // 反序列化
    let deserialized = Scenario::from_yaml_str(&yaml).unwrap();
    assert_eq!(deserialized.name, scenario.name);
    assert_eq!(deserialized.description, scenario.description);
    assert_eq!(deserialized.steps.len(), scenario.steps.len());
}

#[test]
fn test_scenario_complex_actions() {
    let scenario = Scenario {
        name: "complex-scenario".to_string(),
        description: None,
        steps: vec![
            ScenarioStep {
                name: Some("send key".to_string()),
                action: Action::SendKey { key: "ctrl-c".to_string() },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("send text".to_string()),
                action: Action::SendText { text: "test input".to_string() },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("mouse click".to_string()),
                action: Action::MouseClick { x: 500, y: 300, button: "right".to_string() },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("execute command".to_string()),
                action: Action::ExecCommand { command: "echo test".to_string() },
                verify: true,
                timeout: Some(5),
            },
            ScenarioStep {
                name: Some("wait".to_string()),
                action: Action::Wait { duration: 3 },
                verify: false,
                timeout: None,
            },
        ],
        tags: vec!["complex".to_string()],
    };

    assert_eq!(scenario.steps.len(), 5);

    // 验证每个步骤的类型
    assert!(matches!(scenario.steps[0].action, Action::SendKey { .. }));
    assert!(matches!(scenario.steps[1].action, Action::SendText { .. }));
    assert!(matches!(scenario.steps[2].action, Action::MouseClick { .. }));
    assert!(matches!(scenario.steps[3].action, Action::ExecCommand { .. }));
    assert!(matches!(scenario.steps[4].action, Action::Wait { .. }));

    // 验证只有 ExecCommand 步骤设置了 verify
    assert_eq!(scenario.steps[3].verify, true);
}

#[test]
fn test_executor_error_variants() {
    let err1 = ExecutorError::ScenarioLoadFailed("load error".to_string());
    assert!(matches!(err1, ExecutorError::ScenarioLoadFailed(_)));

    let err2 = ExecutorError::StepExecutionFailed("exec error".to_string());
    assert!(matches!(err2, ExecutorError::StepExecutionFailed(_)));

    let err3 = ExecutorError::Timeout;
    assert!(matches!(err3, ExecutorError::Timeout));

    let err4 = ExecutorError::SerdeError("serde error".to_string());
    assert!(matches!(err4, ExecutorError::SerdeError(_)));

    let err5 = ExecutorError::DatabaseError("db error".to_string());
    assert!(matches!(err5, ExecutorError::DatabaseError(_)));
}

#[test]
fn test_executor_error_display() {
    let err = ExecutorError::ScenarioLoadFailed("file not found".to_string());
    let err_str = format!("{}", err);
    assert!(err_str.contains("场景加载失败"));
    assert!(err_str.contains("file not found"));

    let err = ExecutorError::Timeout;
    let err_str = format!("{}", err);
    assert!(err_str.contains("超时"));
}

#[test]
fn test_scenario_clone() {
    let original = Scenario {
        name: "clone-test".to_string(),
        description: Some("test".to_string()),
        steps: vec![
            ScenarioStep {
                name: Some("step".to_string()),
                action: Action::Wait { duration: 1 },
                verify: false,
                timeout: None,
            }
        ],
        tags: vec!["tag1".to_string()],
    };

    let cloned = original.clone();
    assert_eq!(cloned.name, original.name);
    assert_eq!(cloned.description, original.description);
    assert_eq!(cloned.steps.len(), original.steps.len());
    assert_eq!(cloned.tags, original.tags);
}

#[test]
fn test_custom_action_data() {
    let custom_data = serde_json::json!({
        "operation": "screenshot",
        "filename": "test.png",
        "quality": 90
    });

    let action = Action::Custom { data: custom_data.clone() };

    if let Action::Custom { data } = action {
        assert_eq!(data["operation"], "screenshot");
        assert_eq!(data["filename"], "test.png");
        assert_eq!(data["quality"], 90);
    } else {
        panic!("Expected Custom action");
    }
}

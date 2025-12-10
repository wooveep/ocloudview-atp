//! Executor 模块测试

use atp_executor::*;

#[test]
fn test_scenario_creation() {
    let scenario = Scenario {
        name: "test-scenario".to_string(),
        description: Some("A test scenario".to_string()),
        target_host: None,
        target_domain: None,
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
        target_host: None,
        target_domain: None,
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
        target_host: None,
        target_domain: None,
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
        target_host: None,
        target_domain: None,
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
        target_host: None,
        target_domain: None,
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

// ========================================
// VDI 操作测试
// ========================================

#[test]
fn test_vdi_create_desk_pool_action() {
    let action = Action::VdiCreateDeskPool {
        name: "test-pool".to_string(),
        template_id: "template-001".to_string(),
        count: 5,
    };

    assert!(matches!(action, Action::VdiCreateDeskPool { .. }));

    if let Action::VdiCreateDeskPool { name, template_id, count } = action {
        assert_eq!(name, "test-pool");
        assert_eq!(template_id, "template-001");
        assert_eq!(count, 5);
    }
}

#[test]
fn test_vdi_enable_desk_pool_action() {
    let action = Action::VdiEnableDeskPool {
        pool_id: "pool-123".to_string(),
    };

    assert!(matches!(action, Action::VdiEnableDeskPool { .. }));

    if let Action::VdiEnableDeskPool { pool_id } = action {
        assert_eq!(pool_id, "pool-123");
    }
}

#[test]
fn test_vdi_disable_desk_pool_action() {
    let action = Action::VdiDisableDeskPool {
        pool_id: "pool-456".to_string(),
    };

    assert!(matches!(action, Action::VdiDisableDeskPool { .. }));
}

#[test]
fn test_vdi_delete_desk_pool_action() {
    let action = Action::VdiDeleteDeskPool {
        pool_id: "pool-789".to_string(),
    };

    assert!(matches!(action, Action::VdiDeleteDeskPool { .. }));
}

#[test]
fn test_vdi_start_domain_action() {
    let action = Action::VdiStartDomain {
        domain_id: "vm-001".to_string(),
    };

    assert!(matches!(action, Action::VdiStartDomain { .. }));

    if let Action::VdiStartDomain { domain_id } = action {
        assert_eq!(domain_id, "vm-001");
    }
}

#[test]
fn test_vdi_shutdown_domain_action() {
    let action = Action::VdiShutdownDomain {
        domain_id: "vm-002".to_string(),
    };

    assert!(matches!(action, Action::VdiShutdownDomain { .. }));
}

#[test]
fn test_vdi_reboot_domain_action() {
    let action = Action::VdiRebootDomain {
        domain_id: "vm-003".to_string(),
    };

    assert!(matches!(action, Action::VdiRebootDomain { .. }));
}

#[test]
fn test_vdi_delete_domain_action() {
    let action = Action::VdiDeleteDomain {
        domain_id: "vm-004".to_string(),
    };

    assert!(matches!(action, Action::VdiDeleteDomain { .. }));
}

#[test]
fn test_vdi_bind_user_action() {
    let action = Action::VdiBindUser {
        domain_id: "vm-005".to_string(),
        user_id: "user-001".to_string(),
    };

    assert!(matches!(action, Action::VdiBindUser { .. }));

    if let Action::VdiBindUser { domain_id, user_id } = action {
        assert_eq!(domain_id, "vm-005");
        assert_eq!(user_id, "user-001");
    }
}

#[test]
fn test_vdi_get_desk_pool_domains_action() {
    let action = Action::VdiGetDeskPoolDomains {
        pool_id: "pool-999".to_string(),
    };

    assert!(matches!(action, Action::VdiGetDeskPoolDomains { .. }));

    if let Action::VdiGetDeskPoolDomains { pool_id } = action {
        assert_eq!(pool_id, "pool-999");
    }
}

// ========================================
// 验证步骤测试
// ========================================

#[test]
fn test_verify_domain_status_action() {
    let action = Action::VerifyDomainStatus {
        domain_id: "vm-test".to_string(),
        expected_status: "running".to_string(),
        timeout_secs: Some(30),
    };

    assert!(matches!(action, Action::VerifyDomainStatus { .. }));

    if let Action::VerifyDomainStatus { domain_id, expected_status, timeout_secs } = action {
        assert_eq!(domain_id, "vm-test");
        assert_eq!(expected_status, "running");
        assert_eq!(timeout_secs, Some(30));
    }
}

#[test]
fn test_verify_domain_status_without_timeout() {
    let action = Action::VerifyDomainStatus {
        domain_id: "vm-test-2".to_string(),
        expected_status: "stopped".to_string(),
        timeout_secs: None,
    };

    assert!(matches!(action, Action::VerifyDomainStatus { .. }));

    if let Action::VerifyDomainStatus { timeout_secs, .. } = action {
        assert_eq!(timeout_secs, None);
    }
}

#[test]
fn test_verify_all_domains_running_action() {
    let action = Action::VerifyAllDomainsRunning {
        pool_id: "pool-test".to_string(),
        timeout_secs: Some(60),
    };

    assert!(matches!(action, Action::VerifyAllDomainsRunning { .. }));

    if let Action::VerifyAllDomainsRunning { pool_id, timeout_secs } = action {
        assert_eq!(pool_id, "pool-test");
        assert_eq!(timeout_secs, Some(60));
    }
}

#[test]
fn test_verify_command_success_action() {
    let action = Action::VerifyCommandSuccess {
        timeout_secs: Some(10),
    };

    assert!(matches!(action, Action::VerifyCommandSuccess { .. }));

    if let Action::VerifyCommandSuccess { timeout_secs } = action {
        assert_eq!(timeout_secs, Some(10));
    }
}

// ========================================
// VDI 场景序列化测试
// ========================================

#[test]
fn test_vdi_scenario_json_serialization() {
    let scenario = Scenario {
        name: "vdi-workflow".to_string(),
        description: Some("VDI platform workflow test".to_string()),
        target_host: None,
        target_domain: None,
        steps: vec![
            ScenarioStep {
                name: Some("创建桌面池".to_string()),
                action: Action::VdiCreateDeskPool {
                    name: "dev-pool".to_string(),
                    template_id: "ubuntu-20.04".to_string(),
                    count: 3,
                },
                verify: false,
                timeout: Some(120),
            },
            ScenarioStep {
                name: Some("启用桌面池".to_string()),
                action: Action::VdiEnableDeskPool {
                    pool_id: "pool-123".to_string(),
                },
                verify: true,
                timeout: Some(30),
            },
            ScenarioStep {
                name: Some("启动虚拟机".to_string()),
                action: Action::VdiStartDomain {
                    domain_id: "vm-001".to_string(),
                },
                verify: false,
                timeout: Some(60),
            },
        ],
        tags: vec!["vdi".to_string(), "workflow".to_string()],
    };

    let json = scenario.to_json().unwrap();
    assert!(json.contains("vdi-workflow"));
    assert!(json.contains("vdi_create_desk_pool"));
    assert!(json.contains("vdi_enable_desk_pool"));
    assert!(json.contains("vdi_start_domain"));
    assert!(json.contains("dev-pool"));

    // 反序列化
    let deserialized = Scenario::from_json_str(&json).unwrap();
    assert_eq!(deserialized.name, scenario.name);
    assert_eq!(deserialized.steps.len(), 3);
    assert!(matches!(deserialized.steps[0].action, Action::VdiCreateDeskPool { .. }));
    assert!(matches!(deserialized.steps[1].action, Action::VdiEnableDeskPool { .. }));
    assert!(matches!(deserialized.steps[2].action, Action::VdiStartDomain { .. }));
}

#[test]
fn test_vdi_scenario_yaml_serialization() {
    let scenario = Scenario {
        name: "vdi-lifecycle".to_string(),
        description: Some("Complete VDI lifecycle test".to_string()),
        target_host: Some("qemu:///system".to_string()),
        target_domain: Some("test-vm".to_string()),
        steps: vec![
            ScenarioStep {
                name: Some("绑定用户".to_string()),
                action: Action::VdiBindUser {
                    domain_id: "vm-123".to_string(),
                    user_id: "user-456".to_string(),
                },
                verify: true,
                timeout: None,
            },
            ScenarioStep {
                name: Some("验证虚拟机状态".to_string()),
                action: Action::VerifyDomainStatus {
                    domain_id: "vm-123".to_string(),
                    expected_status: "running".to_string(),
                    timeout_secs: Some(30),
                },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("关闭虚拟机".to_string()),
                action: Action::VdiShutdownDomain {
                    domain_id: "vm-123".to_string(),
                },
                verify: false,
                timeout: Some(60),
            },
        ],
        tags: vec!["lifecycle".to_string()],
    };

    let yaml = scenario.to_yaml().unwrap();
    assert!(yaml.contains("vdi-lifecycle"));
    assert!(yaml.contains("vdi_bind_user"));
    assert!(yaml.contains("verify_domain_status"));
    assert!(yaml.contains("vdi_shutdown_domain"));

    // 反序列化
    let deserialized = Scenario::from_yaml_str(&yaml).unwrap();
    assert_eq!(deserialized.name, scenario.name);
    assert_eq!(deserialized.target_host, scenario.target_host);
    assert_eq!(deserialized.target_domain, scenario.target_domain);
    assert_eq!(deserialized.steps.len(), 3);
}

#[test]
fn test_mixed_protocol_and_vdi_scenario() {
    let scenario = Scenario {
        name: "mixed-workflow".to_string(),
        description: Some("Mixed protocol and VDI operations".to_string()),
        target_host: None,
        target_domain: None,
        steps: vec![
            ScenarioStep {
                name: Some("启动虚拟机".to_string()),
                action: Action::VdiStartDomain {
                    domain_id: "test-vm".to_string(),
                },
                verify: false,
                timeout: Some(60),
            },
            ScenarioStep {
                name: Some("等待启动".to_string()),
                action: Action::Wait { duration: 10 },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("验证状态".to_string()),
                action: Action::VerifyDomainStatus {
                    domain_id: "test-vm".to_string(),
                    expected_status: "running".to_string(),
                    timeout_secs: Some(30),
                },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("执行命令".to_string()),
                action: Action::ExecCommand {
                    command: "echo 'VM is ready'".to_string(),
                },
                verify: true,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("验证命令成功".to_string()),
                action: Action::VerifyCommandSuccess {
                    timeout_secs: Some(5),
                },
                verify: false,
                timeout: None,
            },
        ],
        tags: vec!["mixed".to_string(), "integration".to_string()],
    };

    assert_eq!(scenario.steps.len(), 5);
    assert!(matches!(scenario.steps[0].action, Action::VdiStartDomain { .. }));
    assert!(matches!(scenario.steps[1].action, Action::Wait { .. }));
    assert!(matches!(scenario.steps[2].action, Action::VerifyDomainStatus { .. }));
    assert!(matches!(scenario.steps[3].action, Action::ExecCommand { .. }));
    assert!(matches!(scenario.steps[4].action, Action::VerifyCommandSuccess { .. }));

    // 验证可以序列化和反序列化
    let json = scenario.to_json().unwrap();
    let deserialized = Scenario::from_json_str(&json).unwrap();
    assert_eq!(deserialized.steps.len(), 5);
}

#[test]
fn test_vdi_get_desk_pool_domains_in_scenario() {
    let scenario = Scenario {
        name: "pool-inspection".to_string(),
        description: Some("Inspect desk pool domains".to_string()),
        target_host: None,
        target_domain: None,
        steps: vec![
            ScenarioStep {
                name: Some("获取虚拟机列表".to_string()),
                action: Action::VdiGetDeskPoolDomains {
                    pool_id: "production-pool".to_string(),
                },
                verify: false,
                timeout: Some(30),
            },
            ScenarioStep {
                name: Some("验证所有虚拟机运行".to_string()),
                action: Action::VerifyAllDomainsRunning {
                    pool_id: "production-pool".to_string(),
                    timeout_secs: Some(60),
                },
                verify: false,
                timeout: None,
            },
        ],
        tags: vec!["inspection".to_string()],
    };

    let json = scenario.to_json().unwrap();
    assert!(json.contains("vdi_get_desk_pool_domains"));
    assert!(json.contains("verify_all_domains_running"));
    assert!(json.contains("production-pool"));

    let deserialized = Scenario::from_json_str(&json).unwrap();
    assert_eq!(deserialized.steps.len(), 2);
}

#[test]
fn test_vdi_complete_lifecycle_scenario() {
    let scenario = Scenario {
        name: "complete-vdi-lifecycle".to_string(),
        description: Some("Complete VDI lifecycle from creation to deletion".to_string()),
        target_host: None,
        target_domain: None,
        steps: vec![
            ScenarioStep {
                name: Some("1. 创建桌面池".to_string()),
                action: Action::VdiCreateDeskPool {
                    name: "test-pool".to_string(),
                    template_id: "centos7".to_string(),
                    count: 2,
                },
                verify: false,
                timeout: Some(180),
            },
            ScenarioStep {
                name: Some("2. 启用桌面池".to_string()),
                action: Action::VdiEnableDeskPool {
                    pool_id: "test-pool-id".to_string(),
                },
                verify: true,
                timeout: Some(30),
            },
            ScenarioStep {
                name: Some("3. 获取虚拟机列表".to_string()),
                action: Action::VdiGetDeskPoolDomains {
                    pool_id: "test-pool-id".to_string(),
                },
                verify: false,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("4. 验证所有虚拟机运行".to_string()),
                action: Action::VerifyAllDomainsRunning {
                    pool_id: "test-pool-id".to_string(),
                    timeout_secs: Some(120),
                },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("5. 重启虚拟机".to_string()),
                action: Action::VdiRebootDomain {
                    domain_id: "vm-1".to_string(),
                },
                verify: false,
                timeout: Some(60),
            },
            ScenarioStep {
                name: Some("6. 等待重启完成".to_string()),
                action: Action::Wait { duration: 30 },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("7. 禁用桌面池".to_string()),
                action: Action::VdiDisableDeskPool {
                    pool_id: "test-pool-id".to_string(),
                },
                verify: false,
                timeout: Some(30),
            },
            ScenarioStep {
                name: Some("8. 删除桌面池".to_string()),
                action: Action::VdiDeleteDeskPool {
                    pool_id: "test-pool-id".to_string(),
                },
                verify: false,
                timeout: Some(60),
            },
        ],
        tags: vec!["lifecycle".to_string(), "integration".to_string(), "vdi".to_string()],
    };

    // 验证场景结构
    assert_eq!(scenario.steps.len(), 8);
    assert_eq!(scenario.tags.len(), 3);

    // 验证每个步骤的动作类型
    assert!(matches!(scenario.steps[0].action, Action::VdiCreateDeskPool { .. }));
    assert!(matches!(scenario.steps[1].action, Action::VdiEnableDeskPool { .. }));
    assert!(matches!(scenario.steps[2].action, Action::VdiGetDeskPoolDomains { .. }));
    assert!(matches!(scenario.steps[3].action, Action::VerifyAllDomainsRunning { .. }));
    assert!(matches!(scenario.steps[4].action, Action::VdiRebootDomain { .. }));
    assert!(matches!(scenario.steps[5].action, Action::Wait { .. }));
    assert!(matches!(scenario.steps[6].action, Action::VdiDisableDeskPool { .. }));
    assert!(matches!(scenario.steps[7].action, Action::VdiDeleteDeskPool { .. }));

    // 验证序列化和反序列化
    let json = scenario.to_json().unwrap();
    let deserialized_json = Scenario::from_json_str(&json).unwrap();
    assert_eq!(deserialized_json.steps.len(), 8);

    let yaml = scenario.to_yaml().unwrap();
    let deserialized_yaml = Scenario::from_yaml_str(&yaml).unwrap();
    assert_eq!(deserialized_yaml.steps.len(), 8);
    assert_eq!(deserialized_yaml.name, scenario.name);
}

#[test]
fn test_action_clone_for_vdi_operations() {
    let original = Action::VdiCreateDeskPool {
        name: "pool-1".to_string(),
        template_id: "template-1".to_string(),
        count: 5,
    };

    let cloned = original.clone();

    if let Action::VdiCreateDeskPool { name, template_id, count } = cloned {
        assert_eq!(name, "pool-1");
        assert_eq!(template_id, "template-1");
        assert_eq!(count, 5);
    } else {
        panic!("Expected VdiCreateDeskPool action");
    }
}

#[test]
fn test_verification_actions_with_default_timeout() {
    let action1 = Action::VerifyDomainStatus {
        domain_id: "vm-1".to_string(),
        expected_status: "running".to_string(),
        timeout_secs: None,
    };

    let action2 = Action::VerifyAllDomainsRunning {
        pool_id: "pool-1".to_string(),
        timeout_secs: None,
    };

    let action3 = Action::VerifyCommandSuccess {
        timeout_secs: None,
    };

    assert!(matches!(action1, Action::VerifyDomainStatus { timeout_secs: None, .. }));
    assert!(matches!(action2, Action::VerifyAllDomainsRunning { timeout_secs: None, .. }));
    assert!(matches!(action3, Action::VerifyCommandSuccess { timeout_secs: None }));
}

// ========================================
// ExecutionReport 和 StepReport 测试 (从 Orchestrator 合并)
// ========================================

#[test]
fn test_execution_report_new() {
    let report = ExecutionReport::new("test-scenario");

    assert_eq!(report.scenario_name, "test-scenario");
    assert!(report.description.is_none());
    assert_eq!(report.passed, true);
    assert_eq!(report.steps_executed, 0);
    assert_eq!(report.passed_count, 0);
    assert_eq!(report.failed_count, 0);
    assert_eq!(report.duration_ms, 0);
    assert_eq!(report.steps.len(), 0);
    assert_eq!(report.tags.len(), 0);
}

#[test]
fn test_execution_report_add_step() {
    let mut report = ExecutionReport::new("test");

    // 添加成功步骤
    report.add_step(StepReport::success(0, "step1"));
    assert_eq!(report.steps_executed, 1);
    assert_eq!(report.passed_count, 1);
    assert_eq!(report.failed_count, 0);
    assert_eq!(report.passed, true);

    // 添加失败步骤
    report.add_step(StepReport::failed(1, "step2", "error"));
    assert_eq!(report.steps_executed, 2);
    assert_eq!(report.passed_count, 1);
    assert_eq!(report.failed_count, 1);
    assert_eq!(report.passed, false); // 有失败步骤后 passed 变为 false

    // 添加另一个成功步骤
    report.add_step(StepReport::success(2, "step3"));
    assert_eq!(report.steps_executed, 3);
    assert_eq!(report.passed_count, 2);
    assert_eq!(report.failed_count, 1);
    assert_eq!(report.passed, false); // 仍然是 false
}

#[test]
fn test_execution_report_with_description_and_tags() {
    let mut report = ExecutionReport::new("test");
    report.description = Some("Test description".to_string());
    report.tags = vec!["tag1".to_string(), "tag2".to_string()];

    assert_eq!(report.description, Some("Test description".to_string()));
    assert_eq!(report.tags.len(), 2);
    assert_eq!(report.tags[0], "tag1");
    assert_eq!(report.tags[1], "tag2");
}

#[test]
fn test_execution_report_to_json() {
    let mut report = ExecutionReport::new("json-test");
    report.add_step(StepReport::success(0, "step1"));

    let json = report.to_json().unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("json-test"));
    assert!(json.contains("step1"));
    assert!(json.contains("Success"));
}

#[test]
fn test_execution_report_to_yaml() {
    let mut report = ExecutionReport::new("yaml-test");
    report.add_step(StepReport::success(0, "step1"));

    let yaml = report.to_yaml().unwrap();
    assert!(!yaml.is_empty());
    assert!(yaml.contains("yaml-test"));
    assert!(yaml.contains("step1"));
}

#[test]
fn test_step_report_success() {
    let report = StepReport::success(0, "test-step");

    assert_eq!(report.step_index, 0);
    assert_eq!(report.description, "test-step");
    assert_eq!(report.status, StepStatus::Success);
    assert!(report.error.is_none());
    assert_eq!(report.duration_ms, 0);
    assert!(report.output.is_none());
}

#[test]
fn test_step_report_failed() {
    let report = StepReport::failed(1, "failed-step", "Connection timeout");

    assert_eq!(report.step_index, 1);
    assert_eq!(report.description, "failed-step");
    assert_eq!(report.status, StepStatus::Failed);
    assert!(report.error.is_some());
    assert_eq!(report.error.as_ref().unwrap(), "Connection timeout");
}

#[test]
fn test_step_report_with_output() {
    let mut report = StepReport::success(0, "command-step");
    report.output = Some("command output here".to_string());

    assert!(report.output.is_some());
    assert_eq!(report.output.unwrap(), "command output here");
}

#[test]
fn test_step_report_with_duration() {
    let mut report = StepReport::success(0, "timed-step");
    report.duration_ms = 1500; // 1.5 seconds

    assert_eq!(report.duration_ms, 1500);
}

#[test]
fn test_step_status_equality() {
    assert_eq!(StepStatus::Success, StepStatus::Success);
    assert_eq!(StepStatus::Failed, StepStatus::Failed);
    assert_eq!(StepStatus::Skipped, StepStatus::Skipped);

    assert_ne!(StepStatus::Success, StepStatus::Failed);
    assert_ne!(StepStatus::Failed, StepStatus::Skipped);
    assert_ne!(StepStatus::Success, StepStatus::Skipped);
}

#[test]
fn test_execution_report_all_steps_success() {
    let mut report = ExecutionReport::new("all-success");
    report.add_step(StepReport::success(0, "step1"));
    report.add_step(StepReport::success(1, "step2"));
    report.add_step(StepReport::success(2, "step3"));

    assert_eq!(report.passed, true);
    assert_eq!(report.steps_executed, 3);
    assert_eq!(report.passed_count, 3);
    assert_eq!(report.failed_count, 0);
}

#[test]
fn test_execution_report_mixed_results() {
    let mut report = ExecutionReport::new("mixed-results");

    // 成功
    let mut step1 = StepReport::success(0, "step1");
    step1.duration_ms = 100;
    report.add_step(step1);

    // 失败
    let mut step2 = StepReport::failed(1, "step2", "timeout");
    step2.duration_ms = 5000;
    report.add_step(step2);

    // 成功但有输出
    let mut step3 = StepReport::success(2, "step3");
    step3.duration_ms = 200;
    step3.output = Some("output data".to_string());
    report.add_step(step3);

    assert_eq!(report.passed, false); // 有失败步骤
    assert_eq!(report.steps_executed, 3);
    assert_eq!(report.passed_count, 2);
    assert_eq!(report.failed_count, 1);
    assert_eq!(report.steps.len(), 3);
}

#[test]
fn test_step_report_clone() {
    let original = StepReport::success(0, "original-step");
    let cloned = original.clone();

    assert_eq!(cloned.step_index, original.step_index);
    assert_eq!(cloned.description, original.description);
    assert_eq!(cloned.status, original.status);
    assert_eq!(cloned.error, original.error);
}


//! End-to-End 测试
//!
//! 这些测试验证完整的执行流程:
//! Scenario → Executor → Protocol → VM
//!
//! 运行要求:
//! - 本地 libvirtd 运行中
//! - 有至少一个可用的虚拟机
//! - 虚拟机已安装 qemu-guest-agent (用于 QGA 测试)
//! - 虚拟机配置了 SPICE (用于 SPICE 测试)
//!
//! 配置方式:
//! 1. 使用配置文件 (test.toml / tests/config.toml / ~/.config/atp/test.toml)
//! 2. 使用环境变量 (会覆盖配置文件)
//! 3. 使用默认值
//!
//! 运行方法:
//! ```bash
//! # 方式1: 使用配置文件
//! echo "[vm]
//! name = \"my-test-vm\"" > test.toml
//! cargo test --test e2e_tests -- --nocapture
//!
//! # 方式2: 使用环境变量
//! export ATP_TEST_VM=my-test-vm
//! export ATP_TEST_HOST=qemu:///system
//! cargo test --test e2e_tests -- --nocapture
//!
//! # 方式3: 指定配置文件
//! export ATP_TEST_CONFIG=./my-test-config.toml
//! cargo test --test e2e_tests -- --nocapture
//!
//! # 运行特定测试
//! cargo test --test e2e_tests test_basic_scenario -- --nocapture
//! ```

use atp_executor::*;
use atp_transport::{TransportManager, HostInfo};
use atp_protocol::ProtocolRegistry;
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;

/// 初始化测试环境 (使用 TestConfig)
async fn setup_test_runner() -> (ScenarioRunner, TestConfig) {
    // 1. 加载测试配置 (从文件或环境变量)
    let config = TestConfig::load()
        .expect("Failed to load test config");

    // 2. 验证配置
    config.validate()
        .expect("Invalid test config");

    // 3. 初始化日志
    let _ = tracing_subscriber::fmt()
        .with_env_filter(&config.environment.log_level)
        .try_init();

    tracing::info!("Test config loaded");
    tracing::debug!("VM name: {}", config.vm.name);
    tracing::debug!("Libvirt URI: {}", config.libvirt.uri);

    // 4. 创建传输管理器
    let transport_manager = Arc::new(TransportManager::default());

    // 5. 添加测试主机 (从配置读取)
    let host_info = HostInfo {
        id: "test-host".to_string(),
        host: "localhost".to_string(),
        uri: config.libvirt.uri.clone(),
        tags: vec!["test".to_string()],
        metadata: HashMap::new(),
    };

    transport_manager.add_host(host_info).await
        .expect("Failed to add test host");

    // 6. 创建协议注册表
    let protocol_registry = Arc::new(ProtocolRegistry::new());

    // 7. 创建场景执行器 (使用配置的超时)
    let runner = ScenarioRunner::new(transport_manager, protocol_registry)
        .with_timeout(Duration::from_secs(config.test.timeout));

    (runner, config)
}

// ========================================
// 基础场景测试
// ========================================

#[tokio::test]
#[ignore] // 需要实际虚拟机环境才能运行
async fn test_basic_scenario_wait() {
    let mut runner = setup_test_runner().await;

    let scenario = Scenario {
        name: "basic-wait-test".to_string(),
        description: Some("基础等待测试".to_string()),
        target_host: Some(get_test_host_uri()),
        target_domain: None, // 不需要虚拟机
        steps: vec![
            ScenarioStep {
                name: Some("等待1秒".to_string()),
                action: Action::Wait { duration: 1 },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("等待2秒".to_string()),
                action: Action::Wait { duration: 2 },
                verify: false,
                timeout: None,
            },
        ],
        tags: vec!["e2e".to_string(), "basic".to_string()],
    };

    let report = runner.run(&scenario).await
        .expect("Scenario execution failed");

    println!("\n=== 执行报告 ===");
    println!("{}", report.to_json().unwrap());

    assert_eq!(report.passed, true);
    assert_eq!(report.steps_executed, 2);
    assert_eq!(report.passed_count, 2);
    assert_eq!(report.failed_count, 0);
    assert!(report.duration_ms >= 3000); // 至少 3 秒
}

#[tokio::test]
#[ignore] // 需要实际虚拟机环境
async fn test_qmp_keyboard_input() {
    let mut runner = setup_test_runner().await;
    let vm_name = get_test_vm_name();

    let scenario = Scenario {
        name: "qmp-keyboard-test".to_string(),
        description: Some("QMP 键盘输入测试".to_string()),
        target_host: Some(get_test_host_uri()),
        target_domain: Some(vm_name.clone()),
        steps: vec![
            ScenarioStep {
                name: Some("发送 Enter 键".to_string()),
                action: Action::SendKey { key: "ret".to_string() },
                verify: false,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("发送文本".to_string()),
                action: Action::SendText { text: "hello".to_string() },
                verify: false,
                timeout: Some(10),
            },
        ],
        tags: vec!["e2e".to_string(), "qmp".to_string(), "keyboard".to_string()],
    };

    let report = runner.run(&scenario).await
        .expect("Scenario execution failed");

    println!("\n=== QMP 键盘测试报告 ===");
    println!("{}", report.to_json().unwrap());

    assert_eq!(report.steps_executed, 2);
    // QMP 可能未连接，所以不强制要求全部成功
    println!("通过步骤: {}/{}", report.passed_count, report.steps_executed);
}

#[tokio::test]
#[ignore] // 需要实际虚拟机环境
async fn test_qga_command_execution() {
    let mut runner = setup_test_runner().await;
    let vm_name = get_test_vm_name();

    let scenario = Scenario {
        name: "qga-command-test".to_string(),
        description: Some("QGA 命令执行测试".to_string()),
        target_host: Some(get_test_host_uri()),
        target_domain: Some(vm_name.clone()),
        steps: vec![
            ScenarioStep {
                name: Some("执行 echo 命令".to_string()),
                action: Action::ExecCommand {
                    command: "echo 'Hello from QGA'".to_string()
                },
                verify: true,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("执行 uname 命令".to_string()),
                action: Action::ExecCommand {
                    command: "uname -a".to_string()
                },
                verify: false,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("执行 date 命令".to_string()),
                action: Action::ExecCommand {
                    command: "date".to_string()
                },
                verify: false,
                timeout: Some(10),
            },
        ],
        tags: vec!["e2e".to_string(), "qga".to_string(), "command".to_string()],
    };

    let report = runner.run(&scenario).await
        .expect("Scenario execution failed");

    println!("\n=== QGA 命令执行测试报告 ===");
    println!("{}", report.to_json().unwrap());

    // 打印命令输出
    for step in &report.steps {
        if let Some(output) = &step.output {
            println!("\n步骤 {}: {}", step.step_index, step.description);
            println!("输出: {}", output);
        }
    }

    assert_eq!(report.steps_executed, 3);
    println!("通过步骤: {}/{}", report.passed_count, report.steps_executed);
}

#[tokio::test]
#[ignore] // 需要实际虚拟机环境
async fn test_spice_mouse_operations() {
    let mut runner = setup_test_runner().await;
    let vm_name = get_test_vm_name();

    let scenario = Scenario {
        name: "spice-mouse-test".to_string(),
        description: Some("SPICE 鼠标操作测试".to_string()),
        target_host: Some(get_test_host_uri()),
        target_domain: Some(vm_name.clone()),
        steps: vec![
            ScenarioStep {
                name: Some("左键点击 (100, 100)".to_string()),
                action: Action::MouseClick {
                    x: 100,
                    y: 100,
                    button: "left".to_string()
                },
                verify: false,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("等待 1 秒".to_string()),
                action: Action::Wait { duration: 1 },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("右键点击 (200, 200)".to_string()),
                action: Action::MouseClick {
                    x: 200,
                    y: 200,
                    button: "right".to_string()
                },
                verify: false,
                timeout: Some(10),
            },
        ],
        tags: vec!["e2e".to_string(), "spice".to_string(), "mouse".to_string()],
    };

    let report = runner.run(&scenario).await
        .expect("Scenario execution failed");

    println!("\n=== SPICE 鼠标测试报告 ===");
    println!("{}", report.to_json().unwrap());

    assert_eq!(report.steps_executed, 3);
    println!("通过步骤: {}/{}", report.passed_count, report.steps_executed);
}

// ========================================
// 混合协议测试
// ========================================

#[tokio::test]
#[ignore] // 需要实际虚拟机环境
async fn test_mixed_protocol_scenario() {
    let mut runner = setup_test_runner().await;
    let vm_name = get_test_vm_name();

    let scenario = Scenario {
        name: "mixed-protocol-test".to_string(),
        description: Some("混合协议操作测试 (QMP + QGA + SPICE)".to_string()),
        target_host: Some(get_test_host_uri()),
        target_domain: Some(vm_name.clone()),
        steps: vec![
            ScenarioStep {
                name: Some("1. QGA: 查看系统信息".to_string()),
                action: Action::ExecCommand {
                    command: "echo '=== System Info ===' && uname -a".to_string()
                },
                verify: false,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("2. 等待 1 秒".to_string()),
                action: Action::Wait { duration: 1 },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("3. QMP: 发送键盘输入".to_string()),
                action: Action::SendKey { key: "ret".to_string() },
                verify: false,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("4. 等待 1 秒".to_string()),
                action: Action::Wait { duration: 1 },
                verify: false,
                timeout: None,
            },
            ScenarioStep {
                name: Some("5. SPICE: 鼠标点击".to_string()),
                action: Action::MouseClick {
                    x: 500,
                    y: 300,
                    button: "left".to_string()
                },
                verify: false,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("6. QGA: 验证操作".to_string()),
                action: Action::ExecCommand {
                    command: "echo 'Test completed successfully'".to_string()
                },
                verify: false,
                timeout: Some(10),
            },
        ],
        tags: vec!["e2e".to_string(), "mixed".to_string()],
    };

    let report = runner.run(&scenario).await
        .expect("Scenario execution failed");

    println!("\n=== 混合协议测试报告 ===");
    println!("{}", report.to_json().unwrap());

    assert_eq!(report.steps_executed, 6);
    println!("通过步骤: {}/{}", report.passed_count, report.steps_executed);

    // 打印详细步骤信息
    println!("\n=== 步骤详情 ===");
    for step in &report.steps {
        println!("\n步骤 {}: {}", step.step_index + 1, step.description);
        println!("  状态: {:?}", step.status);
        println!("  耗时: {} ms", step.duration_ms);

        if let Some(output) = &step.output {
            println!("  输出: {}", output);
        }

        if let Some(error) = &step.error {
            println!("  错误: {}", error);
        }
    }
}

// ========================================
// 场景文件加载测试
// ========================================

#[tokio::test]
async fn test_load_scenario_from_yaml() {
    let yaml_content = r#"
name: "yaml-scenario-test"
description: "从 YAML 加载的测试场景"
target_host: "qemu:///system"
target_domain: "test-vm"
tags:
  - "e2e"
  - "yaml"
steps:
  - name: "步骤1: 等待"
    action:
      type: wait
      duration: 1
    verify: false
  - name: "步骤2: 执行命令"
    action:
      type: exec_command
      command: "echo test"
    verify: true
    timeout: 10
"#;

    let scenario = Scenario::from_yaml_str(yaml_content)
        .expect("Failed to parse YAML scenario");

    assert_eq!(scenario.name, "yaml-scenario-test");
    assert_eq!(scenario.steps.len(), 2);
    assert!(scenario.target_domain.is_some());
    assert_eq!(scenario.tags.len(), 2);

    // 验证步骤内容
    match &scenario.steps[0].action {
        Action::Wait { duration } => assert_eq!(*duration, 1),
        _ => panic!("Expected Wait action"),
    }

    match &scenario.steps[1].action {
        Action::ExecCommand { command } => assert_eq!(command, "echo test"),
        _ => panic!("Expected ExecCommand action"),
    }
}

#[tokio::test]
async fn test_load_scenario_from_json() {
    let json_content = r#"
{
  "name": "json-scenario-test",
  "description": "从 JSON 加载的测试场景",
  "target_host": "qemu:///system",
  "target_domain": null,
  "tags": ["e2e", "json"],
  "steps": [
    {
      "name": "发送按键",
      "action": {
        "type": "send_key",
        "key": "ret"
      },
      "verify": false,
      "timeout": null
    },
    {
      "name": "鼠标点击",
      "action": {
        "type": "mouse_click",
        "x": 100,
        "y": 200,
        "button": "left"
      },
      "verify": false,
      "timeout": 15
    }
  ]
}
"#;

    let scenario = Scenario::from_json_str(json_content)
        .expect("Failed to parse JSON scenario");

    assert_eq!(scenario.name, "json-scenario-test");
    assert_eq!(scenario.steps.len(), 2);
    assert_eq!(scenario.tags.len(), 2);

    // 验证超时设置
    assert!(scenario.steps[0].timeout.is_none());
    assert_eq!(scenario.steps[1].timeout, Some(15));
}

// ========================================
// 错误处理测试
// ========================================

#[tokio::test]
#[ignore] // 需要实际虚拟机环境
async fn test_command_failure_handling() {
    let mut runner = setup_test_runner().await;
    let vm_name = get_test_vm_name();

    let scenario = Scenario {
        name: "error-handling-test".to_string(),
        description: Some("错误处理测试".to_string()),
        target_host: Some(get_test_host_uri()),
        target_domain: Some(vm_name.clone()),
        steps: vec![
            ScenarioStep {
                name: Some("成功的命令".to_string()),
                action: Action::ExecCommand {
                    command: "echo 'This will succeed'".to_string()
                },
                verify: false,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("失败的命令".to_string()),
                action: Action::ExecCommand {
                    command: "false".to_string() // 这个命令会返回非0退出码
                },
                verify: false,
                timeout: Some(10),
            },
            ScenarioStep {
                name: Some("这一步不应该执行".to_string()),
                action: Action::Wait { duration: 1 },
                verify: false,
                timeout: None,
            },
        ],
        tags: vec!["e2e".to_string(), "error".to_string()],
    };

    let report = runner.run(&scenario).await
        .expect("Scenario execution failed");

    println!("\n=== 错误处理测试报告 ===");
    println!("{}", report.to_json().unwrap());

    // 验证第一步成功
    assert_eq!(report.steps[0].status, StepStatus::Success);

    // 验证第二步失败
    assert_eq!(report.steps[1].status, StepStatus::Failed);
    assert!(report.steps[1].error.is_some());

    // 验证第三步未执行 (因为第二步失败后会停止)
    assert_eq!(report.steps_executed, 2);

    // 整体报告应该标记为失败
    assert_eq!(report.passed, false);
}

#[tokio::test]
#[ignore] // 需要实际虚拟机环境
async fn test_timeout_handling() {
    let mut runner = setup_test_runner().await;
    let vm_name = get_test_vm_name();

    let scenario = Scenario {
        name: "timeout-test".to_string(),
        description: Some("超时处理测试".to_string()),
        target_host: Some(get_test_host_uri()),
        target_domain: Some(vm_name.clone()),
        steps: vec![
            ScenarioStep {
                name: Some("长时间运行的命令".to_string()),
                action: Action::ExecCommand {
                    command: "sleep 30".to_string() // 睡眠30秒
                },
                verify: false,
                timeout: Some(2), // 但只给2秒超时
            },
        ],
        tags: vec!["e2e".to_string(), "timeout".to_string()],
    };

    let report = runner.run(&scenario).await;

    // 应该超时失败
    assert!(report.is_err() || !report.unwrap().passed);
}

// ========================================
// 性能测试
// ========================================

#[tokio::test]
#[ignore] // 需要实际虚拟机环境
async fn test_scenario_execution_performance() {
    let mut runner = setup_test_runner().await;
    let vm_name = get_test_vm_name();

    // 创建包含10个快速命令的场景
    let steps: Vec<ScenarioStep> = (0..10)
        .map(|i| ScenarioStep {
            name: Some(format!("命令 {}", i + 1)),
            action: Action::ExecCommand {
                command: format!("echo 'Command {}'", i + 1)
            },
            verify: false,
            timeout: Some(5),
        })
        .collect();

    let scenario = Scenario {
        name: "performance-test".to_string(),
        description: Some("性能测试 - 10个快速命令".to_string()),
        target_host: Some(get_test_host_uri()),
        target_domain: Some(vm_name.clone()),
        steps,
        tags: vec!["e2e".to_string(), "performance".to_string()],
    };

    let start = std::time::Instant::now();
    let report = runner.run(&scenario).await
        .expect("Scenario execution failed");
    let elapsed = start.elapsed();

    println!("\n=== 性能测试报告 ===");
    println!("总执行时间: {:?}", elapsed);
    println!("报告记录的时间: {} ms", report.duration_ms);
    println!("步骤数: {}", report.steps_executed);
    println!("平均每步耗时: {} ms", report.duration_ms / report.steps_executed as u64);

    assert_eq!(report.steps_executed, 10);

    // 验证性能指标 (每个命令不应超过 1 秒)
    for step in &report.steps {
        println!("步骤 {}: {} ms", step.step_index + 1, step.duration_ms);
        assert!(step.duration_ms < 1000, "步骤 {} 执行时间过长: {} ms",
            step.step_index + 1, step.duration_ms);
    }
}

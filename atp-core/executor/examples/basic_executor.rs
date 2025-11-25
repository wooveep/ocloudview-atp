//! 基础场景执行示例
//!
//! 演示如何使用 ATP Executor 加载和执行测试场景

use std::sync::Arc;
use atp_executor::{Scenario, ScenarioRunner};
use atp_transport::{TransportManager, TransportConfig};
use atp_protocol::ProtocolRegistry;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 1. 加载测试场景
    let scenario_path = "examples/scenarios/basic-keyboard-test.yaml";
    println!("加载场景: {}", scenario_path);

    let scenario = Scenario::from_yaml_file(scenario_path)?;
    println!("场景名称: {}", scenario.name);
    println!("场景描述: {:?}", scenario.description);
    println!("步骤数量: {}", scenario.steps.len());
    println!();

    // 2. 创建传输管理器和协议注册表
    let transport_config = TransportConfig::default();
    let transport_manager = Arc::new(TransportManager::new(transport_config));

    let protocol_registry = Arc::new(ProtocolRegistry::new());

    // 3. 创建场景执行器
    let mut runner = ScenarioRunner::new(
        Arc::clone(&transport_manager),
        Arc::clone(&protocol_registry),
    );

    // 4. 执行场景
    println!("开始执行场景...");
    println!("========================================");

    let report = runner.run(&scenario).await?;

    // 5. 打印执行报告
    println!("========================================");
    println!("执行完成!");
    println!();
    println!("场景: {}", report.scenario_name);
    println!("状态: {}", if report.passed { "✓ 通过" } else { "✗ 失败" });
    println!("总步骤: {}", report.steps_executed);
    println!("成功步骤: {}", report.passed_count);
    println!("失败步骤: {}", report.failed_count);
    println!("总耗时: {} ms", report.duration_ms);
    println!();

    // 6. 打印每个步骤的详情
    println!("步骤详情:");
    println!("----------------------------------------");
    for step in &report.steps {
        let status_icon = match step.status {
            atp_executor::StepStatus::Success => "✓",
            atp_executor::StepStatus::Failed => "✗",
            atp_executor::StepStatus::Skipped => "○",
        };
        println!(
            "{} [步骤 {}] {} ({} ms)",
            status_icon,
            step.step_index + 1,
            step.description,
            step.duration_ms
        );
        if let Some(error) = &step.error {
            println!("   错误: {}", error);
        }
    }
    println!();

    // 7. 导出报告为 JSON
    let json_report = report.to_json()?;
    std::fs::write("execution_report.json", json_report)?;
    println!("报告已保存到: execution_report.json");

    Ok(())
}

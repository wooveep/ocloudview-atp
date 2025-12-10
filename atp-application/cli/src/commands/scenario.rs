//! Scenario 命令处理

use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use atp_executor::{Scenario, ScenarioRunner};
use atp_transport::{TransportManager, TransportConfig, HostInfo};
use atp_protocol::ProtocolRegistry;
use atp_storage::{StorageManager, Storage};

use crate::config::CliConfig;

pub async fn handle(action: crate::ScenarioAction) -> Result<()> {
    match action {
        crate::ScenarioAction::Run { file } => run_scenario(&file).await,
        crate::ScenarioAction::List => list_scenarios().await,
    }
}

async fn run_scenario(file: &str) -> Result<()> {
    let path = Path::new(file);

    // 加载场景
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    spinner.set_message(format!("加载场景: {}", file));
    spinner.enable_steady_tick(Duration::from_millis(100));

    let scenario = if path.extension().and_then(|s| s.to_str()) == Some("yaml") ||
                      path.extension().and_then(|s| s.to_str()) == Some("yml") {
        Scenario::from_yaml_file(path)?
    } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
        Scenario::from_json_file(path)?
    } else {
        anyhow::bail!("不支持的场景文件格式，仅支持 .yaml/.yml 或 .json");
    };

    spinner.finish_with_message(format!("{} 场景加载成功: {}", "✓".green().bold(), scenario.name.cyan()));

    // 显示场景信息
    println!();
    if let Some(desc) = &scenario.description {
        println!("描述: {}", desc.bright_black());
    }
    println!("步骤数: {}", scenario.steps.len().to_string().yellow());
    if !scenario.tags.is_empty() {
        println!("标签: {}", scenario.tags.join(", ").bright_black());
    }
    println!();

    // 初始化传输管理器和协议注册表
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    spinner.set_message("初始化传输管理器...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    // 加载配置以获取主机信息
    let config = CliConfig::load()?;

    // 创建传输管理器
    let transport_config = TransportConfig::default();
    let mut transport_manager = TransportManager::new(transport_config);

    // 添加配置的主机（这里简化处理，实际场景中应该根据需要选择主机）
    // 暂时添加所有配置的主机
    for (id, host_config) in config.hosts.iter() {
        let uri = host_config.uri.clone()
            .unwrap_or_else(|| format!("qemu+ssh://{}:22/system", host_config.host));

        let host_info = HostInfo::new(id, &host_config.host).with_uri(&uri);
        transport_manager.add_host(host_info).await
            .with_context(|| format!("添加主机 {} 失败", id))?;
    }

    let transport_manager = Arc::new(transport_manager);
    let protocol_registry = Arc::new(ProtocolRegistry::new());

    spinner.finish_with_message(format!("{} 传输管理器初始化完成", "✓".green().bold()));

    // 初始化数据库存储
    let storage_manager = StorageManager::new("~/.config/atp/data.db").await
        .context("初始化数据库失败")?;
    let storage = Arc::new(Storage::from_manager(&storage_manager));

    // 创建场景执行器 (with数据库支持)
    let mut runner = ScenarioRunner::new(
        Arc::clone(&transport_manager),
        Arc::clone(&protocol_registry),
    ).with_storage(Arc::clone(&storage));

    // 执行场景
    println!("\n{}\n", "开始执行场景...".bold());

    let progress = ProgressBar::new(scenario.steps.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-")
    );

    let report = runner.run(&scenario).await?;

    progress.finish_with_message("完成".green().to_string());

    // 显示执行报告
    println!("\n{}", "=".repeat(60));
    println!("{}", "执行报告".bold());
    println!("{}", "=".repeat(60));
    println!();

    println!("场景名称: {}", report.scenario_name.cyan().bold());
    if let Some(desc) = &report.description {
        println!("场景描述: {}", desc.bright_black());
    }
    println!("执行时间: {} ms", report.duration_ms.to_string().yellow());
    println!();

    println!("步骤统计:");
    println!("  总步骤: {}", report.steps_executed.to_string().bright_blue());
    println!("  成功:   {}", report.passed_count.to_string().green());
    println!("  失败:   {}", report.failed_count.to_string().red());
    println!();

    // 显示步骤详情
    if !report.steps.is_empty() {
        println!("步骤详情:");
        println!();

        for step in &report.steps {
            let status_icon = match step.status {
                atp_executor::StepStatus::Success => "✓".green(),
                atp_executor::StepStatus::Failed => "✗".red(),
                atp_executor::StepStatus::Skipped => "⊘".yellow(),
            };

            println!(
                "{} 步骤 {}: {}",
                status_icon.bold(),
                (step.step_index + 1).to_string().bright_black(),
                step.description
            );

            if let Some(output) = &step.output {
                println!("   输出: {}", output.bright_black());
            }

            if let Some(error) = &step.error {
                println!("   错误: {}", error.red());
            }

            println!("   耗时: {} ms", step.duration_ms.to_string().bright_black());
            println!();
        }
    }

    // 总结
    println!("{}", "=".repeat(60));
    let status = if report.failed_count == 0 {
        format!("{} 场景执行成功", "✓".green().bold())
    } else {
        format!("{} 场景执行失败", "✗".red().bold())
    };
    println!("{}", status);
    println!("{}", "=".repeat(60));

    if report.failed_count > 0 {
        anyhow::bail!("场景执行失败");
    }

    Ok(())
}

async fn list_scenarios() -> Result<()> {
    let config = CliConfig::load()?;
    let scenario_dir = config.get_scenario_dir();

    if !scenario_dir.exists() {
        println!("{}", format!("场景目录不存在: {:?}", scenario_dir).yellow());
        println!("\n可以通过设置配置文件中的 scenario_dir 来指定场景目录");
        return Ok(());
    }

    println!("{}\n", format!("场景目录: {:?}", scenario_dir).bold());

    let entries = std::fs::read_dir(&scenario_dir)
        .with_context(|| format!("读取场景目录失败: {:?}", scenario_dir))?;

    let mut scenarios = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|s| s.to_str());
        if ext != Some("yaml") && ext != Some("yml") && ext != Some("json") {
            continue;
        }

        // 尝试加载场景
        let scenario_result = if ext == Some("json") {
            Scenario::from_json_file(&path)
        } else {
            Scenario::from_yaml_file(&path)
        };

        if let Ok(scenario) = scenario_result {
            scenarios.push((path, scenario));
        }
    }

    if scenarios.is_empty() {
        println!("{}", "没有找到任何场景文件".yellow());
        return Ok(());
    }

    println!("找到 {} 个场景:\n", scenarios.len().to_string().green());

    for (path, scenario) in scenarios {
        println!("{}", path.file_name().unwrap().to_str().unwrap().cyan().bold());
        println!("  名称: {}", scenario.name);

        if let Some(desc) = &scenario.description {
            println!("  描述: {}", desc.bright_black());
        }

        println!("  步骤: {}", scenario.steps.len().to_string().yellow());

        if !scenario.tags.is_empty() {
            println!("  标签: {}", scenario.tags.join(", ").bright_black());
        }

        println!();
    }

    Ok(())
}

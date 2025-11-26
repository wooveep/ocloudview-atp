//! 测试报告管理命令

use anyhow::Result;
use colored::Colorize;
use chrono::Local;
use atp_storage::{StorageManager, Storage, ReportFilter};

pub async fn handle(action: crate::ReportAction) -> Result<()> {
    match action {
        crate::ReportAction::List {
            scenario,
            passed,
            failed,
            limit,
        } => list_reports(scenario, passed, failed, limit).await,
        crate::ReportAction::Show { id } => show_report(id).await,
        crate::ReportAction::Export { id, output, format } => export_report(id, &output, &format).await,
        crate::ReportAction::Delete { id } => delete_report(id).await,
        crate::ReportAction::Stats { scenario, days } => show_stats(&scenario, days).await,
    }
}

async fn list_reports(
    scenario: Option<String>,
    passed: bool,
    failed: bool,
    limit: i64,
) -> Result<()> {
    println!("{} 加载测试报告...", "⏳".cyan());

    let storage_manager = StorageManager::new("~/.config/atp/data.db").await?;
    let storage = Storage::from_manager(&storage_manager);

    let mut filter = ReportFilter {
        scenario_name: scenario,
        limit: Some(limit),
        ..Default::default()
    };

    if passed {
        filter.passed = Some(true);
    } else if failed {
        filter.passed = Some(false);
    }

    let reports = storage.reports().list(&filter).await?;

    if reports.is_empty() {
        println!("\n{} 没有找到测试报告", "ℹ".yellow());
        return Ok(());
    }

    println!("\n{} 找到 {} 个报告:\n", "✓".green(), reports.len());

    // 表头
    println!(
        "{:<6} {:<25} {:<20} {:<8} {:<10} {:<15}",
        "ID".bold(),
        "场景名称".bold(),
        "执行时间".bold(),
        "结果".bold(),
        "步骤".bold(),
        "耗时".bold()
    );
    println!("{}", "-".repeat(90));

    for report in reports {
        let result_str = if report.passed {
            "通过".green()
        } else {
            "失败".red()
        };

        let local_time = report.start_time.with_timezone(&Local);
        let time_str = local_time.format("%Y-%m-%d %H:%M:%S").to_string();

        let steps_str = format!(
            "{}/{}",
            report.success_count,
            report.total_steps
        );

        let duration_str = if let Some(ms) = report.duration_ms {
            format!("{:.2}s", ms as f64 / 1000.0)
        } else {
            "N/A".to_string()
        };

        println!(
            "{:<6} {:<25} {:<20} {:<8} {:<10} {:<15}",
            report.id,
            report.scenario_name,
            time_str,
            result_str,
            steps_str,
            duration_str
        );
    }

    Ok(())
}

async fn show_report(id: i64) -> Result<()> {
    println!("{} 加载报告详情...", "⏳".cyan());

    let storage_manager = StorageManager::new("~/.config/atp/data.db").await?;
    let storage = Storage::from_manager(&storage_manager);

    let report = storage.reports().get_by_id(id).await?;

    if report.is_none() {
        println!("\n{} 未找到报告 ID: {}", "✗".red(), id);
        return Ok(());
    }

    let report = report.unwrap();
    let steps = storage.reports().get_steps(id).await?;

    println!("\n{} 测试报告详情\n", "📊".cyan());
    println!("  ID: {}", report.id);
    println!("  场景: {}", report.scenario_name.yellow());

    if let Some(desc) = &report.description {
        println!("  描述: {}", desc);
    }

    println!("  结果: {}", if report.passed {
        "通过 ✓".green()
    } else {
        "失败 ✗".red()
    });

    let local_time = report.start_time.with_timezone(&Local);
    println!("  开始时间: {}", local_time.format("%Y-%m-%d %H:%M:%S"));

    if let Some(duration_ms) = report.duration_ms {
        println!("  总耗时: {:.2} 秒", duration_ms as f64 / 1000.0);
    }

    println!("\n  步骤统计:");
    println!("    总步骤数: {}", report.total_steps);
    println!("    成功: {}", report.success_count.to_string().green());
    println!("    失败: {}", report.failed_count.to_string().red());
    println!("    跳过: {}", report.skipped_count);

    if !steps.is_empty() {
        println!("\n  步骤详情:\n");

        for step in steps {
            let status_icon = match step.status.as_str() {
                "Success" => "✓".green(),
                "Failed" => "✗".red(),
                "Skipped" => "⊘".yellow(),
                _ => "?".normal(),
            };

            println!("    {} 步骤 {}: {}", status_icon, step.step_index + 1, step.description);

            if let Some(error) = &step.error {
                println!("      错误: {}", error.red());
            }

            if let Some(duration_ms) = step.duration_ms {
                println!("      耗时: {:.2} 秒", duration_ms as f64 / 1000.0);
            }

            if let Some(output) = &step.output {
                println!("      输出: {}", output.dimmed());
            }
        }
    }

    Ok(())
}

async fn export_report(id: i64, output: &str, format: &str) -> Result<()> {
    println!("{} 导出报告...", "⏳".cyan());

    let storage_manager = StorageManager::new("~/.config/atp/data.db").await?;
    let storage = Storage::from_manager(&storage_manager);

    let report = storage.reports().get_by_id(id).await?;

    if report.is_none() {
        println!("\n{} 未找到报告 ID: {}", "✗".red(), id);
        return Ok(());
    }

    let report = report.unwrap();
    let steps = storage.reports().get_steps(id).await?;

    // 构建导出数据
    let export_data = serde_json::json!({
        "report": report,
        "steps": steps,
    });

    let content = match format {
        "json" => serde_json::to_string_pretty(&export_data)?,
        "yaml" => serde_yaml::to_string(&export_data)?,
        _ => anyhow::bail!("不支持的格式: {}", format),
    };

    std::fs::write(output, content)?;

    println!("\n{} 报告已导出到: {}", "✓".green(), output.yellow());

    Ok(())
}

async fn delete_report(id: i64) -> Result<()> {
    println!("{} 删除报告 {}...", "⏳".cyan(), id);

    let storage_manager = StorageManager::new("~/.config/atp/data.db").await?;
    let storage = Storage::from_manager(&storage_manager);

    storage.reports().delete(id).await?;

    println!("\n{} 报告已删除", "✓".green());

    Ok(())
}

async fn show_stats(scenario: &str, days: i32) -> Result<()> {
    println!("{} 加载统计信息...", "⏳".cyan());

    let storage_manager = StorageManager::new("~/.config/atp/data.db").await?;
    let storage = Storage::from_manager(&storage_manager);

    let success_rate = storage.reports().get_success_rate(scenario, days).await?;

    println!("\n{} 场景统计: {}\n", "📈".cyan(), scenario.yellow());
    println!("  时间范围: 最近 {} 天", days);
    println!("  成功率: {:.2}%", success_rate);

    if success_rate >= 90.0 {
        println!("  评级: {} 优秀", "★★★".green());
    } else if success_rate >= 70.0 {
        println!("  评级: {} 良好", "★★".yellow());
    } else {
        println!("  评级: {} 需要改进", "★".red());
    }

    Ok(())
}

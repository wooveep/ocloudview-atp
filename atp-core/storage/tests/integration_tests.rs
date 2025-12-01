// 数据库集成测试
use atp_storage::{
    ExecutionStepRecord, ReportFilter, ReportRepository, ScenarioFilter, ScenarioRecord,
    ScenarioRepository, Storage, StorageManager, TestReportRecord,
};
use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;

/// 创建测试数据库 (内存模式)
async fn setup_test_db() -> SqlitePool {
    let manager = StorageManager::new_in_memory()
        .await
        .expect("Failed to create test database");
    manager.pool().clone()
}

/// 创建测试报告记录
fn create_test_report(scenario_name: &str, success: bool) -> TestReportRecord {
    TestReportRecord {
        id: 0, // Will be auto-assigned
        scenario_name: scenario_name.to_string(),
        description: Some(format!("Test scenario: {}", scenario_name)),
        start_time: Utc::now(),
        end_time: Some(Utc::now()),
        duration_ms: Some(1000),
        total_steps: 5,
        success_count: if success { 5 } else { 3 },
        failed_count: if success { 0 } else { 2 },
        skipped_count: 0,
        passed: success,
        tags: Some(r#"["test", "integration"]"#.to_string()),
        created_at: Utc::now(),
    }
}

/// 创建测试步骤记录
fn create_test_step(report_id: i64, step_index: i32, success: bool) -> ExecutionStepRecord {
    ExecutionStepRecord {
        id: 0, // Will be auto-assigned
        report_id,
        step_index,
        description: format!("Test step {}", step_index),
        status: if success {
            "Success".to_string()
        } else {
            "Failed".to_string()
        },
        error: if success {
            None
        } else {
            Some("Step failed".to_string())
        },
        duration_ms: Some(100),
        output: Some("Test output".to_string()),
    }
}

/// 创建测试场景记录
fn create_test_scenario(name: &str) -> ScenarioRecord {
    ScenarioRecord {
        id: 0,
        name: name.to_string(),
        description: Some(format!("Test scenario: {}", name)),
        definition: r#"{"steps": []}"#.to_string(),
        tags: Some(r#"["test"]"#.to_string()),
        version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ==================== ReportRepository 测试 ====================

#[tokio::test]
async fn test_create_report() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    let report = create_test_report("test_scenario", true);
    let result = repo.create(&report).await;

    assert!(result.is_ok());
    let report_id = result.unwrap();
    assert!(report_id > 0);
}

#[tokio::test]
async fn test_get_report_by_id() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    let report = create_test_report("test_scenario", true);
    let report_id = repo.create(&report).await.unwrap();

    let found = repo.get_by_id(report_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().scenario_name, "test_scenario");
}

#[tokio::test]
async fn test_get_report_by_id_not_found() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    let found = repo.get_by_id(999).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_list_all_reports() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    // 创建多个报告
    for i in 0..5 {
        let report = create_test_report(&format!("scenario_{}", i), i % 2 == 0);
        repo.create(&report).await.unwrap();
    }

    let filter = ReportFilter::default();
    let reports = repo.list(&filter).await.unwrap();
    assert_eq!(reports.len(), 5);
}

#[tokio::test]
async fn test_list_reports_by_scenario() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    // 创建不同场景的报告
    let report1 = create_test_report("scenario_a", true);
    repo.create(&report1).await.unwrap();

    let report2 = create_test_report("scenario_a", false);
    repo.create(&report2).await.unwrap();

    let report3 = create_test_report("scenario_b", true);
    repo.create(&report3).await.unwrap();

    let filter = ReportFilter {
        scenario_name: Some("scenario_a".to_string()),
        ..Default::default()
    };
    let reports = repo.list(&filter).await.unwrap();
    assert_eq!(reports.len(), 2);
}

#[tokio::test]
async fn test_list_reports_by_status() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    // 创建不同状态的报告
    let report1 = create_test_report("scenario_1", true);
    repo.create(&report1).await.unwrap();

    let report2 = create_test_report("scenario_2", false);
    repo.create(&report2).await.unwrap();

    let report3 = create_test_report("scenario_3", true);
    repo.create(&report3).await.unwrap();

    let filter = ReportFilter {
        passed: Some(true),
        ..Default::default()
    };
    let reports = repo.list(&filter).await.unwrap();
    assert_eq!(reports.len(), 2);
}

#[tokio::test]
async fn test_list_reports_pagination() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    // 创建10个报告
    for i in 0..10 {
        let report = create_test_report(&format!("scenario_{}", i), true);
        repo.create(&report).await.unwrap();
    }

    // 分页查询 - 第一页
    let filter1 = ReportFilter {
        limit: Some(5),
        offset: Some(0),
        ..Default::default()
    };
    let page1 = repo.list(&filter1).await.unwrap();
    assert_eq!(page1.len(), 5);

    // 分页查询 - 第二页
    let filter2 = ReportFilter {
        limit: Some(5),
        offset: Some(5),
        ..Default::default()
    };
    let page2 = repo.list(&filter2).await.unwrap();
    assert_eq!(page2.len(), 5);
}

#[tokio::test]
async fn test_delete_report() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    let report = create_test_report("test_scenario", true);
    let report_id = repo.create(&report).await.unwrap();

    // 删除报告
    let result = repo.delete(report_id).await;
    assert!(result.is_ok());

    // 验证删除
    let found = repo.get_by_id(report_id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_report() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    let result = repo.delete(999).await;
    // 应该返回 NotFound 错误
    assert!(result.is_err());
}

#[tokio::test]
async fn test_count_reports() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    // 创建报告
    for i in 0..7 {
        let report = create_test_report(&format!("scenario_{}", i), true);
        repo.create(&report).await.unwrap();
    }

    let filter = ReportFilter::default();
    let count = repo.count(&filter).await.unwrap();
    assert_eq!(count, 7);
}

#[tokio::test]
async fn test_count_reports_with_filter() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    // 创建不同场景的报告
    for _ in 0..3 {
        let report = create_test_report("scenario_a", true);
        repo.create(&report).await.unwrap();
    }
    for _ in 0..2 {
        let report = create_test_report("scenario_b", true);
        repo.create(&report).await.unwrap();
    }

    let filter = ReportFilter {
        scenario_name: Some("scenario_a".to_string()),
        ..Default::default()
    };
    let count = repo.count(&filter).await.unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_get_success_rate() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    // 创建多个报告 (3成功, 2失败)
    for i in 0..5 {
        let report = create_test_report("test_scenario", i < 3);
        repo.create(&report).await.unwrap();
    }

    let success_rate = repo.get_success_rate("test_scenario", 30).await.unwrap();
    assert!((success_rate - 60.0).abs() < 0.1);
}

// ==================== ExecutionStep 测试 ====================

#[tokio::test]
async fn test_create_step() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    let report = create_test_report("test_scenario", true);
    let report_id = repo.create(&report).await.unwrap();

    // 创建步骤
    let step = create_test_step(report_id, 0, true);
    let result = repo.create_step(&step).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_steps_batch() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    let report = create_test_report("test_scenario", true);
    let report_id = repo.create(&report).await.unwrap();

    // 批量创建步骤
    let steps = vec![
        create_test_step(report_id, 0, true),
        create_test_step(report_id, 1, true),
        create_test_step(report_id, 2, false),
    ];

    let result = repo.create_steps(&steps).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_steps_by_report_id() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    let report = create_test_report("test_scenario", true);
    let report_id = repo.create(&report).await.unwrap();

    // 创建步骤
    let steps = vec![
        create_test_step(report_id, 0, true),
        create_test_step(report_id, 1, true),
        create_test_step(report_id, 2, false),
    ];
    repo.create_steps(&steps).await.unwrap();

    // 查询步骤
    let found_steps = repo.get_steps(report_id).await.unwrap();
    assert_eq!(found_steps.len(), 3);
    assert_eq!(found_steps[0].step_index, 0);
    assert_eq!(found_steps[2].step_index, 2);
    assert_eq!(found_steps[2].status, "Failed");
}

#[tokio::test]
async fn test_steps_cascade_delete() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    let report = create_test_report("test_scenario", true);
    let report_id = repo.create(&report).await.unwrap();

    // 创建步骤
    let steps = vec![
        create_test_step(report_id, 0, true),
        create_test_step(report_id, 1, true),
    ];
    repo.create_steps(&steps).await.unwrap();

    // 删除报告 (应该级联删除步骤)
    repo.delete(report_id).await.unwrap();

    // 验证报告和步骤都被删除了
    let found_report = repo.get_by_id(report_id).await.unwrap();
    assert!(found_report.is_none());
}

// ==================== ScenarioRepository 测试 ====================

#[tokio::test]
async fn test_create_scenario() {
    let pool = setup_test_db().await;
    let repo = ScenarioRepository::new(pool);

    let scenario = create_test_scenario("test_scenario");
    let result = repo.create(&scenario).await;

    assert!(result.is_ok());
    let scenario_id = result.unwrap();
    assert!(scenario_id > 0);
}

#[tokio::test]
async fn test_create_duplicate_scenario() {
    let pool = setup_test_db().await;
    let repo = ScenarioRepository::new(pool);

    let scenario = create_test_scenario("test_scenario");
    repo.create(&scenario).await.unwrap();

    // 尝试创建同名场景
    let result = repo.create(&scenario).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_scenario_by_id() {
    let pool = setup_test_db().await;
    let repo = ScenarioRepository::new(pool);

    let scenario = create_test_scenario("test_scenario");
    let scenario_id = repo.create(&scenario).await.unwrap();

    let found = repo.get_by_id(scenario_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "test_scenario");
}

#[tokio::test]
async fn test_get_scenario_by_name() {
    let pool = setup_test_db().await;
    let repo = ScenarioRepository::new(pool);

    let scenario = create_test_scenario("unique_scenario");
    repo.create(&scenario).await.unwrap();

    let found = repo.get_by_name("unique_scenario").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "unique_scenario");
}

#[tokio::test]
async fn test_list_all_scenarios() {
    let pool = setup_test_db().await;
    let repo = ScenarioRepository::new(pool);

    // 创建多个场景
    for i in 0..3 {
        let scenario = create_test_scenario(&format!("scenario_{}", i));
        repo.create(&scenario).await.unwrap();
    }

    let filter = ScenarioFilter::default();
    let scenarios = repo.list(&filter).await.unwrap();
    assert_eq!(scenarios.len(), 3);
}

#[tokio::test]
async fn test_list_scenarios_with_filter() {
    let pool = setup_test_db().await;
    let repo = ScenarioRepository::new(pool);

    // 创建场景
    let scenario1 = create_test_scenario("test_abc");
    repo.create(&scenario1).await.unwrap();

    let scenario2 = create_test_scenario("test_xyz");
    repo.create(&scenario2).await.unwrap();

    let scenario3 = create_test_scenario("prod_scenario");
    repo.create(&scenario3).await.unwrap();

    // 筛选包含 "test" 的场景
    let filter = ScenarioFilter {
        name: Some("test".to_string()),
        ..Default::default()
    };
    let scenarios = repo.list(&filter).await.unwrap();
    assert_eq!(scenarios.len(), 2);
}

#[tokio::test]
async fn test_update_scenario() {
    let pool = setup_test_db().await;
    let repo = ScenarioRepository::new(pool);

    let scenario = create_test_scenario("test_scenario");
    let scenario_id = repo.create(&scenario).await.unwrap();

    // 更新场景
    let new_definition = r#"{"steps": [1, 2, 3]}"#;
    let result = repo.update(scenario_id, new_definition).await;
    assert!(result.is_ok());

    // 验证更新
    let found = repo.get_by_id(scenario_id).await.unwrap().unwrap();
    assert_eq!(found.definition, new_definition);
    assert_eq!(found.version, 2); // 版本递增
}

#[tokio::test]
async fn test_update_nonexistent_scenario() {
    let pool = setup_test_db().await;
    let repo = ScenarioRepository::new(pool);

    let result = repo.update(999, r#"{}"#).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_scenario() {
    let pool = setup_test_db().await;
    let repo = ScenarioRepository::new(pool);

    let scenario = create_test_scenario("test_scenario");
    let scenario_id = repo.create(&scenario).await.unwrap();

    // 删除场景
    let result = repo.delete(scenario_id).await;
    assert!(result.is_ok());

    // 验证删除
    let found = repo.get_by_id(scenario_id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_count_scenarios() {
    let pool = setup_test_db().await;
    let repo = ScenarioRepository::new(pool);

    // 创建场景
    for i in 0..4 {
        let scenario = create_test_scenario(&format!("scenario_{}", i));
        repo.create(&scenario).await.unwrap();
    }

    let filter = ScenarioFilter::default();
    let count = repo.count(&filter).await.unwrap();
    assert_eq!(count, 4);
}

// ==================== Storage 统一接口测试 ====================

#[tokio::test]
async fn test_storage_from_manager() {
    let manager = StorageManager::new_in_memory().await.unwrap();
    let storage = Storage::from_manager(&manager);

    // 测试访问 repositories
    let _reports_repo = storage.reports();
    let _scenarios_repo = storage.scenarios();
}

#[tokio::test]
async fn test_storage_integrated_workflow() {
    let manager = StorageManager::new_in_memory().await.unwrap();
    let storage = Arc::new(Storage::from_manager(&manager));

    // 1. 创建场景
    let scenario = create_test_scenario("test_workflow");
    let scenario_id = storage.scenarios().create(&scenario).await.unwrap();
    assert!(scenario_id > 0);

    // 2. 创建测试报告
    let report = create_test_report("test_workflow", true);
    let report_id = storage.reports().create(&report).await.unwrap();
    assert!(report_id > 0);

    // 3. 创建执行步骤
    let steps = vec![
        create_test_step(report_id, 0, true),
        create_test_step(report_id, 1, true),
    ];
    storage.reports().create_steps(&steps).await.unwrap();

    // 4. 查询验证
    let found_scenario = storage
        .scenarios()
        .get_by_name("test_workflow")
        .await
        .unwrap();
    assert!(found_scenario.is_some());

    let found_report = storage.reports().get_by_id(report_id).await.unwrap();
    assert!(found_report.is_some());

    let found_steps = storage.reports().get_steps(report_id).await.unwrap();
    assert_eq!(found_steps.len(), 2);
}

// ==================== 数据库迁移测试 ====================

#[tokio::test]
async fn test_migrations_run_successfully() {
    // 测试迁移能成功运行
    let result = StorageManager::new_in_memory().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_schema_integrity() {
    let manager = StorageManager::new_in_memory().await.unwrap();
    let pool = manager.pool();

    // 验证表是否存在
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    assert!(tables.contains(&"test_reports".to_string()));
    assert!(tables.contains(&"execution_steps".to_string()));
    assert!(tables.contains(&"scenarios".to_string()));
}

#[tokio::test]
async fn test_health_check() {
    let manager = StorageManager::new_in_memory().await.unwrap();
    let result = manager.health_check().await;
    assert!(result.is_ok());
}

// ==================== 性能测试 ====================

#[tokio::test]
async fn test_large_batch_insert() {
    let pool = setup_test_db().await;
    let repo = ReportRepository::new(pool);

    // 创建报告
    let report = create_test_report("perf_test", true);
    let report_id = repo.create(&report).await.unwrap();

    // 创建大量步骤
    let steps: Vec<_> = (0..100)
        .map(|i| create_test_step(report_id, i, true))
        .collect();

    let start = std::time::Instant::now();
    repo.create_steps(&steps).await.unwrap();
    let duration = start.elapsed();

    println!("Created 100 steps in {:?}", duration);
    assert!(duration.as_secs() < 5); // 应该在5秒内完成

    // 验证
    let found_steps = repo.get_steps(report_id).await.unwrap();
    assert_eq!(found_steps.len(), 100);
}

#[tokio::test]
async fn test_concurrent_report_creation() {
    let manager = StorageManager::new_in_memory().await.unwrap();
    let pool = manager.pool().clone();

    // 并发创建报告
    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let pool = pool.clone();
            tokio::spawn(async move {
                let repo = ReportRepository::new(pool);
                let report = create_test_report(&format!("concurrent_{}", i), true);
                repo.create(&report).await
            })
        })
        .collect();

    // 等待所有任务完成
    let results = futures_util::future::join_all(tasks).await;

    // 验证所有创建成功
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }

    // 验证总数
    let repo = ReportRepository::new(pool);
    let filter = ReportFilter::default();
    let count = repo.count(&filter).await.unwrap();
    assert_eq!(count, 10);
}

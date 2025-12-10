use chrono::Utc;
use sqlx::SqlitePool;
use tracing::debug;

use crate::error::{Result, StorageError};
use crate::models::{ExecutionStepRecord, ReportFilter, TestReportRecord};

/// 测试报告仓储
pub struct ReportRepository {
    pool: SqlitePool,
}

impl ReportRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 创建新的测试报告
    pub async fn create(&self, report: &TestReportRecord) -> Result<i64> {
        let tags_json = report.tags.as_deref();

        let result = sqlx::query(
            r#"
            INSERT INTO test_reports
            (scenario_name, description, start_time, end_time, duration_ms,
             total_steps, success_count, failed_count, skipped_count, passed, tags)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&report.scenario_name)
        .bind(&report.description)
        .bind(&report.start_time)
        .bind(&report.end_time)
        .bind(report.duration_ms)
        .bind(report.total_steps)
        .bind(report.success_count)
        .bind(report.failed_count)
        .bind(report.skipped_count)
        .bind(report.passed)
        .bind(tags_json)
        .execute(&self.pool)
        .await?;

        let report_id = result.last_insert_rowid();
        debug!("Created test report with ID: {}", report_id);

        Ok(report_id)
    }

    /// 创建执行步骤
    pub async fn create_step(&self, step: &ExecutionStepRecord) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO execution_steps
            (report_id, step_index, description, status, error, duration_ms, output)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(step.report_id)
        .bind(step.step_index)
        .bind(&step.description)
        .bind(&step.status)
        .bind(&step.error)
        .bind(step.duration_ms)
        .bind(&step.output)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 批量创建执行步骤
    pub async fn create_steps(&self, steps: &[ExecutionStepRecord]) -> Result<()> {
        for step in steps {
            self.create_step(step).await?;
        }
        Ok(())
    }

    /// 根据ID获取报告
    pub async fn get_by_id(&self, id: i64) -> Result<Option<TestReportRecord>> {
        let report = sqlx::query_as::<_, TestReportRecord>(
            r#"
            SELECT id, scenario_name, description, start_time, end_time, duration_ms,
                   total_steps, success_count, failed_count, skipped_count, passed, tags, created_at
            FROM test_reports
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(report)
    }

    /// 获取报告的所有步骤
    pub async fn get_steps(&self, report_id: i64) -> Result<Vec<ExecutionStepRecord>> {
        let steps = sqlx::query_as::<_, ExecutionStepRecord>(
            r#"
            SELECT id, report_id, step_index, description, status, error, duration_ms, output
            FROM execution_steps
            WHERE report_id = ?
            ORDER BY step_index ASC
            "#,
        )
        .bind(report_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(steps)
    }

    /// 查询报告列表
    pub async fn list(&self, filter: &ReportFilter) -> Result<Vec<TestReportRecord>> {
        let mut query = String::from(
            r#"
            SELECT id, scenario_name, description, start_time, end_time, duration_ms,
                   total_steps, success_count, failed_count, skipped_count, passed, tags, created_at
            FROM test_reports
            WHERE 1=1
            "#,
        );

        let mut bindings = Vec::new();

        // 构建查询条件
        if let Some(scenario_name) = &filter.scenario_name {
            query.push_str(" AND scenario_name = ?");
            bindings.push(scenario_name.clone());
        }

        if let Some(passed) = filter.passed {
            query.push_str(" AND passed = ?");
            bindings.push(if passed { "1" } else { "0" }.to_string());
        }

        if filter.start_time_from.is_some() {
            query.push_str(" AND start_time >= ?");
        }

        if filter.start_time_to.is_some() {
            query.push_str(" AND start_time <= ?");
        }

        // TODO: 支持 tags 过滤 (需要 JSON 函数)

        query.push_str(" ORDER BY start_time DESC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sql_query = sqlx::query_as::<_, TestReportRecord>(&query);

        for binding in &bindings {
            sql_query = sql_query.bind(binding);
        }

        if let Some(start_from) = filter.start_time_from {
            sql_query = sql_query.bind(start_from);
        }

        if let Some(start_to) = filter.start_time_to {
            sql_query = sql_query.bind(start_to);
        }

        let reports = sql_query.fetch_all(&self.pool).await?;

        Ok(reports)
    }

    /// 删除报告(级联删除步骤)
    pub async fn delete(&self, id: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM test_reports WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!("Report {} not found", id)));
        }

        debug!("Deleted test report {}", id);

        Ok(())
    }

    /// 获取场景的成功率统计
    pub async fn get_success_rate(&self, scenario_name: &str, days: i32) -> Result<f64> {
        let start_time = Utc::now() - chrono::Duration::days(days as i64);

        let result: (f64,) = sqlx::query_as(
            r#"
            SELECT CAST(SUM(CASE WHEN passed = 1 THEN 1 ELSE 0 END) AS REAL) / COUNT(*) * 100
            FROM test_reports
            WHERE scenario_name = ? AND start_time >= ?
            "#,
        )
        .bind(scenario_name)
        .bind(start_time)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or((0.0,));

        Ok(result.0)
    }

    /// 获取报告总数
    pub async fn count(&self, filter: &ReportFilter) -> Result<i64> {
        let mut query = String::from("SELECT COUNT(*) FROM test_reports WHERE 1=1");

        let mut bindings = Vec::new();

        if let Some(scenario_name) = &filter.scenario_name {
            query.push_str(" AND scenario_name = ?");
            bindings.push(scenario_name.clone());
        }

        if let Some(passed) = filter.passed {
            query.push_str(" AND passed = ?");
            bindings.push(if passed { "1" } else { "0" }.to_string());
        }

        let mut sql_query = sqlx::query_as::<_, (i64,)>(&query);

        for binding in &bindings {
            sql_query = sql_query.bind(binding);
        }

        let (count,) = sql_query.fetch_one(&self.pool).await?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::StorageManager;

    #[tokio::test]
    async fn test_create_and_get_report() {
        let storage = StorageManager::new_in_memory().await.unwrap();
        let repo = ReportRepository::new(storage.pool().clone());

        let report = TestReportRecord {
            id: 0,
            scenario_name: "test_scenario".to_string(),
            description: Some("Test description".to_string()),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            duration_ms: Some(1000),
            total_steps: 5,
            success_count: 5,
            failed_count: 0,
            skipped_count: 0,
            passed: true,
            tags: Some(r#"["smoke", "regression"]"#.to_string()),
            created_at: Utc::now(),
        };

        let report_id = repo.create(&report).await.unwrap();
        assert!(report_id > 0);

        let retrieved = repo.get_by_id(report_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().scenario_name, "test_scenario");
    }
}

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::debug;

use crate::error::{Result, StorageError};
use crate::models::{ScenarioFilter, ScenarioRecord};

/// 场景仓储
pub struct ScenarioRepository {
    pool: SqlitePool,
}

impl ScenarioRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 创建新场景
    pub async fn create(&self, scenario: &ScenarioRecord) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO scenarios
            (name, description, definition, tags, version, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&scenario.name)
        .bind(&scenario.description)
        .bind(&scenario.definition)
        .bind(&scenario.tags)
        .bind(scenario.version)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                StorageError::AlreadyExists(format!("Scenario '{}' already exists", scenario.name))
            } else {
                StorageError::DatabaseError(e)
            }
        })?;

        let scenario_id = result.last_insert_rowid();
        debug!(
            "Created scenario '{}' with ID: {}",
            scenario.name, scenario_id
        );

        Ok(scenario_id)
    }

    /// 更新场景(递增版本)
    pub async fn update(&self, id: i64, definition: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE scenarios
            SET definition = ?, version = version + 1, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(definition)
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!("Scenario {} not found", id)));
        }

        debug!("Updated scenario {}", id);

        Ok(())
    }

    /// 根据ID获取场景
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ScenarioRecord>> {
        let scenario = sqlx::query_as::<_, ScenarioRecord>(
            r#"
            SELECT id, name, description, definition, tags, version, created_at, updated_at
            FROM scenarios
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(scenario)
    }

    /// 根据名称获取场景
    pub async fn get_by_name(&self, name: &str) -> Result<Option<ScenarioRecord>> {
        let scenario = sqlx::query_as::<_, ScenarioRecord>(
            r#"
            SELECT id, name, description, definition, tags, version, created_at, updated_at
            FROM scenarios
            WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(scenario)
    }

    /// 查询场景列表
    pub async fn list(&self, filter: &ScenarioFilter) -> Result<Vec<ScenarioRecord>> {
        let mut query = String::from(
            r#"
            SELECT id, name, description, definition, tags, version, created_at, updated_at
            FROM scenarios
            WHERE 1=1
            "#,
        );

        let mut bindings = Vec::new();

        if let Some(name) = &filter.name {
            query.push_str(" AND name LIKE ?");
            bindings.push(format!("%{}%", name));
        }

        // TODO: 支持 tags 过滤

        query.push_str(" ORDER BY updated_at DESC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sql_query = sqlx::query_as::<_, ScenarioRecord>(&query);

        for binding in &bindings {
            sql_query = sql_query.bind(binding);
        }

        let scenarios = sql_query.fetch_all(&self.pool).await?;

        Ok(scenarios)
    }

    /// 删除场景
    pub async fn delete(&self, id: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM scenarios WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!("Scenario {} not found", id)));
        }

        debug!("Deleted scenario {}", id);

        Ok(())
    }

    /// 获取场景总数
    pub async fn count(&self, filter: &ScenarioFilter) -> Result<i64> {
        let mut query = String::from("SELECT COUNT(*) FROM scenarios WHERE 1=1");

        let mut bindings = Vec::new();

        if let Some(name) = &filter.name {
            query.push_str(" AND name LIKE ?");
            bindings.push(format!("%{}%", name));
        }

        let mut sql_query = sqlx::query_as::<_, (i64,)>(&query);

        for binding in &bindings {
            sql_query = sql_query.bind(binding);
        }

        let (count,) = sql_query.fetch_one(&self.pool).await?;

        Ok(count)
    }
}

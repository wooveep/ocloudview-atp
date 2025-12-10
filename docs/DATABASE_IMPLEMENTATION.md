# 数据库层实现总结

## 概述

本文档记录了 OCloudView ATP 项目中数据库层的实现细节。数据库层用于持久化测试报告、场景定义、主机配置和性能指标等数据。

**实现日期**: 2025-11-25
**版本**: v0.1.0
**状态**: 基础架构完成,集成工作待完成

## 设计决策

### 数据库选型: SQLite

**选择理由**:
1. **零配置**: 嵌入式数据库,无需单独安装服务
2. **轻量级**: 适合CLI工具和单用户场景
3. **跨平台**: 支持 Linux, macOS, Windows
4. **事务支持**: ACID 兼容
5. **易于备份**: 单文件数据库,备份简单

**替代方案**:
- PostgreSQL: 适用于未来的 Web 控制台和多用户场景

### ORM 选型: 原生 sqlx

**选择理由**:
1. **编译时类型检查**: sqlx 提供编译时 SQL 验证
2. **异步支持**: 完全支持 tokio 异步
3. **轻量级**: 无重型 ORM 开销
4. **灵活性**: 直接 SQL 控制

**未选择 Diesel/Sea-ORM 的原因**:
- 项目较小,ORM 的抽象层开销不值得
- 需要更灵活的查询控制

## 模块架构

```
atp-core/storage/
├── src/
│   ├── lib.rs               # 模块入口和统一接口
│   ├── connection.rs        # StorageManager - 数据库连接管理
│   ├── models.rs            # 数据模型定义
│   ├── error.rs             # 错误类型
│   └── repositories/        # Repository 数据访问层
│       ├── mod.rs
│       ├── reports.rs       # 测试报告 CRUD
│       └── scenarios.rs     # 场景库 CRUD
├── migrations/
│   └── 001_initial.sql      # 初始数据库 schema
└── Cargo.toml
```

## 数据库 Schema

### 表结构

#### 1. test_reports - 测试报告表

存储测试执行的汇总信息。

```sql
CREATE TABLE test_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scenario_name TEXT NOT NULL,           -- 场景名称
    description TEXT,                       -- 场景描述
    start_time DATETIME NOT NULL,          -- 开始时间
    end_time DATETIME,                     -- 结束时间
    duration_ms INTEGER,                   -- 总耗时(毫秒)
    total_steps INTEGER NOT NULL DEFAULT 0,    -- 总步骤数
    success_count INTEGER NOT NULL DEFAULT 0,  -- 成功步骤数
    failed_count INTEGER NOT NULL DEFAULT 0,   -- 失败步骤数
    skipped_count INTEGER NOT NULL DEFAULT 0,  -- 跳过步骤数
    passed BOOLEAN NOT NULL DEFAULT 0,     -- 是否通过
    tags TEXT,                             -- 标签(JSON array)
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_reports_time ON test_reports(start_time);
CREATE INDEX idx_reports_scenario ON test_reports(scenario_name);
CREATE INDEX idx_reports_passed ON test_reports(passed);
```

**设计说明**:
- `start_time` 用于时间范围查询和排序
- `tags` 存储为 JSON 字符串,便于后续扩展为 JSON 查询
- `passed` 布尔字段便于快速筛选成功/失败报告

#### 2. execution_steps - 执行步骤表

存储测试报告中每个步骤的详细信息。

```sql
CREATE TABLE execution_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_id INTEGER NOT NULL,           -- 关联报告 ID
    step_index INTEGER NOT NULL,          -- 步骤索引
    description TEXT NOT NULL,            -- 步骤描述
    status TEXT NOT NULL,                 -- 状态: 'Success', 'Failed', 'Skipped'
    error TEXT,                           -- 错误信息
    duration_ms INTEGER,                  -- 耗时(毫秒)
    output TEXT,                          -- 输出内容
    FOREIGN KEY (report_id) REFERENCES test_reports(id) ON DELETE CASCADE
);

-- 索引
CREATE INDEX idx_steps_report ON execution_steps(report_id);
CREATE INDEX idx_steps_status ON execution_steps(status);
```

**设计说明**:
- `report_id` 外键级联删除,删除报告时自动删除步骤
- `step_index` 保持执行顺序
- `status` 使用文本而非枚举,便于扩展

#### 3. scenarios - 场景库表

存储测试场景定义,支持版本管理。

```sql
CREATE TABLE scenarios (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,            -- 场景唯一名称
    description TEXT,                     -- 场景描述
    definition TEXT NOT NULL,             -- 场景定义(JSON/YAML)
    tags TEXT,                            -- 标签(JSON array)
    version INTEGER NOT NULL DEFAULT 1,   -- 版本号
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX idx_scenarios_name ON scenarios(name);
CREATE INDEX idx_scenarios_updated ON scenarios(updated_at);
```

**设计说明**:
- `name` 唯一约束,避免重复场景
- `version` 自动递增,支持简单版本管理
- `definition` 存储完整的 YAML/JSON 定义

#### 4. hosts - 主机配置表

存储主机连接配置(可选,当前主机配置仍使用 TOML 文件)。

```sql
CREATE TABLE hosts (
    id TEXT PRIMARY KEY,                  -- 主机 ID
    host TEXT NOT NULL,                   -- IP 地址或主机名
    uri TEXT NOT NULL,                    -- libvirt URI
    tags TEXT,                            -- 标签(JSON array)
    metadata TEXT,                        -- 元数据(JSON object)
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

#### 5. connection_metrics - 性能指标表

存储连接池性能监控数据(时序数据)。

```sql
CREATE TABLE connection_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    host_id TEXT NOT NULL,                    -- 主机 ID
    timestamp DATETIME NOT NULL,              -- 时间戳
    total_connections INTEGER NOT NULL,       -- 总连接数
    active_connections INTEGER NOT NULL,      -- 活跃连接数
    total_requests INTEGER NOT NULL,          -- 总请求数
    total_errors INTEGER NOT NULL,            -- 总错误数
    avg_response_time REAL,                   -- 平均响应时间
    FOREIGN KEY (host_id) REFERENCES hosts(id) ON DELETE CASCADE
);

-- 索引
CREATE INDEX idx_metrics_host_time ON connection_metrics(host_id, timestamp);
CREATE INDEX idx_metrics_timestamp ON connection_metrics(timestamp);
```

**设计说明**:
- 复合索引 `(host_id, timestamp)` 支持高效的时间序列查询
- 适合存储定期采样的性能数据

## 核心组件

### 1. StorageManager - 连接管理器

**职责**:
- 管理数据库连接池
- 执行数据库迁移
- 提供健康检查

**主要API**:
```rust
impl StorageManager {
    // 创建新的存储管理器
    pub async fn new(db_path: &str) -> Result<Self>;

    // 创建内存数据库(用于测试)
    pub async fn new_in_memory() -> Result<Self>;

    // 获取连接池
    pub fn pool(&self) -> &SqlitePool;

    // 健康检查
    pub async fn health_check(&self) -> Result<()>;
}
```

**使用示例**:
```rust
let storage_manager = StorageManager::new("~/.config/atp/data.db").await?;
storage_manager.health_check().await?;
```

### 2. Repository Pattern - 数据访问层

#### ReportRepository

**职责**: 测试报告的 CRUD 操作和聚合查询

**主要API**:
```rust
impl ReportRepository {
    // 创建测试报告
    pub async fn create(&self, report: &TestReportRecord) -> Result<i64>;

    // 批量创建执行步骤
    pub async fn create_steps(&self, steps: &[ExecutionStepRecord]) -> Result<()>;

    // 根据ID获取报告
    pub async fn get_by_id(&self, id: i64) -> Result<Option<TestReportRecord>>;

    // 获取报告的所有步骤
    pub async fn get_steps(&self, report_id: i64) -> Result<Vec<ExecutionStepRecord>>;

    // 查询报告列表(支持过滤)
    pub async fn list(&self, filter: &ReportFilter) -> Result<Vec<TestReportRecord>>;

    // 删除报告(级联删除步骤)
    pub async fn delete(&self, id: i64) -> Result<()>;

    // 获取场景成功率
    pub async fn get_success_rate(&self, scenario_name: &str, days: i32) -> Result<f64>;

    // 获取报告总数
    pub async fn count(&self, filter: &ReportFilter) -> Result<i64>;
}
```

**查询过滤器**:
```rust
pub struct ReportFilter {
    pub scenario_name: Option<String>,
    pub passed: Option<bool>,
    pub start_time_from: Option<DateTime<Utc>>,
    pub start_time_to: Option<DateTime<Utc>>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
```

#### ScenarioRepository

**职责**: 场景定义的 CRUD 操作

**主要API**:
```rust
impl ScenarioRepository {
    // 创建场景
    pub async fn create(&self, scenario: &ScenarioRecord) -> Result<i64>;

    // 更新场景(递增版本)
    pub async fn update(&self, id: i64, definition: &str) -> Result<()>;

    // 根据ID获取场景
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ScenarioRecord>>;

    // 根据名称获取场景
    pub async fn get_by_name(&self, name: &str) -> Result<Option<ScenarioRecord>>;

    // 查询场景列表
    pub async fn list(&self, filter: &ScenarioFilter) -> Result<Vec<ScenarioRecord>>;

    // 删除场景
    pub async fn delete(&self, id: i64) -> Result<()>;
}
```

### 3. Storage - 统一数据访问接口

提供统一的数据访问入口,聚合所有 Repository。

```rust
pub struct Storage {
    reports: ReportRepository,
    scenarios: ScenarioRepository,
    // TODO: hosts, metrics repositories
}

impl Storage {
    pub fn from_manager(manager: &StorageManager) -> Self;

    pub fn reports(&self) -> &ReportRepository;
    pub fn scenarios(&self) -> &ScenarioRepository;
}
```

**使用示例**:
```rust
let storage_manager = StorageManager::new("~/.config/atp/data.db").await?;
let storage = Storage::from_manager(&storage_manager);

// 查询报告
let reports = storage.reports().list(&ReportFilter::default()).await?;

// 查询场景
let scenario = storage.scenarios().get_by_name("test_scenario").await?;
```

## 与现有模块的集成

### 1. Executor 集成 (executor/src/runner.rs)

**集成点**: 在场景执行完成后保存报告到数据库

**实现步骤** (已添加 TODO 注释):

```rust
// 1. 在 ScenarioRunner 添加 storage 字段
pub struct ScenarioRunner {
    transport_manager: Arc<TransportManager>,
    protocol_registry: Arc<ProtocolRegistry>,
    protocol_cache: HashMap<String, Box<dyn Protocol>>,
    default_timeout: Duration,
    storage: Option<Arc<atp_storage::Storage>>, // 新增
}

// 2. 在 run() 方法结束时保存报告
pub async fn run(&mut self, scenario: &Scenario) -> Result<ExecutionReport> {
    // ... 执行场景 ...

    // 保存到数据库
    if let Some(storage) = &self.storage {
        self.save_report_to_db(storage, &report).await?;
    }

    Ok(report)
}

// 3. 实现 save_report_to_db() 辅助方法 (完整代码见 TODO 注释)
async fn save_report_to_db(
    &self,
    storage: &atp_storage::Storage,
    report: &ExecutionReport
) -> Result<i64> {
    // 转换并保存...
}
```

**集成价值**:
- 自动保存所有执行历史
- 支持历史追溯和趋势分析
- 无需手动导出报告

### 2. CLI 报告命令 (cli/src/commands/report.rs)

**已实现命令框架** (完整代码见文件):

| 命令 | 功能 | 示例 |
|------|------|------|
| `atp report list` | 列出测试报告 | `atp report list --scenario test --limit 20` |
| `atp report show <id>` | 显示报告详情 | `atp report show 42` |
| `atp report export <id>` | 导出报告 | `atp report export 42 --output report.json` |
| `atp report delete <id>` | 删除报告 | `atp report delete 42` |
| `atp report stats <scenario>` | 统计信息 | `atp report stats test_scenario --days 30` |

**启用步骤**:
1. 在 `cli/Cargo.toml` 添加依赖: `atp-storage = { path = "../../atp-core/storage" }`
2. 在 `main.rs` 添加 `Report(ReportAction)` 枚举
3. 在 `commands/mod.rs` 取消注释 `pub mod report;`

### 3. Transport 性能指标 (transport/src/manager.rs)

**集成点**: 定期保存连接池监控数据

**实现方法** (已添加 TODO 注释):

```rust
impl TransportManager {
    pub async fn save_metrics(&self, storage: &atp_storage::Storage) -> Result<()> {
        // 获取所有主机统计
        let stats = self.stats().await;

        // 保存到数据库
        for (host_id, pool_stats) in stats {
            let metric_record = ConnectionMetricRecord {
                host_id,
                timestamp: Utc::now(),
                // ... 其他字段
            };
            storage.metrics().create(&metric_record).await?;
        }
        Ok(())
    }
}
```

**使用场景**:
- 定期任务调用 `save_metrics()` (例如每 5 分钟)
- 用于性能趋势分析和容量规划

## 数据迁移

### 迁移策略

当前使用内嵌 SQL 迁移脚本:
- `migrations/001_initial.sql` - 初始 schema
- 未来迁移: `002_xxx.sql`, `003_xxx.sql`, ...

### 迁移执行

迁移在 `StorageManager::new()` 时自动执行:

```rust
async fn run_migrations(&self) -> Result<()> {
    let migration_sql = include_str!("../migrations/001_initial.sql");
    sqlx::query(migration_sql).execute(&self.pool).await?;
    Ok(())
}
```

### 未来改进

可使用 `sqlx-cli` 进行迁移管理:

```bash
# 创建新迁移
sqlx migrate add <name>

# 运行迁移
sqlx migrate run
```

## 数据备份和恢复

### 备份策略

**手动备份**:
```bash
cp ~/.config/atp/data.db ~/.config/atp/backups/data-$(date +%Y%m%d).db
```

**定期备份** (cron):
```bash
0 2 * * * cp ~/.config/atp/data.db ~/.config/atp/backups/data-$(date +\%Y\%m\%d).db
```

**导出为 SQL**:
```bash
sqlite3 ~/.config/atp/data.db .dump > backup.sql
```

### 数据保留策略

**建议策略**:
- 测试报告: 保留 180 天详细数据
- 性能指标: 保留 30 天原始数据,30 天后降采样
- 场景定义: 永久保留

**实现示例**:
```sql
-- 清理 180 天前的报告
DELETE FROM test_reports
WHERE start_time < datetime('now', '-180 days');

-- 聚合旧指标数据
INSERT INTO metrics_hourly
SELECT
    date_trunc('hour', timestamp),
    host_id,
    AVG(value), MAX(value), MIN(value)
FROM connection_metrics
WHERE timestamp < datetime('now', '-30 days')
GROUP BY 1, 2;
```

## 性能优化

### 索引策略

已添加关键索引:
- 时间范围查询: `idx_reports_time`, `idx_metrics_timestamp`
- 外键查询: `idx_steps_report`
- 过滤查询: `idx_reports_passed`, `idx_steps_status`

### 查询优化

1. **使用准备好的语句** - sqlx 自动使用
2. **批量插入** - `create_steps()` 批量插入步骤
3. **分页查询** - ReportFilter 支持 `limit` 和 `offset`

### 连接池配置

```rust
SqlitePoolOptions::new()
    .max_connections(5)  // 单用户场景足够
    .connect(&db_url)
    .await?
```

## 测试策略

### 单元测试

每个 Repository 都包含基本测试:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::StorageManager;

    #[tokio::test]
    async fn test_create_and_get_report() {
        let storage = StorageManager::new_in_memory().await.unwrap();
        let repo = ReportRepository::new(storage.pool().clone());
        // ... 测试逻辑
    }
}
```

### 集成测试

TODO: 添加端到端测试,验证:
- 场景执行 -> 报告保存 -> 报告查询
- 多并发写入
- 数据一致性

## 待完成工作

### 高优先级

1. **HostRepository 和 MetricRepository** (约 200 行)
   - 主机配置持久化
   - 性能指标 CRUD

2. **Executor 集成** (约 50 行)
   - 取消注释 TODO 代码
   - 添加依赖
   - 测试验证

3. **CLI 报告命令集成** (约 50 行)
   - 取消注释 TODO 代码
   - 添加依赖
   - 测试验证

### 中优先级

4. **单元测试** (约 300 行)
   - Repository 测试覆盖率 > 80%
   - 异常情况测试

5. **场景导入/导出** (约 100 行)
   - YAML 文件 -> 数据库
   - 数据库 -> YAML 文件

6. **数据清理工具** (约 50 行)
   - CLI 命令: `atp db cleanup --days 180`
   - 自动清理旧数据

### 低优先级

7. **PostgreSQL 支持** (约 200 行)
   - 添加 PostgreSQL 适配器
   - 配置选择数据库类型

8. **数据分析功能** (约 200 行)
   - 成功率趋势图数据
   - 失败根因分析
   - 性能回归检测

## 技术债务

1. **JSON 字段查询**: 当前 tags 存储为 JSON 字符串,未使用 SQLite JSON 函数
2. **事务支持**: Repository 未使用事务,批量操作可能不一致
3. **错误处理**: 部分错误信息不够详细
4. **连接池调优**: 未针对不同场景调优连接池配置

## 参考资源

- **sqlx 文档**: https://docs.rs/sqlx/
- **SQLite 文档**: https://www.sqlite.org/docs.html
- **Repository Pattern**: https://martinfowler.com/eaaCatalog/repository.html

---

**文档版本**: v1.0
**最后更新**: 2025-11-25
**维护者**: OCloudView ATP Team

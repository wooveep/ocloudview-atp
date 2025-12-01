# ATP 数据库使用指南

## 概述

本文档详细说明如何在 OCloudView ATP 项目中使用数据库层,包括测试报告持久化、场景管理和性能监控数据存储。

**数据库类型**: SQLite
**默认位置**: `~/.config/atp/data.db`
**ORM**: sqlx (异步 SQL 客户端)

---

## 目录

- [快速开始](#快速开始)
- [数据库架构](#数据库架构)
- [核心组件](#核心组件)
- [使用示例](#使用示例)
- [最佳实践](#最佳实践)
- [故障排查](#故障排查)
- [API 参考](#api-参考)

---

## 快速开始

### 1. 初始化数据库

```rust
use atp_storage::{StorageManager, Storage};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化存储管理器
    let storage_manager = StorageManager::new("~/.config/atp/data.db").await?;

    // 创建统一存储接口
    let storage = Arc::new(Storage::from_manager(&storage_manager));

    // 健康检查
    storage_manager.health_check().await?;

    println!("数据库已成功初始化!");
    Ok(())
}
```

### 2. 保存测试报告

```rust
use atp_storage::TestReportRecord;
use chrono::Utc;

// 创建测试报告
let report = TestReportRecord {
    id: 0, // 自动分配
    scenario_name: "键盘输入测试".to_string(),
    description: Some("测试键盘输入功能".to_string()),
    start_time: Utc::now(),
    end_time: Some(Utc::now()),
    duration_ms: Some(5000),
    total_steps: 10,
    success_count: 10,
    failed_count: 0,
    skipped_count: 0,
    passed: true,
    tags: Some(r#"["smoke", "regression"]"#.to_string()),
    created_at: Utc::now(),
};

// 保存到数据库
let report_id = storage.reports().create(&report).await?;
println!("报告已保存,ID: {}", report_id);
```

### 3. 查询测试报告

```rust
use atp_storage::ReportFilter;

// 查询最近的测试报告
let filter = ReportFilter {
    limit: Some(10),
    ..Default::default()
};

let reports = storage.reports().list(&filter).await?;

for report in reports {
    println!("{} - {} - {}",
        report.id,
        report.scenario_name,
        if report.passed { "通过" } else { "失败" }
    );
}
```

---

## 数据库架构

### 表结构

#### 1. test_reports (测试报告表)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PRIMARY KEY | 报告ID(自增) |
| scenario_name | TEXT NOT NULL | 场景名称 |
| description | TEXT | 场景描述 |
| start_time | TIMESTAMP NOT NULL | 开始时间 |
| end_time | TIMESTAMP | 结束时间 |
| duration_ms | INTEGER | 执行时长(毫秒) |
| total_steps | INTEGER | 总步骤数 |
| success_count | INTEGER | 成功步骤数 |
| failed_count | INTEGER | 失败步骤数 |
| skipped_count | INTEGER | 跳过步骤数 |
| passed | BOOLEAN | 是否通过 |
| tags | TEXT | 标签(JSON数组) |
| created_at | TIMESTAMP | 创建时间 |

**索引**:
- `idx_scenario_name` - 按场景名称查询
- `idx_start_time` - 按时间范围查询
- `idx_passed` - 按通过状态筛选

#### 2. execution_steps (执行步骤表)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PRIMARY KEY | 步骤ID(自增) |
| report_id | INTEGER NOT NULL | 关联报告ID |
| step_index | INTEGER | 步骤序号 |
| description | TEXT | 步骤描述 |
| status | TEXT | 状态(Success/Failed/Skipped) |
| error | TEXT | 错误信息 |
| duration_ms | INTEGER | 执行时长(毫秒) |
| output | TEXT | 输出内容 |

**外键**: `report_id` → `test_reports(id)` (级联删除)
**索引**: `idx_report_id` - 按报告查询步骤

#### 3. scenarios (场景库表)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PRIMARY KEY | 场景ID(自增) |
| name | TEXT UNIQUE NOT NULL | 场景名称(唯一) |
| description | TEXT | 场景描述 |
| definition | TEXT NOT NULL | 场景定义(JSON/YAML) |
| tags | TEXT | 标签(JSON数组) |
| version | INTEGER | 版本号 |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

**索引**: `idx_scenario_name_unique` - 确保名称唯一

---

## 核心组件

### 1. StorageManager

负责数据库连接、迁移和生命周期管理。

```rust
pub struct StorageManager {
    pool: SqlitePool,
}

impl StorageManager {
    // 创建生产环境数据库
    pub async fn new(db_path: &str) -> Result<Self>

    // 创建内存数据库(测试用)
    pub async fn new_in_memory() -> Result<Self>

    // 获取连接池
    pub fn pool(&self) -> &SqlitePool

    // 健康检查
    pub async fn health_check(&self) -> Result<()>

    // 关闭数据库
    pub async fn close(&self)
}
```

### 2. Storage (统一接口)

提供访问所有 Repository 的统一入口。

```rust
pub struct Storage {
    reports: ReportRepository,
    scenarios: ScenarioRepository,
}

impl Storage {
    pub fn from_manager(manager: &StorageManager) -> Self
    pub fn reports(&self) -> &ReportRepository
    pub fn scenarios(&self) -> &ScenarioRepository
}
```

### 3. ReportRepository

管理测试报告的 CRUD 操作。

**主要方法**:
- `create(&self, report: &TestReportRecord) -> Result<i64>` - 创建报告
- `get_by_id(&self, id: i64) -> Result<Option<TestReportRecord>>` - 根据ID查询
- `list(&self, filter: &ReportFilter) -> Result<Vec<TestReportRecord>>` - 列表查询
- `delete(&self, id: i64) -> Result<()>` - 删除报告
- `count(&self, filter: &ReportFilter) -> Result<i64>` - 统计数量
- `get_success_rate(&self, scenario_name: &str, days: i32) -> Result<f64>` - 成功率
- `create_step(&self, step: &ExecutionStepRecord) -> Result<i64>` - 创建步骤
- `create_steps(&self, steps: &[ExecutionStepRecord]) -> Result<()>` - 批量创建步骤
- `get_steps(&self, report_id: i64) -> Result<Vec<ExecutionStepRecord>>` - 查询步骤

### 4. ScenarioRepository

管理测试场景的 CRUD 操作。

**主要方法**:
- `create(&self, scenario: &ScenarioRecord) -> Result<i64>` - 创建场景
- `get_by_id(&self, id: i64) -> Result<Option<ScenarioRecord>>` - 根据ID查询
- `get_by_name(&self, name: &str) -> Result<Option<ScenarioRecord>>` - 根据名称查询
- `list(&self, filter: &ScenarioFilter) -> Result<Vec<ScenarioRecord>>` - 列表查询
- `update(&self, id: i64, definition: &str) -> Result<()>` - 更新场景
- `delete(&self, id: i64) -> Result<()>` - 删除场景
- `count(&self, filter: &ScenarioFilter) -> Result<i64>` - 统计数量

---

## 使用示例

### 示例1: 完整的测试报告保存流程

```rust
use atp_storage::{Storage, StorageManager, TestReportRecord, ExecutionStepRecord};
use chrono::Utc;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化数据库
    let manager = StorageManager::new("~/.config/atp/data.db").await?;
    let storage = Arc::new(Storage::from_manager(&manager));

    // 2. 创建测试报告
    let start_time = Utc::now();
    let report = TestReportRecord {
        id: 0,
        scenario_name: "用户登录测试".to_string(),
        description: Some("测试VDI用户登录流程".to_string()),
        start_time,
        end_time: Some(Utc::now()),
        duration_ms: Some(3000),
        total_steps: 3,
        success_count: 3,
        failed_count: 0,
        skipped_count: 0,
        passed: true,
        tags: Some(r#"["vdi", "login", "critical"]"#.to_string()),
        created_at: Utc::now(),
    };

    let report_id = storage.reports().create(&report).await?;
    println!("报告ID: {}", report_id);

    // 3. 创建执行步骤
    let steps = vec![
        ExecutionStepRecord {
            id: 0,
            report_id,
            step_index: 0,
            description: "打开登录页面".to_string(),
            status: "Success".to_string(),
            error: None,
            duration_ms: Some(500),
            output: Some("页面已加载".to_string()),
        },
        ExecutionStepRecord {
            id: 0,
            report_id,
            step_index: 1,
            description: "输入用户名和密码".to_string(),
            status: "Success".to_string(),
            error: None,
            duration_ms: Some(1000),
            output: Some("凭据已输入".to_string()),
        },
        ExecutionStepRecord {
            id: 0,
            report_id,
            step_index: 2,
            description: "点击登录按钮".to_string(),
            status: "Success".to_string(),
            error: None,
            duration_ms: Some(1500),
            output: Some("登录成功".to_string()),
        },
    ];

    storage.reports().create_steps(&steps).await?;
    println!("已保存 {} 个执行步骤", steps.len());

    Ok(())
}
```

### 示例2: 高级查询和统计

```rust
use atp_storage::{ReportFilter};
use chrono::{Utc, Duration};

async fn report_statistics(storage: &Storage) -> Result<(), Box<dyn std::error::Error>> {
    // 查询最近30天的失败测试
    let thirty_days_ago = Utc::now() - Duration::days(30);

    let filter = ReportFilter {
        passed: Some(false),
        start_time_from: Some(thirty_days_ago),
        limit: Some(100),
        ..Default::default()
    };

    let failed_reports = storage.reports().list(&filter).await?;
    println!("最近30天失败的测试: {} 个", failed_reports.len());

    // 计算特定场景的成功率
    let success_rate = storage.reports()
        .get_success_rate("用户登录测试", 30)
        .await?;
    println!("用户登录测试成功率: {:.2}%", success_rate);

    // 统计总报告数
    let total_filter = ReportFilter::default();
    let total_count = storage.reports().count(&total_filter).await?;
    println!("总测试报告数: {}", total_count);

    Ok(())
}
```

### 示例3: 场景管理

```rust
use atp_storage::ScenarioRecord;
use chrono::Utc;

async fn manage_scenarios(storage: &Storage) -> Result<(), Box<dyn std::error::Error>> {
    // 创建新场景
    let scenario = ScenarioRecord {
        id: 0,
        name: "桌面池创建测试".to_string(),
        description: Some("测试桌面池的创建流程".to_string()),
        definition: r#"{
            "steps": [
                {"action": "CreateDeskPool", "name": "test-pool"},
                {"action": "Wait", "duration": 5000},
                {"action": "VerifyDeskPoolStatus", "status": "Running"}
            ]
        }"#.to_string(),
        tags: Some(r#"["vdi", "desktop-pool"]"#.to_string()),
        version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let scenario_id = storage.scenarios().create(&scenario).await?;
    println!("场景ID: {}", scenario_id);

    // 更新场景定义
    let new_definition = r#"{
        "steps": [
            {"action": "CreateDeskPool", "name": "test-pool", "size": 10},
            {"action": "Wait", "duration": 10000},
            {"action": "VerifyDeskPoolStatus", "status": "Running"}
        ]
    }"#;

    storage.scenarios().update(scenario_id, new_definition).await?;
    println!("场景已更新(版本: 2)");

    // 查询场景
    let found = storage.scenarios().get_by_name("桌面池创建测试").await?;
    if let Some(s) = found {
        println!("场景版本: {}", s.version); // 应该是 2
    }

    Ok(())
}
```

### 示例4: 与 Executor 集成

```rust
use atp_executor::ScenarioRunner;
use atp_storage::{Storage, StorageManager};
use std::sync::Arc;

async fn run_scenario_with_database() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化数据库
    let storage_manager = StorageManager::new("~/.config/atp/data.db").await?;
    let storage = Arc::new(Storage::from_manager(&storage_manager));

    // 创建场景执行器(带数据库支持)
    let mut runner = ScenarioRunner::new(
        Arc::clone(&transport_manager),
        Arc::clone(&protocol_registry),
    ).with_storage(Arc::clone(&storage));

    // 运行场景 - 报告会自动保存到数据库
    let report = runner.run().await?;

    println!("场景执行完成!");
    println!("状态: {}", if report.success { "成功" } else { "失败" });
    println!("报告已自动保存到数据库");

    Ok(())
}
```

---

## 最佳实践

### 1. 连接管理

```rust
// ✅ 推荐: 使用 Arc 共享 Storage 实例
let storage = Arc::new(Storage::from_manager(&manager));

// ❌ 避免: 重复创建 Storage 实例
// 每次都创建新的会浪费资源
```

### 2. 错误处理

```rust
// ✅ 推荐: 优雅处理数据库错误
match storage.reports().create(&report).await {
    Ok(id) => println!("报告ID: {}", id),
    Err(StorageError::ConnectionError(e)) => {
        eprintln!("数据库连接失败: {}", e);
        // 降级策略: 保存到本地文件
    }
    Err(e) => eprintln!("保存失败: {}", e),
}

// ❌ 避免: 直接 panic
let id = storage.reports().create(&report).await.unwrap();
```

### 3. 批量操作

```rust
// ✅ 推荐: 使用批量创建方法
let steps = vec![step1, step2, step3];
storage.reports().create_steps(&steps).await?;

// ❌ 避免: 逐个创建(性能差)
for step in steps {
    storage.reports().create_step(&step).await?;
}
```

### 4. 查询优化

```rust
// ✅ 推荐: 使用分页
let filter = ReportFilter {
    limit: Some(100),
    offset: Some(0),
    ..Default::default()
};

// ❌ 避免: 一次性加载所有数据
let all_reports = storage.reports().list(&ReportFilter::default()).await?;
```

### 5. 数据库备份

```bash
# 定期备份数据库
cp ~/.config/atp/data.db ~/.config/atp/backups/data_$(date +%Y%m%d).db

# 或使用 SQLite 备份命令
sqlite3 ~/.config/atp/data.db ".backup ~/.config/atp/backups/data_$(date +%Y%m%d).db"
```

---

## 故障排查

### 问题1: 数据库文件权限错误

**错误信息**:
```
Error: ConnectionError("unable to open database file")
```

**解决方案**:
```bash
# 检查目录权限
ls -la ~/.config/atp/

# 修复权限
chmod 755 ~/.config/atp/
chmod 644 ~/.config/atp/data.db
```

### 问题2: 数据库锁定

**错误信息**:
```
Error: DatabaseError("database is locked")
```

**解决方案**:
```rust
// 增加连接池大小
let pool = SqlitePoolOptions::new()
    .max_connections(10)  // 增加到 10
    .connect(&db_url)
    .await?;
```

### 问题3: 迁移失败

**错误信息**:
```
Error: MigrationError("migration failed")
```

**解决方案**:
```bash
# 备份现有数据库
mv ~/.config/atp/data.db ~/.config/atp/data.db.bak

# 删除数据库,重新创建
rm ~/.config/atp/data.db

# 重新运行程序(会自动创建和迁移)
```

### 问题4: 查询性能慢

**症状**: 查询大量数据时响应缓慢

**解决方案**:
```rust
// 1. 使用分页
let filter = ReportFilter {
    limit: Some(50),
    ..Default::default()
};

// 2. 添加合适的索引(已在 schema 中定义)

// 3. 使用筛选条件减少数据量
let filter = ReportFilter {
    scenario_name: Some("specific_scenario".to_string()),
    start_time_from: Some(recent_date),
    ..Default::default()
};
```

---

## API 参考

### ReportFilter

```rust
pub struct ReportFilter {
    pub scenario_name: Option<String>,    // 场景名称筛选
    pub passed: Option<bool>,             // 成功/失败筛选
    pub start_time_from: Option<DateTime<Utc>>,  // 开始时间下限
    pub start_time_to: Option<DateTime<Utc>>,    // 开始时间上限
    pub tags: Option<Vec<String>>,        // 标签筛选(暂未实现)
    pub limit: Option<i64>,               // 限制返回数量
    pub offset: Option<i64>,              // 偏移量(分页)
}
```

### ScenarioFilter

```rust
pub struct ScenarioFilter {
    pub name: Option<String>,       // 名称模糊匹配(LIKE)
    pub tags: Option<Vec<String>>,  // 标签筛选(暂未实现)
    pub limit: Option<i64>,         // 限制返回数量
    pub offset: Option<i64>,        // 偏移量(分页)
}
```

### StorageError

```rust
pub enum StorageError {
    ConnectionError(String),    // 连接错误
    DatabaseError(sqlx::Error), // 数据库操作错误
    MigrationError(String),     // 迁移错误
    NotFound(String),           // 记录不存在
    AlreadyExists(String),      // 记录已存在
}
```

---

## CLI 命令

ATP CLI 提供了丰富的报告查询命令:

```bash
# 列出所有报告
atp report list

# 筛选特定场景
atp report list --scenario "用户登录测试"

# 只显示失败的报告
atp report list --failed

# 分页查询
atp report list --limit 20 --offset 40

# 显示报告详情
atp report show 123

# 导出报告为 JSON
atp report export 123 --format json --output report.json

# 导出为 YAML
atp report export 123 --format yaml --output report.yaml

# 删除报告
atp report delete 123

# 查看场景统计
atp report stats "用户登录测试" --days 30
```

---

## 性能指标

基于集成测试的性能数据:

- **单个报告创建**: < 10ms
- **批量创建100个步骤**: < 1000ms
- **查询报告(带筛选)**: < 20ms
- **并发创建10个报告**: < 100ms
- **成功率统计查询**: < 30ms

---

## 数据迁移

### 导出数据

```bash
# 导出为 SQL
sqlite3 ~/.config/atp/data.db .dump > backup.sql

# 导出为 CSV
sqlite3 -header -csv ~/.config/atp/data.db \
  "SELECT * FROM test_reports" > reports.csv
```

### 导入数据

```bash
# 从 SQL 导入
sqlite3 ~/.config/atp/data.db < backup.sql

# 从 CSV 导入
sqlite3 ~/.config/atp/data.db <<EOF
.mode csv
.import reports.csv test_reports
EOF
```

---

## 未来计划

- [ ] 支持标签筛选(Tag filtering)
- [ ] 添加全文搜索功能
- [ ] 实现报告归档功能(自动清理旧数据)
- [ ] 添加 HostRepository 和 MetricRepository
- [ ] 支持 PostgreSQL 后端
- [ ] 实现数据库备份和恢复工具
- [ ] 添加性能指标可视化

---

## 相关文档

- [数据库实现总结](DATABASE_INTEGRATION_SUMMARY.md)
- [数据库实现指南](DATABASE_IMPLEMENTATION.md)
- [Executor 使用指南](STAGE4_EXECUTOR_IMPLEMENTATION.md)
- [CLI 使用指南](STAGE5_CLI_IMPLEMENTATION.md)

---

**最后更新**: 2025-12-01
**维护者**: OCloudView ATP Team

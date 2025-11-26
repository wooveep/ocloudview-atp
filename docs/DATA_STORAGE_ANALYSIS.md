# 数据存储方式分析与建议

## 当前情况总结

### 1. **主机信息 (Host Information)**

#### 当前存储方式: ❌ **TOML 配置文件**

**位置**: `~/.config/atp/config.toml`

**数据结构**:
```rust
// CLI 配置 (atp-application/cli/src/config.rs)
pub struct CliConfig {
    pub hosts: HashMap<String, HostConfig>,  // 主机列表
    pub default_host: Option<String>,
    pub scenario_dir: Option<String>,
    pub version: String,
}

pub struct HostConfig {
    pub host: String,              // 主机地址
    pub uri: Option<String>,       // Libvirt URI
    pub tags: Vec<String>,         // 标签
    pub metadata: HashMap<String, String>,  // 元数据
}
```

**操作方式**:
- **添加主机**: `atp host add <id> <host> [--uri <uri>]`
  - 调用 `CliConfig::load()` 从 TOML 加载
  - 调用 `config.add_host()` 添加到内存 HashMap
  - 调用 `config.save()` 保存回 TOML 文件

- **列出主机**: `atp host list`
  - 从 TOML 文件加载并展示

- **删除主机**: `atp host remove <id>`
  - 从内存 HashMap 删除后保存回 TOML

**实现文件**:
- [config.rs](atp-application/cli/src/config.rs:10-169) - 配置管理
- [host.rs](atp-application/cli/src/commands/host.rs:1-92) - 主机命令

---

### 2. **虚拟机信息 (VM/Domain Information)**

#### 当前存储方式: ⚠️ **实时从 VDI 平台 API 查询 (无本地持久化)**

**数据来源**: VDI 平台 REST API

**数据结构**:
```rust
// VDI 平台数据模型 (atp-core/vdiplatform/src/models/mod.rs)
pub struct Domain {
    pub id: String,
    pub name: String,
    pub status: String,       // 运行时状态
    pub host_id: String,
    pub vcpu: u32,
    pub memory: u64,
    pub created_at: Option<String>,
}

pub struct DeskPool {
    pub id: String,
    pub name: String,
    pub status: String,
    pub template_id: String,
    pub vm_count: u32,
    pub created_at: Option<String>,
}
```

**操作方式**:
- 通过 `VdiClient` 实时调用 API:
  - `domain_api.list_domains()` - 查询虚拟机列表
  - `domain_api.get_domain(id)` - 获取单个虚拟机
  - `deskpool_api.list_pools()` - 查询桌面池

**特点**:
- ✅ 数据始终保持最新
- ❌ 无历史记录
- ❌ 离线无法查询
- ❌ 频繁 API 调用可能影响性能

**实现文件**:
- [models/mod.rs](atp-core/vdiplatform/src/models/mod.rs:1-180) - 数据模型
- [api/domain.rs](atp-core/vdiplatform/src/api/domain.rs) - Domain API
- [api/deskpool.rs](atp-core/vdiplatform/src/api/deskpool.rs) - 桌面池 API

---

### 3. **测试场景/任务信息 (Scenario/Task Information)**

#### 当前存储方式: ⚠️ **YAML/JSON 文件 (无数据库)**

**位置**:
- 示例场景: `examples/vdi-scenarios/*.yaml`
- 用户场景: 配置的 `scenario_dir` 目录

**数据结构**:
```rust
// 场景定义 (atp-core/orchestrator/src/scenario.rs)
pub struct TestScenario {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<TestStep>,
    pub tags: Vec<String>,
    pub timeout: Option<u64>,
}
```

**操作方式**:
- **运行场景**: `atp scenario run <file.yaml>`
  - 从 YAML/JSON 文件加载场景定义
  - 解析为 `Scenario` 对象
  - 由 `ScenarioRunner` 执行

- **列出场景**: `atp scenario list`
  - 扫描场景目录的 YAML/JSON 文件

**特点**:
- ✅ 版本控制友好 (Git)
- ✅ 人类可读
- ❌ 无法快速搜索/过滤
- ❌ 无元数据查询
- ❌ 无执行历史关联

**实现文件**:
- [scenario.rs](atp-core/orchestrator/src/scenario.rs:1-200) - 场景定义
- [scenario.rs](atp-application/cli/src/commands/scenario.rs:1-130) - 场景命令

---

### 4. **测试执行报告 (Test Execution Reports)**

#### 当前存储方式: ❌ **仅内存 + 可选导出 JSON/YAML (无数据库)**

**数据结构**:
```rust
// 执行报告 (atp-core/executor/src/runner.rs)
pub struct ExecutionReport {
    pub scenario_name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub passed: bool,
    pub steps_executed: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub duration_ms: u64,
    pub steps: Vec<StepReport>,
}
```

**操作方式**:
- 场景执行完成后返回 `ExecutionReport`
- 可选导出为 JSON/YAML 文件
- **重启后丢失所有历史数据**

**特点**:
- ❌ 无持久化
- ❌ 无历史查询
- ❌ 无趋势分析
- ❌ 无聚合统计

---

## 问题诊断

### ❌ **都不在数据库中**

| 数据类型 | 当前存储 | 是否在数据库 | 状态 |
|---------|---------|------------|------|
| 主机信息 | TOML 文件 | ❌ | 配置文件 |
| VM 信息 | VDI API (实时) | ❌ | 无本地存储 |
| 测试场景 | YAML/JSON 文件 | ❌ | 文件系统 |
| 执行报告 | 内存 (临时) | ❌ | 未持久化 |

### 已实现的数据库层

虽然已经创建了数据库模块 (`atp-core/storage/`),但**所有集成工作都是 TODO 状态**:

- ✅ 数据库 Schema 已定义 (5 张表)
- ✅ Repository 已实现 (ReportRepository, ScenarioRepository)
- ❌ **但没有任何代码实际使用数据库**

**数据库表现状**:
```sql
-- 已定义但未使用的表
test_reports         -- 用于存储执行报告 (待集成)
execution_steps      -- 用于存储步骤详情 (待集成)
scenarios            -- 用于存储场景定义 (待集成)
hosts                -- 用于存储主机配置 (待集成)
connection_metrics   -- 用于存储性能指标 (待集成)
```

---

## 建议方案

### 建议 1: **主机信息** - 保持 TOML 文件 ✅

**理由**:
- 主机数量通常较少 (<50 台)
- 配置变更频率低
- TOML 文件便于手动编辑和版本控制
- 无复杂查询需求

**当前实现**: 合理,无需修改

**可选优化**:
- 如果未来主机数 >100,可考虑迁移到数据库

---

### 建议 2: **VM 信息** - 添加数据库缓存层 🔄

**问题**:
- 当前每次查询都调用 VDI API
- 无历史状态记录
- 离线无法查询

**建议**: 实现**缓存 + 定期同步**模式

#### 实现方案:

1. **添加 VM 信息表到数据库**:

```sql
-- 新增表 (需要添加到 migrations/002_vm_cache.sql)
CREATE TABLE vm_cache (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    host_id TEXT NOT NULL,
    vcpu INTEGER,
    memory INTEGER,
    vdi_pool_id TEXT,
    last_synced_at DATETIME NOT NULL,
    metadata TEXT,  -- JSON
    FOREIGN KEY (host_id) REFERENCES hosts(id)
);

-- 历史状态表 (可选,用于趋势分析)
CREATE TABLE vm_status_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vm_id TEXT NOT NULL,
    status TEXT NOT NULL,
    timestamp DATETIME NOT NULL,
    FOREIGN KEY (vm_id) REFERENCES vm_cache(id)
);
```

2. **实现混合查询策略**:

```rust
// atp-core/vdiplatform/src/cache.rs (新文件)
pub struct VmCacheManager {
    vdi_client: VdiClient,
    storage: Storage,
    cache_ttl: Duration,  // 缓存有效期 (如 5 分钟)
}

impl VmCacheManager {
    // 查询 VM (优先从缓存)
    pub async fn get_vm(&self, vm_id: &str) -> Result<Domain> {
        // 1. 先从数据库缓存查询
        if let Some(cached) = self.storage.vms().get_by_id(vm_id).await? {
            if cached.last_synced_at + self.cache_ttl > Utc::now() {
                return Ok(cached.into());  // 缓存未过期,直接返回
            }
        }

        // 2. 缓存过期或不存在,从 VDI API 查询
        let vm = self.vdi_client.domain_api().get_domain(vm_id).await?;

        // 3. 更新缓存
        self.storage.vms().upsert(&vm).await?;

        Ok(vm)
    }

    // 强制刷新缓存
    pub async fn sync_all_vms(&self) -> Result<()> {
        let vms = self.vdi_client.domain_api().list_domains().await?;
        for vm in vms {
            self.storage.vms().upsert(&vm).await?;
        }
        Ok(())
    }
}
```

**优点**:
- ✅ 减少 API 调用频率
- ✅ 离线查询支持
- ✅ 可记录历史状态
- ✅ 提升查询性能

**使用场景**:
```bash
# 查询 VM (从缓存)
atp vm list

# 强制刷新缓存
atp vm sync

# 查看 VM 历史状态
atp vm history <vm-id>
```

---

### 建议 3: **测试场景** - 双轨制存储 🔄

**问题**:
- YAML 文件无法快速搜索
- 无版本管理
- 无执行统计关联

**建议**: **YAML 文件 + 数据库** 双轨制

#### 实现方案:

**工作流**:
1. **开发期**: 使用 YAML 文件编写和维护场景
2. **导入期**: 将 YAML 场景导入数据库
3. **执行期**: 优先从数据库加载场景
4. **导出期**: 可将数据库场景导出为 YAML

**CLI 命令扩展**:

```bash
# 导入场景到数据库
atp scenario import ./examples/vdi-scenarios/

# 从数据库列出场景
atp scenario list

# 搜索场景
atp scenario search --tag smoke --name "login"

# 运行场景 (优先从数据库,fallback 到文件)
atp scenario run test_scenario

# 导出场景为 YAML
atp scenario export test_scenario --output scenario.yaml
```

**实现代码** (已有 TODO 注释):

```rust
// atp-application/cli/src/commands/scenario.rs

async fn import_scenarios(dir: &str) -> Result<()> {
    let storage_manager = StorageManager::new("~/.config/atp/data.db").await?;
    let storage = Storage::from_manager(&storage_manager);

    // 扫描目录
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension() == Some("yaml") {
            let scenario = Scenario::from_yaml_file(&path)?;

            // 转换为 ScenarioRecord
            let record = ScenarioRecord {
                id: 0,
                name: scenario.name.clone(),
                description: scenario.description.clone(),
                definition: std::fs::read_to_string(&path)?,
                tags: Some(serde_json::to_string(&scenario.tags)?),
                version: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            storage.scenarios().create(&record).await?;
            println!("✓ 导入场景: {}", scenario.name);
        }
    }

    Ok(())
}
```

**优点**:
- ✅ 保留 YAML 文件的版本控制优势
- ✅ 数据库提供快速搜索和过滤
- ✅ 场景与执行报告关联
- ✅ 支持场景版本管理

---

### 建议 4: **执行报告** - **立即启用数据库** ⚠️ 高优先级

**问题**:
- 当前执行报告完全不持久化
- 无法查询历史执行记录
- 无法统计成功率

**建议**: **立即启用数据库保存**

#### 实施步骤:

**已完成**:
- ✅ 数据库表已定义 (`test_reports`, `execution_steps`)
- ✅ Repository 已实现 (`ReportRepository`)
- ✅ 完整的集成代码已写在 TODO 注释中

**待完成** (约 30 分钟工作):

1. **启用 Executor 集成**:

```bash
# 1. 添加依赖
cd atp-core/executor
# 在 Cargo.toml 添加: atp-storage = { path = "../storage" }

# 2. 取消注释 runner.rs 的 TODO 代码
# - ScenarioRunner 添加 storage 字段
# - run() 方法添加保存逻辑
# - 取消注释 save_report_to_db() 方法
```

参考位置: [runner.rs:29-277](atp-core/executor/src/runner.rs:29-277)

2. **启用 CLI 报告命令**:

```bash
# 1. 添加依赖
cd atp-application/cli
# 在 Cargo.toml 添加: atp-storage = { path = "../../atp-core/storage" }

# 2. 取消注释 commands/mod.rs
# 取消注释: pub mod report;

# 3. 在 main.rs 添加 Report 枚举
```

参考位置: [report.rs](atp-application/cli/src/commands/report.rs:1-302)

**效果**:

```bash
# 查看最近 10 次测试报告
atp report list --limit 10

# 查看指定报告详情
atp report show 42

# 查看场景成功率统计
atp report stats test_scenario --days 30

# 导出报告
atp report export 42 --output report.json
```

---

## 总体建议优先级

| 数据类型 | 当前状态 | 建议方案 | 优先级 | 工作量 |
|---------|---------|---------|--------|--------|
| **执行报告** | 不持久化 | 立即启用数据库 | 🔥 **极高** | 30 分钟 |
| **主机信息** | TOML 文件 | 保持现状 | ✅ 无需改动 | 0 |
| **测试场景** | YAML 文件 | 双轨制 (YAML + DB) | 🟡 中 | 2-3 小时 |
| **VM 信息** | VDI API | 添加缓存层 | 🟢 低 | 4-6 小时 |

---

## 实施建议

### 阶段 1: 立即执行 (本周)
✅ **启用执行报告数据库存储**
- 取消注释 Executor 和 CLI 的 TODO 代码
- 测试验证
- 预期收益: 完整的执行历史和趋势分析

### 阶段 2: 短期优化 (2 周内)
🟡 **场景导入/导出功能**
- 实现 `atp scenario import` 命令
- 实现数据库场景加载
- 保留 YAML 文件作为备份

### 阶段 3: 中期扩展 (1 个月内)
🟢 **VM 信息缓存**
- 添加 VM 缓存表
- 实现混合查询策略
- 添加同步命令

---

## 结论

**当前状态**: ❌ **所有数据都不在数据库中**

| 存储位置 | 数据类型 |
|---------|---------|
| TOML 文件 | 主机配置 |
| VDI API (实时) | VM 信息 |
| YAML 文件 | 测试场景 |
| 内存 (临时) | 执行报告 |
| **数据库** | **空 (未使用)** |

**最紧急问题**: 执行报告完全不持久化,导致无法进行历史分析和趋势跟踪。

**建议优先启用**: 执行报告的数据库存储 (已有完整实现代码,只需取消 TODO 注释)。

---

**文档版本**: v1.0
**分析日期**: 2025-11-25
**分析师**: Claude (ATP Team)

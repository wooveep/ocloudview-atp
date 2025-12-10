# Executor/Orchestrator 统一实施计划

**项目**: OCloudView ATP
**日期**: 2025-12-01
**预计工期**: 9-14 天
**优先级**: 🟡 中

---

## 快速摘要

**问题**: 项目中存在两个功能重叠的执行引擎
- **Executor**: 协议集成完整，数据库支持，代码质量高
- **Orchestrator**: VDI 集成，但协议未实现，技术债务多

**方案**: 以 **Executor 为主引擎**，废弃 Orchestrator

**收益**:
- ✅ 消除 40% 重复代码
- ✅ 减少维护成本
- ✅ 统一架构标准
- ✅ 提升代码质量

---

## 实施阶段

### 阶段 1: 扩展 Executor (2-3 天) 🔥

**目标**: 添加 VDI 平台操作支持

#### 任务清单

- [ ] **T1.1**: 扩展 Action 枚举（1 天）
  ```rust
  // 添加到 atp-core/executor/src/scenario.rs
  pub enum Action {
      // ... 现有操作

      // 新增：VDI 平台操作
      VdiCreateDeskPool { name, template_id, count },
      VdiEnableDeskPool { pool_id },
      VdiDisableDeskPool { pool_id },
      VdiStartDomain { domain_id },
      VdiShutdownDomain { domain_id },
      VdiRebootDomain { domain_id },
      VdiDeleteDomain { domain_id },
      VdiBindUser { domain_id, user_id },
      VdiGetDeskPoolDomains { pool_id },
  }
  ```

- [ ] **T1.2**: 添加 VdiClient 到 ScenarioRunner（0.5 天）
  ```rust
  // 修改 atp-core/executor/src/runner.rs
  pub struct ScenarioRunner {
      // ... 现有字段
      vdi_client: Option<Arc<VdiClient>>,  // 新增
  }

  impl ScenarioRunner {
      pub fn with_vdi_client(mut self, client: Arc<VdiClient>) -> Self {
          self.vdi_client = Some(client);
          self
      }
  }
  ```

- [ ] **T1.3**: 实现 VDI 操作执行方法（1 天）
  - `execute_vdi_create_desk_pool()`
  - `execute_vdi_enable_desk_pool()`
  - `execute_vdi_start_domain()`
  - `execute_vdi_shutdown_domain()`
  - 其他 VDI 操作...

- [ ] **T1.4**: 更新 execute_action() 分发逻辑（0.5 天）
  ```rust
  async fn execute_action(&mut self, action: &Action, index: usize) -> Result<StepReport> {
      match action {
          // 现有协议操作
          Action::SendKey { .. } => { /* ... */ }

          // 新增：VDI 操作
          Action::VdiEnableDeskPool { pool_id } => {
              self.execute_vdi_enable_desk_pool(pool_id, index).await
          }
          // ...
      }
  }
  ```

**验收标准**:
- ✅ 所有 VDI 操作编译通过
- ✅ 至少实现 EnableDeskPool、StartDomain、ShutdownDomain
- ✅ 单元测试覆盖新功能

---

### 阶段 2: 实现验证功能 (2-3 天) 🟡

**目标**: 添加验证步骤支持

#### 任务清单

- [ ] **T2.1**: 添加验证动作（0.5 天）
  ```rust
  pub enum Action {
      // ...

      // 新增：验证步骤
      VerifyDomainStatus {
          domain_id: String,
          expected_status: String,
          timeout_secs: Option<u64>,
      },
      VerifyAllDomainsRunning {
          pool_id: String,
          timeout_secs: Option<u64>,
      },
      VerifyCommandSuccess {
          domain_id: String,
          timeout_secs: Option<u64>,
      },
  }
  ```

- [ ] **T2.2**: 实现验证方法（2 天）
  - `verify_domain_status()` - 查询虚拟机状态
  - `verify_all_domains_running()` - 检查桌面池所有虚拟机
  - `verify_command_success()` - 检查命令执行结果

- [ ] **T2.3**: 添加超时和重试逻辑（0.5 天）
  ```rust
  async fn verify_with_retry<F>(
      &self,
      verify_fn: F,
      timeout: Duration,
      interval: Duration,
  ) -> Result<StepReport>
  where
      F: Fn() -> Future<Output = Result<bool>>,
  {
      let start = Instant::now();
      loop {
          if verify_fn().await? {
              return Ok(StepReport::success(...));
          }

          if start.elapsed() > timeout {
              return Ok(StepReport::failed(...));
          }

          tokio::time::sleep(interval).await;
      }
  }
  ```

**验收标准**:
- ✅ 验证功能编译通过
- ✅ 支持超时和轮询
- ✅ 单元测试覆盖

---

### 阶段 3: 合并测试 (1 天) 🟡

**目标**: 将 Orchestrator 的测试迁移到 Executor

#### 任务清单

- [ ] **T3.1**: 复制测试文件（0.5 天）
  ```bash
  # 复制测试
  cp atp-core/orchestrator/tests/*.rs atp-core/executor/tests/

  # 重命名避免冲突
  mv atp-core/executor/tests/orchestrator_tests.rs \
     atp-core/executor/tests/vdi_tests.rs
  ```

- [ ] **T3.2**: 更新测试导入和类型（0.5 天）
  ```rust
  // 原来
  use atp_orchestrator::{TestScenario, TestStep, VdiAction};

  // 改为
  use atp_executor::{Scenario, Action};
  ```

- [ ] **T3.3**: 适配测试用例
  - 更新动作类型名称
  - 更新场景结构
  - 确保所有测试通过

- [ ] **T3.4**: 添加新的集成测试
  ```rust
  #[tokio::test]
  async fn test_vdi_and_protocol_integration() {
      let scenario = Scenario {
          name: "VDI 集成测试".to_string(),
          steps: vec![
              // VDI 操作
              Action::VdiEnableDeskPool { pool_id: "pool-1".to_string() },
              // 协议操作
              Action::SendKey { key: "enter".to_string() },
              // 验证
              Action::VerifyDomainStatus { ... },
          ],
      };

      let mut runner = ScenarioRunner::new(/* ... */);
      let report = runner.run(&scenario).await.unwrap();
      assert!(report.passed);
  }
  ```

**验收标准**:
- ✅ 所有原有测试通过
- ✅ 新增的集成测试通过
- ✅ 测试覆盖率不低于 80%

---

### 阶段 4: 清理和文档 (1 天) 🟢

**目标**: 移除 Orchestrator，更新文档

#### 任务清单

- [ ] **T4.1**: 备份 Orchestrator（0.1 天）
  ```bash
  git checkout -b backup/orchestrator
  git push origin backup/orchestrator
  ```

- [ ] **T4.2**: 移除 Orchestrator 模块（0.2 天）
  ```bash
  rm -rf atp-core/orchestrator

  # 更新 workspace Cargo.toml
  sed -i '/orchestrator/d' Cargo.toml

  # 更新其他依赖
  grep -r "atp-orchestrator" . --exclude-dir=.git | \
      xargs sed -i 's/atp-orchestrator/atp-executor/g'
  ```

- [ ] **T4.3**: 更新文档（0.5 天）
  - ✏️ 更新 [TODO.md](../TODO.md)
  - ✏️ 更新 [README.md](../README.md)
  - ✏️ 创建迁移说明文档
  - ✏️ 更新 [STAGE4_EXECUTOR_IMPLEMENTATION.md](STAGE4_EXECUTOR_IMPLEMENTATION.md)

- [ ] **T4.4**: 创建迁移指南（0.2 天）
  ```markdown
  # 从 Orchestrator 迁移到 Executor 指南

  ## 场景文件迁移

  ### 原 Orchestrator 场景
  ```yaml
  name: "测试场景"
  steps:
    - type: vdi_action
      action: enable_desk_pool
      pool_id: "pool-123"
  ```

  ### 新 Executor 场景
  ```yaml
  name: "测试场景"
  steps:
    - action:
        type: vdi_enable_desk_pool
        pool_id: "pool-123"
  ```
  ```

**验收标准**:
- ✅ Orchestrator 完全移除
- ✅ 编译无警告
- ✅ 文档更新完整
- ✅ 提供迁移指南

---

## 时间表

| 阶段 | 开始日期 | 结束日期 | 工作日 | 责任人 |
|-----|---------|---------|--------|--------|
| 阶段 1 | Day 1 | Day 3 | 2-3 天 | TBD |
| 阶段 2 | Day 4 | Day 6 | 2-3 天 | TBD |
| 阶段 3 | Day 7 | Day 7 | 1 天 | TBD |
| 阶段 4 | Day 8 | Day 8 | 1 天 | TBD |
| **缓冲** | Day 9 | Day 10 | 2 天 | - |
| **总计** | | | **9-14 天** | |

---

## 风险管理

### 高风险项

| 风险 | 概率 | 影响 | 缓解措施 |
|-----|------|------|---------|
| VDI API 不兼容 | 中 | 高 | 提前测试 VdiClient 集成 |
| 测试失败 | 中 | 中 | 每阶段验证，及时修复 |
| 功能遗漏 | 低 | 高 | 详细功能清单对照 |

### 回滚计划

如果遇到重大问题：

```bash
# 恢复 Orchestrator
git checkout backup/orchestrator
git cherry-pick atp-core/orchestrator

# 保留 Executor 改进
git checkout main
git merge --squash feature/unified-executor

# 重新评估
```

---

## 验收标准

### 功能验收

- [ ] 所有原 Executor 功能正常
- [ ] 所有原 Orchestrator 的 VDI 功能已迁移
- [ ] 验证功能正常工作
- [ ] 数据库持久化正常
- [ ] 测试覆盖率 ≥ 80%

### 性能验收

- [ ] 场景执行时间无明显增加
- [ ] 内存占用无明显增加
- [ ] 编译时间无明显增加

### 代码质量

- [ ] `cargo clippy` 无警告
- [ ] `cargo test` 全部通过
- [ ] 代码审查通过

---

## 成功指标

| 指标 | 目标 | 测量方法 |
|-----|------|---------|
| 代码行数减少 | -30% | `wc -l` 对比 |
| 重复代码消除 | 100% | 移除整个 orchestrator |
| 测试覆盖率 | ≥80% | `cargo tarpaulin` |
| 功能完整性 | 100% | 功能清单验证 |
| 文档完整性 | 100% | 文档审查 |

---

## 下一步行动

### 立即执行（本周）

1. ✅ 创建功能分支
   ```bash
   git checkout -b feature/unified-executor
   ```

2. ✅ 备份 Orchestrator
   ```bash
   git checkout -b backup/orchestrator
   git push origin backup/orchestrator
   ```

3. ✅ 开始阶段 1.1：扩展 Action 枚举

### 本月完成

4. ⏳ 完成阶段 1-4 所有任务
5. ⏳ 合并到 main 分支
6. ⏳ 发布 v0.3.2

---

## 相关文档

- 📄 [详细分析文档](EXECUTOR_ORCHESTRATOR_ANALYSIS.md) - 完整对比分析
- 📄 [TODO.md](../TODO.md) - 项目任务清单
- 📄 [STAGE4_EXECUTOR_IMPLEMENTATION.md](STAGE4_EXECUTOR_IMPLEMENTATION.md) - Executor 实现
- 📄 [VDI_PLATFORM_TESTING.md](VDI_PLATFORM_TESTING.md) - VDI 平台测试

---

**创建日期**: 2025-12-01
**最后更新**: 2025-12-01
**状态**: 📋 待执行
**审批**: 待批准

---

**变更历史**:
- 2025-12-01: 初始版本

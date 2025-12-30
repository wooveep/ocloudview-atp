# OCloudView ATP 文档中心

## 项目概览

**OCloudView ATP** (Automated Testing Platform) 是一个用于虚拟化环境的输入自动化测试和验证平台。

| 项目属性 | 值 |
|---------|-----|
| 当前版本 | v0.4.1 |
| 整体进度 | 87% |
| 代码行数 | ~15,700+ 行 |
| 文档数量 | 44 个 |
| 测试用例 | 158 个 |
| 最后更新 | 2025-12-31 |

---

## 文档导航

### 1. 快速开始

| 文档 | 说明 |
|------|------|
| [快速开始指南](QUICKSTART.md) | 环境准备、项目构建、运行示例 |
| [开发指南](DEVELOPMENT.md) | 开发环境搭建、代码规范、调试技巧 |
| [测试配置指南](TESTING_CONFIG_GUIDE.md) | test.toml 配置、环境变量说明 |

---

### 2. 架构设计

| 文档 | 说明 | 优先级 |
|------|------|--------|
| [分层架构设计](LAYERED_ARCHITECTURE.md) | 系统整体分层架构、组件说明 | ⭐⭐⭐ 必读 |
| [连接模式设计](CONNECTION_MODES.md) | Libvirt复用模式、SPICE多通道模式 | ⭐⭐ |
| [Guest验证服务器设计](GUEST_VERIFICATION_SERVER_DESIGN.md) | 验证服务器架构、VM ID路由 | ⭐⭐ |
| [VM ID映射方案](VM_ID_MAPPING_SOLUTIONS.md) | VDI与libvirt虚拟机ID映射 | ⭐ |
| [数据存储分析](DATA_STORAGE_ANALYSIS.md) | 存储层需求分析和设计 | ⭐ |

---

### 3. 实现总结 (按阶段)

| 阶段 | 文档 | 完成度 |
|------|------|--------|
| 阶段1 | [传输层实现](STAGE1_TRANSPORT_IMPLEMENTATION.md) | 85% |
| 阶段2 | [协议层实现](STAGE2_PROTOCOL_IMPLEMENTATION.md) | 70% |
| 阶段4 | [执行器实现](STAGE4_EXECUTOR_IMPLEMENTATION.md) | 85% |
| 阶段5 | [CLI实现](STAGE5_CLI_IMPLEMENTATION.md) | 90% |
| 阶段8 | [测试框架](STAGE8_TESTING.md) | 65% |

---

### 4. 协议与技术指南

#### 4.1 协议实现

| 文档 | 说明 |
|------|------|
| [QGA使用指南](QGA_GUIDE.md) | QEMU Guest Agent命令和最佳实践 |
| [VirtIO Serial指南](VIRTIO_SERIAL_GUIDE.md) | 自定义协议开发指南 |
| [SPICE协议实现](SPICE_PROTOCOL_IMPLEMENTATION.md) | SPICE多通道架构和实现 |
| [USB重定向实现指南](USB_REDIRECTION_IMPLEMENTATION_GUIDE.md) | USB重定向协议详解 |
| [鼠标操作指南](MOUSE_OPERATIONS_GUIDE.md) | SPICE鼠标集成和QGA备用方案 |
| [SPICE鼠标集成总结](SPICE_MOUSE_INTEGRATION_SUMMARY.md) | SPICE鼠标实现总结 |

#### 4.2 数据库与存储

| 文档 | 说明 |
|------|------|
| [数据库实现设计](DATABASE_IMPLEMENTATION.md) | SQLite架构和Schema设计 |
| [数据库使用指南](DATABASE_USAGE_GUIDE.md) | 报告查询、备份恢复操作 |
| [数据库集成总结](DATABASE_INTEGRATION_SUMMARY.md) | Executor和CLI集成详情 |

#### 4.3 测试配置

| 文档 | 说明 |
|------|------|
| [测试配置指南](TESTING_CONFIG_GUIDE.md) | 完整的测试配置指南 |
| [测试配置实现](TEST_CONFIG_IMPLEMENTATION.md) | TestConfig模块设计 |
| [测试配置实现总结](TEST_CONFIG_IMPLEMENTATION_SUMMARY.md) | 配置加载模块总结 |
| [E2E测试指南](E2E_TESTING_GUIDE.md) | 端到端测试配置和运行 |
| [E2E测试总结](E2E_TESTING_SUMMARY.md) | E2E测试框架实现总结 |

---

### 5. VDI平台集成

| 文档 | 说明 |
|------|------|
| [VDI平台测试设计](VDI_PLATFORM_TESTING.md) | VDI API客户端和场景编排 |
| [VDI+libvirt集成报告](VDI_LIBVIRT_INTEGRATION.md) | 完整集成测试报告 |
| [VDI登录API指南](VDI_LOGIN_API_GUIDE.md) | MD5密码加密和Token认证 |
| [VDI API发现](VDI_API_DISCOVERY.md) | Swagger API文档解析 |
| [CLI VDI命令](CLI_VDI_COMMANDS.md) | atp vdi 命令使用指南 |
| [连通性测试指南](CONNECTIVITY_TEST_GUIDE.md) | VDI和libvirt连通性测试 |

---

### 6. Guest验证器

| 文档 | 说明 | 平台 |
|------|------|------|
| [Guest验证器总结](GUEST_VERIFICATION_SUMMARY.md) | 验证器整体架构和功能 | 通用 |
| [Windows验证器实现](WINDOWS_VERIFIER_IMPLEMENTATION.md) | Hook API实现详解 | Windows |
| [Windows验证器部署](WINDOWS_VERIFIER_DEPLOYMENT.md) | 编译、安装、配置 | Windows |
| [Windows验证器总结](WINDOWS_VERIFIER_SUMMARY.md) | Windows实现总结 | Windows |

---

### 7. 项目分析与规划

| 文档 | 说明 |
|------|------|
| [项目完成度分析](PROJECT_COMPLETION_ANALYSIS.md) | 各模块完成度评估 |
| [Executor/Orchestrator分析](EXECUTOR_ORCHESTRATOR_ANALYSIS.md) | 执行器合并分析 |
| [迁移计划](EXECUTOR_ORCHESTRATOR_MIGRATION_PLAN.md) | Orchestrator到Executor迁移 |

---

### 8. 部署指南

| 文档 | 说明 |
|------|------|
| [便携式构建指南](deployment/PORTABLE_BUILD_GUIDE.md) | 便携式二进制打包 |
| [GLIBC兼容性指南](deployment/GLIBC_COMPATIBILITY_GUIDE.md) | 跨系统部署兼容性 |

---

## 推荐阅读路径

### 新手入门 (30分钟)

```
1. QUICKSTART.md          → 环境准备和快速运行
2. LAYERED_ARCHITECTURE.md → 理解系统架构
3. CLI_VDI_COMMANDS.md     → 使用CLI工具
```

### 开发者指南 (2小时)

```
1. LAYERED_ARCHITECTURE.md       → 整体架构
2. CONNECTION_MODES.md           → 连接机制
3. STAGE1_TRANSPORT_IMPLEMENTATION.md → 传输层
4. STAGE2_PROTOCOL_IMPLEMENTATION.md  → 协议层
5. DEVELOPMENT.md                → 开发规范
```

### 协议开发 (3小时)

```
1. QGA_GUIDE.md                  → QGA基础
2. VIRTIO_SERIAL_GUIDE.md        → 自定义协议
3. SPICE_PROTOCOL_IMPLEMENTATION.md → SPICE协议
4. USB_REDIRECTION_IMPLEMENTATION_GUIDE.md → USB重定向
```

### 测试开发 (2小时)

```
1. TESTING_CONFIG_GUIDE.md       → 测试配置
2. E2E_TESTING_GUIDE.md          → E2E测试
3. STAGE8_TESTING.md             → 测试框架
```

### VDI平台集成 (1小时)

```
1. VDI_PLATFORM_TESTING.md       → VDI设计
2. VDI_LOGIN_API_GUIDE.md        → 登录认证
3. CLI_VDI_COMMANDS.md           → CLI使用
```

---

## 模块级文档

项目中各模块也包含独立的README文档：

| 路径 | 说明 |
|------|------|
| [atp-core/verification-server/README.md](../atp-core/verification-server/README.md) | 验证服务器文档 |
| [atp-core/executor/examples/scenarios/README.md](../atp-core/executor/examples/scenarios/README.md) | E2E测试场景说明 |
| [guest-verifier/README.md](../guest-verifier/README.md) | Guest验证器文档 |
| [examples/vdi-scenarios/README.md](../examples/vdi-scenarios/README.md) | VDI场景示例说明 |

---

## 根目录文档

| 文档 | 说明 |
|------|------|
| [README.md](../README.md) | 项目主文档 |
| [TODO.md](../TODO.md) | 开发任务清单 |
| [CHANGELOG.md](../CHANGELOG.md) | 版本历史和变更日志 |

---

## 文档统计

| 类别 | 数量 | 行数 |
|------|------|------|
| 架构设计 | 5 | ~3,000 |
| 实现总结 | 5 | ~2,500 |
| 技术指南 | 15 | ~8,000 |
| VDI集成 | 6 | ~3,000 |
| 验证器 | 4 | ~2,000 |
| 项目分析 | 4 | ~3,000 |
| 部署指南 | 2 | ~500 |
| **总计** | **41** | **~22,000** |

---

## 文档贡献

### 文档规范

- 使用 Markdown 格式
- 表格用于结构化信息
- 代码块标注语言类型
- 及时更新版本和日期

### 命名规范

| 类型 | 命名格式 | 示例 |
|------|----------|------|
| 实现总结 | STAGE{N}_{MODULE}_IMPLEMENTATION.md | STAGE1_TRANSPORT_IMPLEMENTATION.md |
| 使用指南 | {MODULE}_GUIDE.md | QGA_GUIDE.md |
| 设计文档 | {FEATURE}_DESIGN.md | CONNECTION_MODES.md |
| 分析报告 | {TOPIC}_ANALYSIS.md | PROJECT_COMPLETION_ANALYSIS.md |

---

**最后更新**: 2025-12-31
**维护者**: OCloudView ATP Team
**项目版本**: v0.4.1

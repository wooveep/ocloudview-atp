# OCloudView ATP 文档中心

## 📚 文档导航

欢迎来到 OCloudView ATP（自动化测试平台）的文档中心。本目录包含了项目的架构设计、开发指南、实现总结等完整文档。

---

## 🏗️ 架构设计

### 核心架构文档
- **[分层架构设计](LAYERED_ARCHITECTURE.md)** ⭐ 推荐首先阅读
  - 系统整体分层架构
  - 各层职责和组件说明
  - 数据流向和交互关系
  - 项目目录结构

- **[连接模式设计](CONNECTION_MODES.md)**
  - Libvirt 复用模式（QMP/QGA/VirtioSerial）
  - Spice 独立多通道模式
  - URI 格式和配置

### VDI 平台集成
- **[VDI 平台测试设计](VDI_PLATFORM_TESTING.md)**
  - VDI 平台 API 客户端
  - 场景编排器设计
  - 端到端集成测试
  - 使用示例

---

## 📖 开发指南

### 快速上手
- **[快速开始指南](QUICKSTART.md)**
  - 环境准备
  - 项目构建
  - 运行示例
  - 常见问题

- **[开发指南](DEVELOPMENT.md)**
  - 开发环境搭建
  - 代码规范
  - 测试方法
  - 调试技巧

### 协议使用指南
- **[QGA 使用指南](QGA_GUIDE.md)**
  - QEMU Guest Agent 介绍
  - 常用命令示例
  - 最佳实践

---

## ✅ 实现总结

按阶段记录了详细的实现过程、技术挑战和解决方案：

- **[阶段1: 传输层实现](STAGE1_TRANSPORT_IMPLEMENTATION.md)**
  - 连接管理（自动重连、心跳检测）
  - 连接池（策略、扩缩容、监控）
  - 传输管理器（并发执行、负载均衡）
  - 代码统计：~1,310 行

- **[阶段2: 协议层实现](STAGE2_PROTOCOL_IMPLEMENTATION.md)**
  - QMP 协议（Unix Socket、键盘输入）
  - QGA 协议（命令执行、输出捕获）
  - 协议抽象和注册中心
  - 代码统计：~1,100 行

---

## 📋 项目管理

- **[开发 TODO 清单](../TODO.md)**
  - 开发任务清单
  - 进度跟踪
  - 优先级标记
  - 代码统计

---

## 🔧 API 参考

### VDI 平台 API
- **[OCloudView 9.0 API 规范](Ocloud%20View%209.0接口文档_OpenAPI.json)** (OpenAPI 3.0)
- **[OCloudView API 文档](Ocloud%20View接口文档.doc)** (Word 文档)

---

## 📁 归档文档

以下文档已被新文档替代或不再维护，仅作参考：

<details>
<summary>点击展开查看归档文档列表</summary>

### 已归档的架构文档
- **[旧架构设计](archive/ARCHITECTURE.md)**
  - 原始的控制端-代理端架构
  - 已被分层架构替代

- **[架构更新总结](archive/ARCHITECTURE_UPDATE_SUMMARY.md)**
  - 架构演进过程
  - 已整合到最新架构文档

- **[重构计划](archive/REFACTORING_PLAN.md)**
  - 项目重构规划
  - 重构已完成

### 已归档的总结文档
- **[VDI 平台测试更新](archive/VDI_PLATFORM_TESTING_UPDATE.md)**
  - 已合并到 VDI_PLATFORM_TESTING.md

- **[项目结构总结](archive/PROJECT_STRUCTURE_SUMMARY.md)**
  - 已整合到分层架构文档

</details>

---

## 🎯 推荐阅读路径

### 新手入门
1. [快速开始指南](QUICKSTART.md) - 了解如何运行项目
2. [分层架构设计](LAYERED_ARCHITECTURE.md) - 理解系统架构
3. [开发指南](DEVELOPMENT.md) - 开始开发

### 深入理解
1. [分层架构设计](LAYERED_ARCHITECTURE.md) - 完整架构
2. [连接模式设计](CONNECTION_MODES.md) - 连接机制
3. [VDI 平台测试设计](VDI_PLATFORM_TESTING.md) - VDI 集成
4. [阶段1实现总结](STAGE1_TRANSPORT_IMPLEMENTATION.md) - 传输层实现
5. [阶段2实现总结](STAGE2_PROTOCOL_IMPLEMENTATION.md) - 协议层实现

### 协议开发
1. [协议层架构](LAYERED_ARCHITECTURE.md#2-协议层-protocol-layer)
2. [QGA 使用指南](QGA_GUIDE.md)
3. [阶段2实现总结](STAGE2_PROTOCOL_IMPLEMENTATION.md)

### VDI 平台测试
1. [VDI 平台测试设计](VDI_PLATFORM_TESTING.md)
2. [VDI API 参考](Ocloud%20View%209.0接口文档_OpenAPI.json)

---

## 📝 文档贡献

### 文档规范
- 使用 Markdown 格式
- 添加目录和锚点链接
- 包含代码示例和图表
- 及时更新过时内容

### 提交文档更新
1. 确保文档准确性
2. 运行拼写检查
3. 更新文档索引（本 README）
4. 提交 PR 并说明变更

---

## 🔗 相关链接

- **代码仓库**: [atp-core/](../atp-core/)
- **示例场景**: [examples/](../examples/)
- **配置文件**: [config/](../config/)
- **TODO 清单**: [TODO.md](../TODO.md)

---

**最后更新**: 2025-11-24
**维护者**: OCloudView ATP Team
**项目版本**: v0.1.0

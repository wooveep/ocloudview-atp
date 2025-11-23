# OCloudView ATP - 开发 TODO 清单

## 项目状态

✅ **已完成**: 项目架构重构和基础框架搭建

当前版本: v0.1.0 (重构分支)
最后更新: 2024-11-23

---

## 阶段 1: 传输层实现 (优先级: 🔥 高)

### 1.1 连接管理 ✅
- [x] 创建 HostConnection 基础结构
- [x] 实现连接状态管理
- [ ] 实现自动重连逻辑
- [ ] 添加心跳检测机制
- [ ] 实现连接健康检查

### 1.2 连接池 ⏳
- [x] 创建 ConnectionPool 基础结构
- [ ] 实现连接获取策略（轮询、最少连接等）
- [ ] 实现连接池自动扩缩容
- [ ] 添加连接空闲超时处理
- [ ] 实现连接池监控指标

### 1.3 传输管理器 ⏳
- [x] 创建 TransportManager 基础结构
- [ ] 实现并发任务执行
- [ ] 添加负载均衡功能
- [ ] 实现故障转移机制
- [ ] 添加性能监控

### 1.4 测试 📝
- [ ] 单元测试 (config, connection, pool, manager)
- [ ] 集成测试 (多主机场景)
- [ ] 性能测试 (并发能力、延迟)
- [ ] Mock Libvirt 连接用于测试

---

## 阶段 2: 协议层实现 (优先级: 🔥 高)

### 2.1 协议抽象 ✅
- [x] 定义 Protocol trait
- [x] 定义 ProtocolBuilder trait
- [x] 创建 ProtocolRegistry

### 2.2 QMP 协议迁移 📝
- [ ] 将 test-controller/src/qmp 代码迁移到 protocol/qmp
- [ ] 适配 Protocol trait 接口
- [ ] 实现 QmpProtocolBuilder
- [ ] 添加单元测试
- [ ] 更新文档

**迁移步骤**:
1. 复制现有 QMP 代码到新位置
2. 重构为实现 Protocol trait
3. 移除对旧 libvirt 模块的直接依赖
4. 通过 transport 层获取连接
5. 测试验证

### 2.3 QGA 协议迁移 📝
- [ ] 将 test-controller/src/qga 代码迁移到 protocol/qga
- [ ] 适配 Protocol trait 接口
- [ ] 实现 QgaProtocolBuilder
- [ ] 添加单元测试
- [ ] 更新文档

### 2.4 自定义协议支持 📝
- [ ] 实现 virtio-serial 通道发现
- [ ] 实现通道读写逻辑
- [ ] 添加协议示例
- [ ] 编写开发指南

### 2.5 SPICE 协议预留 📋
- [ ] 定义 SPICE 协议接口
- [ ] 创建占位实现
- [ ] 编写集成计划文档

---

## 阶段 3: 执行器实现 (优先级: 🟡 中)

### 3.1 场景定义 ✅
- [x] 定义 Scenario 数据结构
- [x] 定义 Action 类型
- [ ] 添加场景验证逻辑
- [ ] 支持场景变量和参数化

### 3.2 场景加载 📝
- [ ] 实现 YAML 场景加载
- [ ] 实现 JSON 场景加载
- [ ] 添加场景验证
- [ ] 支持场景模板

### 3.3 场景执行 📝
- [ ] 实现 ScenarioRunner
- [ ] 支持步骤顺序执行
- [ ] 添加错误处理和重试
- [ ] 实现超时控制
- [ ] 生成执行报告

### 3.4 场景库 📋
- [ ] 创建示例场景
  - [ ] 键盘输入测试
  - [ ] 鼠标操作测试
  - [ ] 命令执行测试
  - [ ] 组合操作测试
- [ ] 场景文档和注释

---

## 阶段 4: CLI 应用实现 (优先级: 🟡 中)

### 4.1 基础命令 ✅
- [x] 创建 CLI 框架 (clap)
- [x] 定义命令结构
- [ ] 实现主机管理命令
  - [ ] `atp host add`
  - [ ] `atp host list`
  - [ ] `atp host remove`

### 4.2 输入命令 📝
- [ ] 实现键盘命令
  - [ ] `atp keyboard send`
  - [ ] `atp keyboard text`
- [ ] 实现鼠标命令
  - [ ] `atp mouse click`
  - [ ] `atp mouse move`

### 4.3 执行命令 📝
- [ ] 实现命令执行
  - [ ] `atp command exec`
- [ ] 实现场景命令
  - [ ] `atp scenario run`
  - [ ] `atp scenario list`

### 4.4 高级功能 📋
- [ ] 添加并发执行支持 (`--concurrent`)
- [ ] 添加循环执行支持 (`--loop`)
- [ ] 添加交互式模式
- [ ] 美化输出 (进度条、彩色输出)

---

## 阶段 5: HTTP API 实现 (优先级: 🟡 中)

### 5.1 基础框架 📝
- [ ] 创建 Axum 应用
- [ ] 设置路由
- [ ] 添加中间件 (CORS, 日志, 错误处理)
- [ ] 配置管理

### 5.2 API 端点 📝
- [ ] 主机管理 API
  - [ ] `POST /api/v1/hosts`
  - [ ] `GET /api/v1/hosts`
  - [ ] `DELETE /api/v1/hosts/:id`
  - [ ] `GET /api/v1/hosts/:id/vms`
- [ ] 输入控制 API
  - [ ] `POST /api/v1/keyboard/send`
  - [ ] `POST /api/v1/mouse/click`
  - [ ] `POST /api/v1/mouse/move`
- [ ] 命令执行 API
  - [ ] `POST /api/v1/command/exec`
- [ ] 场景管理 API
  - [ ] `POST /api/v1/scenarios/run`
  - [ ] `GET /api/v1/scenarios/:id/status`

### 5.3 WebSocket 📋
- [ ] 实现 WebSocket 端点
- [ ] 实时事件推送
- [ ] 实时日志流

### 5.4 文档 📋
- [ ] OpenAPI/Swagger 文档
- [ ] API 使用示例
- [ ] Postman 集合

---

## 阶段 6: Guest 验证器实现 (优先级: 🟢 低)

### 6.1 核心库 ✅
- [x] 定义 Verifier trait
- [x] 定义 VerifierTransport trait
- [x] 定义 Event 和 VerifyResult

### 6.2 验证器实现 📝
- [ ] 实现 KeyboardVerifier
  - [ ] Linux (evdev)
  - [ ] Windows (Hook API)
- [ ] 实现 MouseVerifier
  - [ ] Linux (evdev)
  - [ ] Windows (Hook API)
- [ ] 实现 CommandVerifier

### 6.3 传输层实现 📝
- [ ] 实现 WebSocketTransport
- [ ] 实现 TcpTransport
- [ ] 添加重连逻辑

### 6.4 Agent 应用 📝
- [ ] 实现 Agent 主程序
- [ ] 添加 CLI 参数解析
- [ ] 实现事件循环
- [ ] 添加配置文件支持

### 6.5 Web 验证器 📋
- [ ] 迁移现有 Web Agent
- [ ] 适配新的 API 格式
- [ ] 优化用户界面

---

## 阶段 7: 集成和测试 (优先级: 🔥 高)

### 7.1 单元测试 ⏳
- [ ] transport 模块测试覆盖率 > 80%
- [ ] protocol 模块测试覆盖率 > 80%
- [ ] executor 模块测试覆盖率 > 80%

### 7.2 集成测试 📝
- [ ] 端到端测试
  - [ ] CLI -> Transport -> Protocol -> VM
  - [ ] HTTP API -> Transport -> Protocol -> VM
- [ ] 多主机并发测试
- [ ] 场景执行测试

### 7.3 性能测试 📋
- [ ] 连接池性能
- [ ] 并发执行能力 (50+ VMs)
- [ ] 延迟测试 (< 20ms)
- [ ] 压力测试

---

## 阶段 8: 文档和示例 (优先级: 🟡 中)

### 8.1 架构文档 ✅
- [x] LAYERED_ARCHITECTURE.md
- [x] REFACTORING_PLAN.md
- [ ] 更新 README.md
- [ ] MIGRATION_GUIDE.md (从旧代码迁移)

### 8.2 API 文档 📝
- [ ] Transport API 文档
- [ ] Protocol API 文档
- [ ] Executor API 文档
- [ ] HTTP API 文档

### 8.3 使用指南 📋
- [ ] CLI 使用指南
- [ ] HTTP API 使用指南
- [ ] 场景编写指南
- [ ] 自定义协议开发指南
- [ ] Guest 验证器部署指南

### 8.4 示例 📋
- [ ] 基础示例
  - [ ] 简单键盘输入
  - [ ] 鼠标点击
  - [ ] 命令执行
- [ ] 高级示例
  - [ ] 多主机并发
  - [ ] 复杂场景
  - [ ] 自定义协议

---

## 阶段 9: Web 控制台 (优先级: 🟢 低)

### 9.1 前端框架 📋
- [ ] 选择前端框架 (React/Vue)
- [ ] 设置项目结构
- [ ] 配置构建工具

### 9.2 功能模块 📋
- [ ] 主机管理界面
- [ ] 虚拟机列表
- [ ] 实时控制台
- [ ] 场景管理器
- [ ] 监控面板

---

## 阶段 10: 优化和扩展 (优先级: 🟢 低)

### 10.1 性能优化 📋
- [ ] 连接池优化
- [ ] 协议解析优化
- [ ] 内存使用优化

### 10.2 功能扩展 📋
- [ ] SPICE 协议实现
- [ ] 视频流捕获
- [ ] 编码能力测试
- [ ] 更多验证器类型

### 10.3 DevOps 📋
- [ ] CI/CD 配置
- [ ] Docker 镜像
- [ ] 部署文档

---

## 近期优先任务 (本周)

1. ✅ 创建项目目录结构
2. ⏳ 实现传输层核心功能
   - 自动重连
   - 心跳检测
   - 连接池策略
3. 📝 迁移 QMP 协议到新架构
4. 📝 实现基础 CLI 命令

---

## 技术债务

- [ ] 添加更完善的错误处理
- [ ] 统一日志格式
- [ ] 添加性能监控指标
- [ ] 改进测试覆盖率
- [ ] 添加 benchmarks

---

## 已知问题

目前无已知问题

---

## 图例

- 📋 待开始
- 📝 进行中
- ⏳ 部分完成
- ✅ 已完成
- 🔥 高优先级
- 🟡 中优先级
- 🟢 低优先级

---

## 更新日志

### 2024-11-23
- 创建分层架构设计
- 重构项目目录结构
- 创建所有模块的基础框架
- 编制详细 TODO 清单

---

## 贡献指南

如果你想参与开发，请：
1. 阅读 LAYERED_ARCHITECTURE.md 了解架构
2. 从 TODO 中选择一个任务
3. 创建 feature 分支
4. 提交 PR

---

**项目地址**: https://github.com/wooveep/ocloudview-atp
**文档**: docs/
**联系**: OCloudView ATP Team

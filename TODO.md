# OCloudView ATP - 开发 TODO 清单

## 项目状态

✅ **阶段1完成**: 传输层核心功能实现
✅ **阶段2部分完成**: QMP 和 QGA 协议实现
🔄 **当前阶段**: 协议层剩余部分（VirtioSerial、SPICE）

当前版本: v0.1.0 (重构分支)
最后更新: 2025-11-24

---

## 阶段 1: 传输层实现 ✅ (已完成)

### 1.1 连接管理 ✅
- [x] 创建 HostConnection 基础结构
- [x] 实现连接状态管理
- [x] 实现自动重连逻辑（指数退避）
- [x] 添加心跳检测机制（异步任务）
- [x] 实现连接健康检查

**完成情况**: 所有功能已实现，支持可配置的重连策略和心跳检测

### 1.2 连接池 ✅
- [x] 创建 ConnectionPool 基础结构
- [x] 实现连接获取策略（轮询、最少连接、随机）
- [x] 实现连接池自动扩缩容
- [x] 添加连接空闲超时处理
- [x] 实现连接池监控指标（ConnectionMetrics）

**完成情况**: 完整的连接池管理，包括自动扩缩容和详细监控

### 1.3 传输管理器 ✅
- [x] 创建 TransportManager 基础结构
- [x] 实现并发任务执行（execute_on_hosts）
- [x] 添加负载均衡功能（通过连接池策略）
- [x] 实现多主机管理
- [x] 添加性能监控（统计查询）

**完成情况**: 支持多主机并发执行和负载均衡

### 1.4 测试 📝
- [ ] 单元测试 (config, connection, pool, manager)
- [ ] 集成测试 (多主机场景)
- [ ] 性能测试 (并发能力、延迟)
- [ ] Mock Libvirt 连接用于测试

**参考文档**: `docs/STAGE1_TRANSPORT_IMPLEMENTATION.md`

---

## 阶段 2: 协议层实现 (优先级: 🔥 高) ⏳ 部分完成

### 2.1 协议抽象 ✅
- [x] 定义 Protocol trait
- [x] 定义 ProtocolBuilder trait
- [x] 创建 ProtocolRegistry
- [x] 定义 ProtocolType 枚举

**完成情况**: 统一的协议抽象接口已完成

### 2.2 QMP 协议实现 ✅
- [x] 迁移 test-controller/src/qmp 代码到 protocol/qmp
- [x] 适配 Protocol trait 接口
- [x] 实现 QmpProtocolBuilder
- [x] 使用 tokio::io::split() 实现异步读写
- [x] 实现键盘输入支持（send_keys, send_key）
- [x] 实现虚拟机状态查询（query_version, query_status）
- [x] 实现 QMP 握手和能力协商
- [ ] 添加单元测试
- [ ] 更新文档

**代码行数**: ~440 行
**完成情况**: 核心功能已完成，支持 Unix Socket 通信和异步操作

### 2.3 QGA 协议实现 ✅
- [x] 迁移 test-controller/src/qga 代码到 protocol/qga
- [x] 适配 Protocol trait 接口
- [x] 实现 QgaProtocolBuilder
- [x] 使用 spawn_blocking 封装同步 libvirt 调用
- [x] 实现 Guest 命令执行（exec, exec_shell）
- [x] 实现命令状态查询（exec_status, exec_and_wait）
- [x] 实现 Base64 编解码支持
- [x] 启用 virt crate 的 qemu 特性
- [ ] 添加单元测试
- [ ] 更新文档

**代码行数**: ~381 行
**完成情况**: 核心功能已完成，支持 Guest 命令执行和输出捕获

**参考文档**: `docs/STAGE2_PROTOCOL_IMPLEMENTATION.md`

### 2.4 VirtioSerial 自定义协议支持 📝 (进行中)
- [ ] 实现 virtio-serial 通道发现
  - [ ] 通过 libvirt XML 解析通道配置
  - [ ] 实现通道路径查找（/dev/virtio-ports/）
- [ ] 实现通道读写逻辑
  - [ ] 实现 VirtioSerialProtocol struct
  - [ ] 实现 Protocol trait
  - [ ] 使用 tokio::fs::File 进行异步 I/O
- [ ] 添加协议示例
  - [ ] 简单的命令响应示例
  - [ ] 文件传输示例
- [ ] 编写开发指南
  - [ ] 通道配置说明
  - [ ] 协议设计建议
  - [ ] 调试技巧

**优先级**: 中
**预计代码**: ~300 行

### 2.5 SPICE 协议接口 📋 (预留)
- [ ] 定义 SPICE 协议接口
  - [ ] 定义 SpiceProtocol trait
  - [ ] 定义多通道管理接口
- [ ] 创建占位实现
  - [ ] 基础结构定义
  - [ ] Protocol trait 实现（返回未实现错误）
- [ ] 编写集成计划文档
  - [ ] SPICE 架构说明
  - [ ] 通道类型列表
  - [ ] 实现路线图

**优先级**: 低（占位）
**预计代码**: ~150 行（占位）

---

## 阶段 3: VDI 平台集成 (优先级: 🟡 中) ⏳ 部分完成

### 3.1 VDI 平台客户端 ✅
- [x] 创建 VdiClient 基础结构
- [x] 实现 HTTP 客户端封装
- [x] 定义 API 数据模型
- [x] 实现各 API 模块
  - [x] DomainApi（虚拟机管理）
  - [x] DeskPoolApi（桌面池管理）
  - [x] HostApi（主机管理）
  - [x] ModelApi（模板管理）
  - [x] UserApi（用户管理）
- [ ] 添加单元测试
- [ ] 添加 API 使用示例

**代码行数**: ~650 行
**完成情况**: 完整的 VDI 平台 API 客户端

### 3.2 集成适配器 ⏳
- [x] 创建 VdiVirtualizationAdapter
- [x] 定义虚拟化层和 VDI 平台的映射
- [ ] 实现桌面池到虚拟机的查询
- [ ] 实现虚拟机状态同步
- [ ] 添加错误处理和重试

**代码行数**: ~120 行
**完成情况**: 基础框架已创建

### 3.3 场景编排器 ⏳
- [x] 定义 TestScenario 数据结构
- [x] 定义 TestStep 枚举
  - [x] VdiAction（VDI 平台操作）
  - [x] VirtualizationAction（虚拟化层操作）
  - [x] Wait（等待）
  - [x] Verify（验证条件）
- [x] 实现 YAML/JSON 场景加载
- [x] 创建 ScenarioExecutor 基础结构
- [ ] 实现场景执行逻辑
- [ ] 实现验证条件检查
- [ ] 添加执行报告生成

**代码行数**: ~370 行
**完成情况**: 数据结构和加载器已完成，执行器待实现

**参考文档**: `docs/VDI_PLATFORM_TESTING.md`

---

## 阶段 4: 执行器实现 (优先级: 🟡 中) 📝

### 4.1 场景执行引擎 📝
- [x] 创建 ScenarioExecutor 框架
- [ ] 实现步骤顺序执行
- [ ] 实现 VDI 操作执行
  - [ ] 桌面池管理（创建、启用、禁用、删除）
  - [ ] 虚拟机管理（启动、关闭、重启、删除）
  - [ ] 用户绑定
- [ ] 实现虚拟化操作执行
  - [ ] 连接到虚拟机
  - [ ] 发送键盘输入
  - [ ] 发送鼠标操作
  - [ ] 执行命令
- [ ] 添加错误处理和重试
- [ ] 实现超时控制
- [ ] 生成执行报告

### 4.2 验证条件实现 📝
- [ ] 实现虚拟机状态验证
- [ ] 实现所有虚拟机运行验证
- [ ] 实现命令执行成功验证
- [ ] 添加自定义验证支持

### 4.3 场景模板 📋
- [ ] 创建示例场景
  - [ ] 桌面池创建和启动场景
  - [ ] 用户登录测试场景
  - [ ] 应用程序启动场景
  - [ ] 键盘输入测试场景
  - [ ] 鼠标操作测试场景
- [ ] 场景文档和注释

---

## 阶段 5: CLI 应用实现 (优先级: 🟢 低) 📋

### 5.1 基础命令 📋
- [ ] 创建 CLI 框架 (clap)
- [ ] 定义命令结构
- [ ] 实现主机管理命令
  - [ ] `atp host add`
  - [ ] `atp host list`
  - [ ] `atp host remove`
- [ ] 实现配置管理
  - [ ] `atp config init`
  - [ ] `atp config show`

### 5.2 输入命令 📋
- [ ] 实现键盘命令
  - [ ] `atp keyboard send`
  - [ ] `atp keyboard text`
- [ ] 实现鼠标命令
  - [ ] `atp mouse click`
  - [ ] `atp mouse move`

### 5.3 执行命令 📋
- [ ] 实现命令执行
  - [ ] `atp command exec`
- [ ] 实现场景命令
  - [ ] `atp scenario run`
  - [ ] `atp scenario list`
  - [ ] `atp scenario validate`

### 5.4 高级功能 📋
- [ ] 添加并发执行支持 (`--concurrent`)
- [ ] 添加循环执行支持 (`--loop`)
- [ ] 添加交互式模式
- [ ] 美化输出 (进度条、彩色输出)

---

## 阶段 6: HTTP API 实现 (优先级: 🟢 低) 📋

### 6.1 基础框架 📋
- [ ] 创建 Axum 应用
- [ ] 设置路由
- [ ] 添加中间件 (CORS, 日志, 错误处理)
- [ ] 配置管理

### 6.2 API 端点 📋
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

### 6.3 WebSocket 📋
- [ ] 实现 WebSocket 端点
- [ ] 实时事件推送
- [ ] 实时日志流

### 6.4 文档 📋
- [ ] OpenAPI/Swagger 文档
- [ ] API 使用示例
- [ ] Postman 集合

---

## 阶段 7: Guest 验证器实现 (优先级: 🟢 低) 📋

### 7.1 核心库 ✅
- [x] 定义 Verifier trait
- [x] 定义 VerifierTransport trait
- [x] 定义 Event 和 VerifyResult

### 7.2 验证器实现 📋
- [ ] 实现 KeyboardVerifier
  - [ ] Linux (evdev)
  - [ ] Windows (Hook API)
- [ ] 实现 MouseVerifier
  - [ ] Linux (evdev)
  - [ ] Windows (Hook API)
- [ ] 实现 CommandVerifier

### 7.3 传输层实现 📋
- [ ] 实现 WebSocketTransport
- [ ] 实现 TcpTransport
- [ ] 添加重连逻辑

### 7.4 Agent 应用 📋
- [ ] 实现 Agent 主程序
- [ ] 添加 CLI 参数解析
- [ ] 实现事件循环
- [ ] 添加配置文件支持

### 7.5 Web 验证器 📋
- [ ] 迁移现有 Web Agent
- [ ] 适配新的 API 格式
- [ ] 优化用户界面

---

## 阶段 8: 集成和测试 (优先级: 🔥 高) 📝

### 8.1 单元测试 📝
- [ ] transport 模块测试覆盖率 > 80%
- [ ] protocol 模块测试覆盖率 > 80%
- [ ] vdiplatform 模块测试覆盖率 > 80%
- [ ] orchestrator 模块测试覆盖率 > 80%
- [ ] executor 模块测试覆盖率 > 80%

### 8.2 集成测试 📝
- [ ] 端到端测试
  - [ ] Scenario -> Executor -> Transport -> Protocol -> VM
  - [ ] VDI Platform -> Adapter -> Virtualization
- [ ] 多主机并发测试
- [ ] 场景执行测试

### 8.3 性能测试 📋
- [ ] 连接池性能
- [ ] 并发执行能力 (50+ VMs)
- [ ] 延迟测试 (< 20ms)
- [ ] 压力测试

---

## 阶段 9: 文档和示例 (优先级: 🟡 中) ⏳

### 9.1 架构文档 ⏳
- [x] LAYERED_ARCHITECTURE.md
- [x] CONNECTION_MODES.md
- [x] STAGE1_TRANSPORT_IMPLEMENTATION.md
- [x] STAGE2_PROTOCOL_IMPLEMENTATION.md
- [x] VDI_PLATFORM_TESTING.md
- [ ] 更新 README.md
- [ ] MIGRATION_GUIDE.md (从旧代码迁移)
- [ ] CONTRIBUTING.md (贡献指南)

### 9.2 API 文档 📝
- [ ] Transport API 文档
- [ ] Protocol API 文档
- [ ] VDI Platform API 文档
- [ ] Executor API 文档
- [ ] HTTP API 文档

### 9.3 使用指南 📋
- [ ] CLI 使用指南
- [ ] HTTP API 使用指南
- [ ] 场景编写指南
- [ ] 自定义协议开发指南
- [ ] VDI 平台集成指南
- [ ] Guest 验证器部署指南

### 9.4 示例 📋
- [ ] 基础示例
  - [ ] 简单键盘输入
  - [ ] 鼠标点击
  - [ ] 命令执行
- [ ] VDI 场景示例
  - [ ] 桌面池创建和启动
  - [ ] 用户登录测试
  - [ ] 应用程序测试
- [ ] 高级示例
  - [ ] 多主机并发
  - [ ] 复杂场景
  - [ ] 自定义协议

---

## 阶段 10: Web 控制台 (优先级: 🟢 低) 📋

### 10.1 前端框架 📋
- [ ] 选择前端框架 (React/Vue)
- [ ] 设置项目结构
- [ ] 配置构建工具

### 10.2 功能模块 📋
- [ ] 主机管理界面
- [ ] 虚拟机列表
- [ ] 实时控制台
- [ ] 场景管理器
- [ ] 监控面板

---

## 阶段 11: 优化和扩展 (优先级: 🟢 低) 📋

### 11.1 性能优化 📋
- [ ] 连接池优化
- [ ] 协议解析优化
- [ ] 内存使用优化

### 11.2 功能扩展 📋
- [ ] SPICE 协议实现
- [ ] 视频流捕获
- [ ] 编码能力测试
- [ ] 更多验证器类型

### 11.3 DevOps 📋
- [ ] CI/CD 配置
- [ ] Docker 镜像
- [ ] 部署文档

---

## 近期优先任务 (本周)

1. ✅ 创建项目目录结构
2. ✅ 实现传输层核心功能
   - ✅ 自动重连
   - ✅ 心跳检测
   - ✅ 连接池策略
   - ✅ 自动扩缩容
3. ✅ 实现 QMP 和 QGA 协议
   - ✅ QMP 协议完整实现
   - ✅ QGA 协议完整实现
4. 🔄 实现场景执行器（当前）
   - [ ] VDI 操作执行
   - [ ] 虚拟化操作执行
   - [ ] 验证条件实现
5. 📝 实现 VirtioSerial 协议
6. 📝 添加单元测试

---

## 技术债务

- [ ] 添加更完善的错误处理（协议层）
- [ ] 统一日志格式
- [ ] 添加性能监控指标（执行器层）
- [ ] 改进测试覆盖率（所有模块 < 50%）
- [ ] 添加 benchmarks
- [ ] QMP Socket 路径解析（当前是简化版）
- [ ] QGA 轮询机制配置化
- [ ] VDI 客户端生命周期警告处理
- [ ] 场景执行器中未使用的字段

---

## 已知问题

### 协议层
1. **QMP Socket 路径**: 当前使用简化的路径构建，应从 libvirt XML 读取
2. **QMP/QGA receive()**: 由于是请求-响应模式，独立的 receive() 返回错误

### VDI 平台层
1. **生命周期警告**: VdiClient 中有未使用的字段和方法警告

### 场景编排器
1. **未实现的执行逻辑**: ScenarioExecutor 框架已创建但核心逻辑待实现

---

## 代码统计

### 已完成模块
- **transport**: ~1,310 行（完整）
- **protocol**: ~1,100 行（QMP + QGA + 抽象层）
- **vdiplatform**: ~650 行（API 客户端）
- **orchestrator**: ~370 行（场景定义）
- **executor**: ~150 行（框架）

**总计**: ~3,580 行代码

### 文档
- **架构文档**: 5 个
- **实现总结**: 2 个（Stage 1 & 2）
- **测试文档**: 2 个

---

## 图例

- 📋 待开始
- 📝 进行中
- ⏳ 部分完成
- ✅ 已完成
- 🔄 当前任务
- 🔥 高优先级
- 🟡 中优先级
- 🟢 低优先级

---

## 更新日志

### 2025-11-24
- ✅ 完成阶段2协议层 QMP 和 QGA 实现
- ✅ 启用 virt crate 的 qemu 特性
- ✅ 创建 STAGE2_PROTOCOL_IMPLEMENTATION.md 文档
- ✅ 提交协议层实现代码
- 🔄 更新 TODO.md 以反映当前进度

### 2024-11-23
- ✅ 完成阶段1传输层所有核心功能
- ✅ 创建 VDI 平台客户端模块
- ✅ 创建场景编排器框架
- ✅ 创建 STAGE1_TRANSPORT_IMPLEMENTATION.md 文档

### 2024-11-22
- ✅ 创建分层架构设计
- ✅ 重构项目目录结构
- ✅ 创建所有模块的基础框架
- ✅ 编制详细 TODO 清单

---

## 贡献指南

如果你想参与开发，请：
1. 阅读 `docs/LAYERED_ARCHITECTURE.md` 了解架构
2. 阅读相关阶段的实现总结文档（`docs/STAGE*_IMPLEMENTATION.md`）
3. 从 TODO 中选择一个标记为 📋 或 📝 的任务
4. 创建 feature 分支
5. 提交 PR，并确保：
   - 代码通过 `cargo check` 和 `cargo clippy`
   - 添加必要的测试
   - 更新相关文档

---

## 相关链接

**文档目录**:
- 架构设计: `docs/LAYERED_ARCHITECTURE.md`
- 连接模式: `docs/CONNECTION_MODES.md`
- 阶段1总结: `docs/STAGE1_TRANSPORT_IMPLEMENTATION.md`
- 阶段2总结: `docs/STAGE2_PROTOCOL_IMPLEMENTATION.md`
- VDI平台测试: `docs/VDI_PLATFORM_TESTING.md`

**代码目录**:
- 传输层: `atp-core/transport/`
- 协议层: `atp-core/protocol/`
- VDI平台: `atp-core/vdiplatform/`
- 场景编排: `atp-core/orchestrator/`
- 执行器: `atp-core/executor/`

---

**最后更新**: 2025-11-24
**维护者**: OCloudView ATP Team

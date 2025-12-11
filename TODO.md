# OCloudView ATP - 开发任务清单

## 项目状态总览

| 指标 | 当前值 | 目标值 |
|------|--------|--------|
| 整体进度 | **83%** | 100% |
| 代码行数 | 15,200+ | - |
| 测试用例 | 66+ | 150+ |
| 测试覆盖率 | 60% | 80%+ |
| 文档数量 | 44 | - |

**当前版本**: v0.4.0
**最后更新**: 2025-12-11

---

## 模块完成度

| 模块 | 完成度 | 代码行数 | 状态 |
|------|--------|----------|------|
| Transport (传输层) | 85% | ~1,439 | 核心完成 |
| Protocol - QMP | 100% | ~440 | 完成 |
| Protocol - QGA | 100% | ~381 | 完成 |
| Protocol - VirtioSerial | 95% | ~653 | 完成 |
| Protocol - SPICE | 60% | ~4,785 | 框架完成 |
| Executor (执行器) | 85% | ~1,200 | 核心完成 |
| Storage (存储层) | 85% | ~800 | 核心完成 |
| VDI Platform | 60% | ~650 | 基础完成 |
| Verification Server | 95% | ~1,010 | 完成 |
| Guest Verifier | 80% | ~2,910 | Linux/Windows完成 |
| CLI | 90% | ~550 | 核心完成 |
| HTTP API | 0% | - | 未开始 |

---

## 近期优先任务

### 高优先级 (本周)

#### 1. 集成测试运行
- [ ] 配置本地 libvirtd 测试环境
- [ ] 运行 E2E 测试套件验证功能
- [ ] 修复发现的问题

#### 2. VDI 平台集成完善
- [ ] 完善 VdiVirtualizationAdapter 适配器
- [ ] 实现桌面池到虚拟机映射
- [ ] 虚拟机状态同步机制

#### 3. 测试覆盖率提升
- [ ] Transport 模块集成测试
- [ ] Protocol 协议实现测试
- [ ] 目标: 覆盖率达到 70%

### 中优先级 (本月)

#### 4. SPICE 协议细节实现
- [ ] RSA-OAEP 密码加密
- [ ] TLS 加密连接
- [ ] 视频流解码 (VP8/H.264)

#### 5. Executor VDI 操作
- [ ] 桌面池管理 (创建、启用、禁用)
- [ ] 虚拟机管理 (启动、关闭、重启)
- [ ] 验证条件实现

#### 6. Custom 协议实现
- [ ] 实现 connect/disconnect 逻辑
- [ ] 实现 send/receive 方法

### 低优先级 (后续)

#### 7. HTTP API
- [ ] Axum 框架搭建
- [ ] RESTful API 端点
- [ ] WebSocket 实时推送
- [ ] Swagger 文档

#### 8. 性能优化
- [ ] 连接池性能优化
- [ ] 并发测试 (50+ VMs)
- [ ] 延迟测试 (< 20ms)

#### 9. Web 控制台
- [ ] 前端框架选择
- [ ] 功能模块实现

---

## 各阶段详细任务

### 阶段 1: 传输层 (85%)

**已完成**:
- [x] HostConnection 连接管理
- [x] 自动重连逻辑 (指数退避)
- [x] 心跳检测机制
- [x] ConnectionPool 连接池
- [x] 连接获取策略 (轮询/最少连接/随机)
- [x] 连接池扩缩容
- [x] TransportManager 多主机管理
- [x] 并发任务执行

**待完成**:
- [ ] 性能指标持久化到数据库

---

### 阶段 2: 协议层 (70%)

#### QMP 协议 (100%)
- [x] Unix Socket 连接
- [x] QMP 握手和能力协商
- [x] 键盘输入 (send_keys, send_key)
- [x] 虚拟机状态查询

#### QGA 协议 (100%)
- [x] libvirt qemu_agent_command 集成
- [x] Guest 命令执行
- [x] 命令状态查询
- [x] Base64 编解码

#### VirtioSerial 协议 (95%)
- [x] 通道发现 (XML 解析)
- [x] Unix Socket 异步 I/O
- [x] 可扩展协议处理器
- [x] Raw/JSON 处理器

#### SPICE 协议 (60%)
**已完成**:
- [x] 多通道架构 (Main, Display, Inputs, Usbredir)
- [x] 通道连接和握手
- [x] 空认证流程
- [x] libvirt 集成 (SpiceDiscovery)
- [x] 输入通道 (键盘/鼠标)
- [x] 显示通道 (Surface, 视频流事件)
- [x] USB 重定向框架

**待完成** (29 个 TODO):
- [ ] RSA-OAEP 密码加密
- [ ] TLS 支持
- [ ] 视频流解码
- [ ] 绘图命令解析 (QUIC, LZ, GLZ)
- [ ] 完整 USB 重定向协议
- [ ] quick-xml 改进 XML 解析

---

### 阶段 3: VDI 平台集成 (60%)

**已完成**:
- [x] VdiClient HTTP 客户端
- [x] MD5 密码加密和 Token 认证
- [x] DomainApi (虚拟机管理)
- [x] DeskPoolApi (桌面池管理)
- [x] HostApi (主机管理)
- [x] ModelApi (模板管理)
- [x] UserApi (用户管理)
- [x] 分页查询支持

**待完成**:
- [ ] VdiVirtualizationAdapter 完善
- [ ] 桌面池到虚拟机映射
- [ ] 虚拟机状态同步
- [ ] 数据库缓存层

---

### 阶段 4: 执行器 (85%)

**已完成**:
- [x] ScenarioRunner 执行引擎
- [x] 场景加载 (YAML/JSON)
- [x] 协议集成 (QMP/QGA/SPICE)
- [x] QMP 键盘输入
- [x] QMP 文本输入
- [x] SPICE 鼠标操作
- [x] QGA 命令执行
- [x] QGA + xdotool 备用方案
- [x] 执行报告生成
- [x] 测试配置加载模块 (TestConfig)

**待完成**:
- [ ] VDI 操作集成
  - [ ] 桌面池管理
  - [ ] 虚拟机管理
  - [ ] 用户绑定
- [ ] 验证条件实现
  - [ ] 虚拟机状态验证
  - [ ] 命令成功验证
  - [ ] 自定义验证支持

---

### 阶段 5: CLI (90%)

**已完成**:
- [x] 主机管理命令 (add, list, remove)
- [x] 场景执行命令 (run, list)
- [x] 报告管理命令 (list, show, export, delete, stats, cleanup)
- [x] 数据库备份命令 (backup, restore, list, delete, cleanup)
- [x] VDI 集成命令 (verify, list-hosts, list-vms)
- [x] 配置文件管理
- [x] 彩色输出和进度条

**待完成**:
- [ ] 并发执行支持 (--concurrent)
- [ ] 循环执行支持 (--loop)
- [ ] 交互式模式

---

### 阶段 6: 数据库层 (85%)

**已完成**:
- [x] SQLite 数据库支持
- [x] StorageManager 连接管理
- [x] ReportRepository (测试报告)
- [x] ScenarioRepository (场景库)
- [x] 数据库迁移脚本
- [x] BackupManager (备份恢复)
- [x] 36 个单元测试

**待完成**:
- [ ] HostRepository (主机配置) - 低优先级
- [ ] MetricRepository (性能指标) - 低优先级
- [ ] tags 过滤支持

---

### 阶段 7: Guest 验证器 (80%)

**已完成**:
- [x] Linux 平台支持 (evdev)
  - [x] KeyboardVerifier
  - [x] MouseVerifier
  - [x] CommandVerifier
- [x] Windows 平台支持 (Hook API)
  - [x] KeyboardVerifier
  - [x] MouseVerifier
  - [x] CommandVerifier
- [x] WebSocketTransport
- [x] TcpTransport
- [x] VM ID 握手机制
- [x] 自动重连逻辑
- [x] Agent 主程序

**待完成**:
- [ ] 配置文件支持 - 低优先级
- [ ] macOS 平台支持 - 低优先级

---

### 阶段 8: 测试 (65%)

**已完成**:
- [x] Executor 单元测试 (47 个, 100%)
- [x] Transport 基础测试 (21 个)
- [x] Protocol 基础测试 (6 个)
- [x] Storage 单元测试 (36 个, 100%)
- [x] E2E 测试框架 (10 个测试用例)
- [x] 测试配置指南

**待完成**:
- [ ] Transport 集成测试 (Mock libvirt)
- [ ] Protocol 协议测试
- [ ] VDI Platform API 测试
- [ ] 实际环境 E2E 测试
- [ ] 性能测试

**目标**: 测试覆盖率 80%+

---

## 技术债务

| 类别 | 问题 | 优先级 | 位置 |
|------|------|--------|------|
| 协议 | QMP Socket 路径从 XML 读取 | 中 | qmp.rs:276 |
| 协议 | SPICE XML 解析优化 (quick-xml) | 低 | discovery.rs |
| 协议 | Custom 协议待实现 | 中 | custom.rs |
| VDI | 生命周期警告处理 | 低 | vdiplatform |
| 执行器 | 未使用的协议缓存字段 | 低 | runner.rs |
| 日志 | 统一日志格式 | 低 | 全局 |

---

## 已知问题

### 协议层
1. **QMP Socket 路径**: 当前使用简化路径构建，应从 libvirt XML 读取
2. **QMP/QGA receive()**: 请求-响应模式下独立 receive() 返回错误

### SPICE 协议
1. **对齐错误**: 部分测试存在结构体对齐问题
2. **认证**: RSA 密码认证未实现
3. **TLS**: 加密连接未实现

### VDI 平台
1. **警告**: VdiClient 有未使用的字段警告

---

## 代码统计

### 模块代码量

| 模块 | 行数 | 文件数 |
|------|------|--------|
| transport | 1,439 | 5 |
| protocol/qmp | 440 | 1 |
| protocol/qga | 381 | 1 |
| protocol/virtio | 653 | 3 |
| protocol/spice | 4,785 | 10 |
| protocol/抽象层 | 279 | 2 |
| executor | 1,200 | 4 |
| storage | 800 | 8 |
| vdiplatform | 650 | 10 |
| verification-server | 1,010 | 5 |
| guest-verifier | 2,910 | 8 |
| cli | 550 | 5 |
| **总计** | **15,097** | **62** |

### 测试统计

| 模块 | 测试数 | 通过率 |
|------|--------|--------|
| executor | 47 | 100% |
| storage | 36 | 100% |
| transport | 21 | 部分 |
| protocol | 6 | 部分 |
| **总计** | **110** | **75%** |

### TODO/FIXME 统计

| 模块 | TODO | FIXME |
|------|------|-------|
| protocol (SPICE) | 29 | 0 |
| executor | 4 | 0 |
| cli | 5 | 0 |
| custom protocol | 4 | 0 |
| storage | 2 | 0 |
| vdiplatform | 1 | 0 |
| transport | 1 | 0 |
| **总计** | **46** | **0** |

---

## 版本规划

### v0.4.0 (当前)
- [x] 基础架构完成
- [x] 核心协议实现 (QMP/QGA/VirtioSerial)
- [x] 数据库层集成
- [x] CLI 工具完善
- [x] VDI 平台基础集成
- [x] Guest 验证器 (Linux/Windows)

### v0.5.0 (计划)
- [ ] VDI 平台完整集成
- [ ] 测试覆盖率 80%
- [ ] SPICE 协议完善

### v0.6.0 (计划)
- [ ] HTTP API
- [ ] WebSocket 实时推送
- [ ] 性能优化

### v1.0.0 (目标)
- [ ] 生产级稳定性
- [ ] 完整文档
- [ ] Web 控制台

---

## 图例

| 标记 | 含义 |
|------|------|
| [x] | 已完成 |
| [ ] | 待完成 |
| - | 不适用 |

---

## 更新日志

### 2025-12-11
- 重新整理 TODO 文档结构
- 更新项目状态和各模块完成度
- 整理近期优先任务
- 更新代码统计数据

### 2025-12-01
- 完成测试配置加载模块
- 完成 Executor/Orchestrator 合并
- E2E 测试框架完成

### 2025-11-26
- Guest 验证器 Windows 实现完成
- 单元测试框架建立

### 2025-11-25
- 数据库集成完成
- CLI 报告命令实现

---

## 贡献指南

1. 阅读 `docs/LAYERED_ARCHITECTURE.md` 了解架构
2. 阅读相关模块的实现文档
3. 选择标记为 `[ ]` 的任务
4. 创建 feature 分支
5. 提交 PR

### PR 要求
- 代码通过 `cargo check` 和 `cargo clippy`
- 添加必要的测试
- 更新相关文档

---

## 相关链接

- **文档中心**: [docs/README.md](docs/README.md)
- **架构设计**: [docs/LAYERED_ARCHITECTURE.md](docs/LAYERED_ARCHITECTURE.md)
- **快速开始**: [docs/QUICKSTART.md](docs/QUICKSTART.md)
- **测试配置**: [docs/TESTING_CONFIG_GUIDE.md](docs/TESTING_CONFIG_GUIDE.md)

---

**维护者**: OCloudView ATP Team

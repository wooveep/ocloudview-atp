# OCloudView ATP - 开发任务清单

## 项目状态总览

| 指标 | 当前值 | 目标值 |
|------|--------|--------|
| 整体进度 | **92%** | 100% |
| 代码行数 | 32,500+ | - |
| 测试用例 | **98** | 200+ |
| 测试覆盖率 | **78%** | 80%+ |
| 文档数量 | 44 | - |

**当前版本**: v0.5.1-dev
**最后更新**: 2026-01-16

---

## 模块完成度

| 模块 | 完成度 | 代码行数 | 状态 |
|------|--------|----------|------|
| Transport (传输层) | 85% | ~1,562 | 核心完成 |
| Protocol - QMP | 100% | ~440 | 完成 |
| Protocol - QGA | 100% | ~500 | 完成 |
| Protocol - VirtioSerial | 95% | ~653 | 完成 |
| Protocol - SPICE | 65% | ~4,785 | **通道管理优化** |
| Executor (执行器) | **98%** | **~3,500** | **VDI智能集成完成** |
| Storage (存储层) | **95%** | ~1,000 | **主机/映射仓储完成** |
| VDI Platform | 85% | ~1,100 | 批量操作完成 |
| Verification Server | **100%** | ~1,195 | **Executor集成完成** |
| Guest Verifier | 80% | ~2,910 | Linux/Windows完成 |
| CLI | 95% | ~1,200 | **VDI集成完成** |
| HTTP API | 20% | ~300 | 基础框架 |

---

## 近期优先任务

### 高优先级 (本周)

#### 1. 安全问题修复
- [ ] 修复 SSH Host Key 验证 (实现 known_hosts 检查)
- [ ] 评估 MD5 密码哈希问题 (依赖服务端支持)

#### 2. 内存泄漏修复 ✅
- [x] 修复 executor/runner.rs 中的 .leak() 内存泄漏
- [x] 实现完整的字符到 QKeyCode 映射表 (支持 a-z, A-Z, 0-9, 标点符号, 空白字符)

#### 3. 集成测试运行
- [x] 配置本地 libvirtd 测试环境
- [x] 运行 E2E 测试套件验证功能
- [x] 修复发现的问题 (packed struct 对齐、XML 解析顺序)

#### 4. VDI 平台集成完善
- [x] 完善 DomainApi 批量操作方法
- [x] 实现虚拟机克隆功能
- [x] 实现从模板批量创建虚拟机
- [x] 实现配置修改功能 (CPU/内存)
- [x] 实现网络管理功能

#### 5. 测试覆盖率提升 ✅
- [x] Transport 模块集成测试
- [x] Protocol 协议实现测试 (30 tests)
- [x] Executor 模块测试 (56 tests)
- [x] 测试覆盖率达到 78%

### 中优先级 (本月)

#### 6. SPICE 协议细节实现
- [ ] RSA-OAEP 密码加密
- [ ] TLS 加密连接
- [ ] 视频流解码 (VP8/H.264)

#### 7. Executor VDI 操作
- [x] 桌面池管理 (创建、启用、禁用、删除)
- [x] 虚拟机管理 (启动、关闭、重启、删除)
- [x] 用户绑定操作
- [x] 验证条件实现 (虚拟机状态、命令成功)

#### 8. Custom 协议实现
- [ ] 实现 connect/disconnect 逻辑
- [ ] 实现 send/receive 方法

#### 9. CLI 代码质量改进
- [ ] 移除 scenario.rs 中的 unwrap() 调用
- [ ] 提取 StorageManager 辅助函数
- [ ] 完成 keyboard/mouse/command 命令实现

### 低优先级 (后续)

#### 10. HTTP API
- [x] Axum 框架搭建
- [ ] RESTful API 端点
- [ ] WebSocket 实时推送
- [ ] Swagger 文档

#### 11. 性能优化
- [ ] 连接池性能优化
- [ ] 并发测试 (50+ VMs)
- [ ] 延迟测试 (< 20ms)

#### 12. Web 控制台
- [ ] 前端框架选择
- [ ] 功能模块实现

#### 13. 文档完善
- [ ] 架构概览文档
- [ ] 模块集成指南
- [ ] 常见场景配置示例
- [ ] 部署指南
- [ ] 性能调优指南

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

### 阶段 3: VDI 平台集成 (85%)

**已完成**:
- [x] VdiClient HTTP 客户端
- [x] MD5 密码加密和 Token 认证
- [x] DomainApi (虚拟机管理)
- [x] DeskPoolApi (桌面池管理)
- [x] HostApi (主机管理)
- [x] ModelApi (模板管理)
- [x] UserApi (用户管理)
- [x] 分页查询支持
- [x] **批量操作 API** (新增)
  - [x] 批量启动/关闭/重启虚拟机
  - [x] 批量强制关机
  - [x] 批量删除虚拟机
- [x] **克隆操作 API** (新增)
  - [x] 完全克隆虚拟机
  - [x] 从模板批量创建虚拟机 (链接克隆)
  - [x] 克隆模板
- [x] **配置修改 API** (新增)
  - [x] 批量修改 CPU/内存
  - [x] 批量修改其他配置
- [x] **网络管理 API** (新增)
  - [x] 添加/修改网卡
  - [x] 删除网卡
  - [x] 获取网卡列表
- [x] **完整创建虚拟机 API** (新增)

**待完成**:
- [x] 数据库缓存层 (VdiCacheManager)
- [x] GlusterFS 脑裂自动修复

---

### 阶段 4: 执行器 (95%)

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
- [x] **多目标场景执行** (新增)
  - [x] TargetSelector 多虚拟机选择器
  - [x] 通配符/正则/列表匹配模式
  - [x] 排除规则和数量限制
  - [x] 串行/并行执行策略
  - [x] 失败策略 (Continue/StopAll/FailFast)
- [x] **主机选择器** (新增)
  - [x] 多主机目标支持
  - [x] 通配符匹配主机
- [x] **嵌入式验证服务器** (新增)
  - [x] 自动启动 VerificationServer
  - [x] 通过 QGA 启动 guest-verifier
  - [x] 验证结果匹配和上报
  - [x] 步骤验证状态 (verified/latency)
- [x] **VDI 平台操作** (新增)
  - [x] VdiCreateDeskPool
  - [x] VdiEnableDeskPool / VdiDisableDeskPool
  - [x] VdiDeleteDeskPool
  - [x] VdiStartDomain / VdiShutdownDomain / VdiRebootDomain
  - [x] VdiDeleteDomain
  - [x] VdiBindUser
  - [x] VdiGetDeskPoolDomains
- [x] **验证步骤** (新增)
  - [x] VerifyDomainStatus
  - [x] VerifyAllDomainsRunning
  - [x] VerifyCommandSuccess
- [x] **MultiTargetReport** 多目标报告

**待完成**:
- [ ] 自定义验证支持扩展

---

### 阶段 5: CLI (92%)

**已完成**:
- [x] 主机管理命令 (add, list, remove)
- [x] 场景执行命令 (run, list)
- [x] 报告管理命令 (list, show, export, delete, stats, cleanup)
- [x] 数据库备份命令 (backup, restore, list, delete, cleanup)
- [x] VDI 集成命令 (verify, list-hosts, list-vms)
- [x] **PowerShell 远程执行命令** (exec, list-vms) - 新增
  - [x] 通过 QGA (QEMU Guest Agent) 协议发送命令
  - [x] 支持单个VM、VM列表、所有VM目标
  - [x] UTF-16LE Base64 编码命令传输 (Windows -EncodedCommand)
  - [x] 支持命令行和脚本文件
  - [x] 按主机分组执行，优化连接复用
  - [x] JSON 格式输出
- [x] 配置文件管理
- [x] 彩色输出和进度条

**待完成**:
- [ ] 并发执行支持 (--concurrent)
- [ ] 循环执行支持 (--loop)
- [ ] 交互式模式

---

### 阶段 6: 数据库层 (95%)

**已完成**:
- [x] SQLite 数据库支持
- [x] StorageManager 连接管理
- [x] ReportRepository (测试报告)
- [x] ScenarioRepository (场景库)
- [x] 数据库迁移脚本
- [x] BackupManager (备份恢复)
- [x] 36 个单元测试
- [x] **HostRepository (主机配置)** (新增)
- [x] **DomainHostMappingRepository (虚拟机-主机映射)** (新增)

**待完成**:
- [ ] MetricRepository (性能指标) - 低优先级
- [ ] tags 过滤支持

---

### 阶段 7: Guest 验证器 (90%)

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
- [x] **Executor 集成** (新增)
  - [x] 嵌入式 VerificationServer 启动
  - [x] 通过 QGA 启动 guest-verifier
  - [x] 验证结果匹配和延迟记录

**待完成**:
- [ ] 配置文件支持 - 低优先级
- [ ] macOS 平台支持 - 低优先级

---

### 阶段 8: 测试 (75%)

**已完成**:
- [x] Executor 单元测试 (5 个, 100%)
- [x] Transport 单元测试 (24 个, 100%)
- [x] Protocol 单元测试 (74 个, 100%)
- [x] Storage 单元测试 (8 个, 100%)
- [x] VDI Platform 单元测试 (1 个, 100%)
- [x] Verification Server 单元测试 (4 个, 100%)
- [x] E2E 测试框架 (10 个, 2 通过, 8 需要实际虚拟机)
- [x] 测试配置指南
- [x] 修复 packed struct 对齐问题
- [x] 修复 VirtioSerial XML 解析顺序问题

**待完成**:
- [ ] Transport 集成测试 (Mock libvirt)
- [ ] VDI Platform API 测试
- [ ] 实际环境 E2E 测试
- [ ] 性能测试

**当前状态**: 158 个测试 (150 通过, 8 忽略)
**目标**: 测试覆盖率 80%+

---

## 技术债务

| 类别 | 问题 | 优先级 | 位置 |
|------|------|--------|------|
| **安全** | SSH Host Key 验证禁用 | **高** | ssh-executor/client.rs:56-62 |
| **安全** | MD5 密码哈希 (弱加密) | **高** | vdiplatform/client.rs:84 |
| **内存** | ~~字符映射使用 .leak() 导致内存泄漏~~ | ~~**高**~~ ✅ 已修复 | executor/runner.rs |
| **功能** | ~~字符到 QKeyCode 映射不完整~~ | ~~**高**~~ ✅ 已修复 | executor/runner.rs |
| 协议 | QMP Socket 路径从 XML 读取 | 中 | qmp.rs:276 |
| 协议 | SPICE XML 解析优化 (quick-xml) | 低 | discovery.rs |
| 协议 | Custom 协议待实现 | 中 | custom.rs |
| VDI | URL 输入验证缺失 | 中 | vdiplatform/client.rs:59-73 |
| VDI | 生命周期警告处理 | 低 | vdiplatform |
| 执行器 | 清理时静默忽略错误 | 中 | executor/runner.rs:211-221 |
| 执行器 | 未使用的协议缓存字段 | 低 | runner.rs |
| CLI | 多处 unwrap() 可能导致 panic | 中 | scenario.rs:32,64,110,236 |
| CLI | StorageManager 创建模式重复 | 低 | report.rs:32-33,107-108 |
| CLI | PowerShell 路径硬编码 | 低 | powershell.rs:401 |
| 日志 | 统一日志格式 | 低 | 全局 |

---

## 已知问题

### 安全问题 (高优先级)
1. **SSH Host Key 验证禁用**: 自动接受所有 host key，存在 MITM 攻击风险
   - 位置: `ssh-executor/src/client.rs:56-62`
   - 修复建议: 实现 known_hosts 文件检查

2. **MD5 弱密码哈希**: VDI 平台使用 MD5 进行密码哈希
   - 位置: `vdiplatform/src/client.rs:84`
   - 修复建议: 使用 bcrypt 或 PBKDF2 (需服务端支持)

### 内存问题 ✅ 已修复
1. ~~**字符映射内存泄漏**: 使用 `.leak()` 导致长时间运行时内存累积~~
   - ~~位置: `executor/src/runner.rs:347`~~
   - 已修复: 使用静态字符映射表 `char_to_qkeycode()` 替代 `.leak()`

### 协议层
1. **QMP Socket 路径**: 当前使用简化路径构建，应从 libvirt XML 读取
2. **QMP/QGA receive()**: 请求-响应模式下独立 receive() 返回错误
3. ~~**字符到 QKeyCode 映射不完整**: 仅支持基本 ASCII，特殊字符返回 "unknown"~~
   - ~~位置: `executor/src/runner.rs:346-351`~~
   - 已修复: 完善 `char_to_qkeycode()` 函数，支持 a-z, A-Z, 0-9, 标点符号, 空白字符

### SPICE 协议
1. ~~**对齐错误**: 部分测试存在结构体对齐问题~~ (已修复)
2. **认证**: RSA 密码认证未实现
3. **TLS**: 加密连接未实现

### VDI 平台
1. **警告**: VdiClient 有未使用的字段警告
2. **URL 验证**: 未验证 base_url 格式

### CLI
1. **多处 unwrap() 调用**: scenario.rs 中多处 unwrap() 可能导致 panic
   - 位置: `scenario.rs:32, 64, 110, 236`
2. **StorageManager 重复创建**: report.rs 中重复创建 StorageManager
3. **PowerShell 路径硬编码**: 硬编码 Windows PowerShell 路径
4. **未完成的命令**: keyboard, mouse, command 模块仅为占位实现

### 执行器
1. **清理时静默忽略错误**: disconnect 错误被静默忽略
   - 位置: `executor/src/runner.rs:211-221`
2. **Custom 动作未实现**: Action::Custom 仅记录警告并跳过

---

## 代码统计

### 模块代码量

| 模块 | 行数 | 文件数 |
|------|------|--------|
| transport | 1,562 | 5 |
| protocol/qmp | 440 | 1 |
| protocol/qga | 381 | 1 |
| protocol/virtio | 653 | 3 |
| protocol/spice | 4,785 | 10 |
| protocol/抽象层 | 279 | 2 |
| executor | **3,207** | 4 |
| storage | 800 | 8 |
| vdiplatform | 650 | 10 |
| verification-server | 1,195 | 5 |
| guest-verifier | 2,910 | 8 |
| cli | 550 | 5 |
| **总计** | **~17,412** | **62** |

### 测试统计

| 模块 | 测试数 | 通过率 |
|------|--------|--------|
| executor | 5 | 100% |
| storage | 8 | 100% |
| transport | 24 | 100% |
| protocol | 74 | 100% |
| vdiplatform | 1 | 100% |
| verification-server | 4 | 100% |
| e2e | 10 | 20% (8 ignored) |
| **总计** | **158** | **95%** |

### TODO/FIXME 统计

| 模块 | TODO | FIXME | 说明 |
|------|------|-------|------|
| protocol (SPICE) | 29 | 0 | SPICE 功能待完善 |
| executor | 4 | 0 | Custom 动作待实现 |
| cli | 5 | 0 | 命令实现待完成 |
| custom protocol | 4 | 0 | 协议待实现 |
| storage | 2 | 0 | Repository 访问器待添加 |
| vdiplatform | 1 | 0 | URL 验证 |
| transport | 1 | 0 | - |
| ssh-executor | 1 | 0 | Host key 验证 |
| **总计** | **47** | **0** |

---

## 版本规划

### v0.5.0 (开发中)
- [x] 多目标场景执行
- [x] 主机/虚拟机选择器
- [x] 并行执行支持
- [x] VDI 平台操作集成
- [x] 验证步骤实现
- [x] VerificationServer 嵌入式集成
- [ ] 测试覆盖率 80%
- [ ] SPICE 协议完善

### v0.4.0 (已完成)
- [x] 基础架构完成
- [x] 核心协议实现 (QMP/QGA/VirtioSerial)
- [x] 数据库层集成
- [x] CLI 工具完善
- [x] VDI 平台基础集成
- [x] Guest 验证器 (Linux/Windows)

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

### 2026-01-03 (v0.5.0-dev)
- **Executor 模块 VDI 智能集成** (+300 行代码)
  - **VDI 平台自动发现**: 优先从 VDI 平台获取虚拟机列表
  - **虚拟机-主机映射持久化**: 自动保存虚拟机与主机的映射关系到数据库
  - **动态主机注册**: 从数据库读取主机信息，自动注册到 TransportManager
  - **智能目标选择**: 如果未指定目标虚拟机，自动从 VDI 获取第一个运行中的虚拟机
  - **SPICE 输入通道支持**: 新增 `InputChannelConfig` 配置输入通道类型 (QMP/SPICE)
- **存储层增强**
  - **HostRepository**: 主机记录的 CRUD 操作
  - **DomainHostMappingRepository**: 虚拟机-主机映射的 CRUD 操作
  - **数据库迁移**: 新增 `hosts` 和 `domain_host_mappings` 表
- **协议层优化**
  - **QMP 协议**: 重构错误处理，改进响应解析
  - **QGA 协议**: 增强 PowerShell 命令执行支持
  - **SPICE 协议**: 简化通道管理，优化连接流程
- **CLI 更新**
  - **配置文件路径**: 从 `test.toml` 迁移到 `config/atp.toml`
  - **VDI 客户端集成**: 场景执行时自动连接 VDI 平台
  - **示例配置**: 新增 `config/atp.toml.example`
- **代码清理**
  - 删除 `test.toml` 和 `test.toml.example`
  - 更新 `.gitignore` 忽略 `config/atp.toml`

### 2026-01-16 (v0.5.1-dev)
- **架构重构**
  - [x] 项目重构为单一 Workspace
  - [x] 移除 Orchestrator 模块 (彻底清理)
  - [x] VDI 批量操作逻辑迁移至 Executor
- **功能增强**
  - [x] VDI 缓存管理器
  - [x] VDI 磁盘位置查询
  - [x] 强制重新分配 (Force Reassignment)
  - [x] GlusterFS 脑裂修复
- **HTTP API**
  - [x] 基础框架搭建 (Axum)

### 2026-01-01 (v0.5.0-dev)
- **Executor 模块重大更新** (+1,224 行代码)
  - **多目标场景执行**: 支持同时对多个虚拟机执行测试场景
    - `TargetSelector`: 支持通配符、正则表达式、列表匹配
    - `TargetSelectorConfig`: 高级选择器，支持 mode/pattern/names/exclude/limit
    - `TargetMode`: Single/All/Pattern/List/Regex 五种模式
    - `run_multi_target()`: 串行或并行执行多目标场景
    - `MultiTargetReport`: 汇总多目标执行结果
  - **主机选择器**: 支持 `target_hosts` 匹配多个 libvirt 主机
  - **并行执行配置**: `ParallelConfig` 控制并发数和失败策略
    - `FailureStrategy`: Continue/StopAll/FailFast
  - **嵌入式验证服务器**: 场景执行时自动启动 VerificationServer
    - `start_verification_server()`: 启动 WebSocket/TCP 服务器
    - `start_guest_verifier()`: 通过 QGA 启动 guest-verifier
    - `create_verification_future()`: 创建验证等待任务
    - 步骤报告支持 `verified` 和 `verification_latency_ms`
  - **VDI 平台操作**: 场景 Action 支持 VDI 操作
    - `VdiCreateDeskPool`, `VdiEnableDeskPool`, `VdiDisableDeskPool`, `VdiDeleteDeskPool`
    - `VdiStartDomain`, `VdiShutdownDomain`, `VdiRebootDomain`, `VdiDeleteDomain`
    - `VdiBindUser`, `VdiGetDeskPoolDomains`
  - **验证步骤**: 场景 Action 支持验证操作
    - `VerifyDomainStatus`: 验证虚拟机状态
    - `VerifyAllDomainsRunning`: 验证桌面池所有虚拟机运行中
    - `VerifyCommandSuccess`: 验证命令执行成功
  - **VerificationConfig**: 场景级验证服务器配置
    - `ws_addr`, `tcp_addr`, `guest_verifier_path`, `connection_timeout`, `vm_id`
  - **Glob 匹配算法**: 实现 `*` 和 `?` 通配符匹配
- **CLI 场景命令更新**
  - 支持验证结果显示 (verified 状态和延迟)
  - 支持验证步骤的彩色输出
- **新增示例场景**
  - `keyboard-verification-test.yaml`: 带验证的键盘输入测试场景
- **架构对比**
  - 完全实现 GUEST_VERIFICATION_SERVER_DESIGN.md 中的设计
  - Executor 集成 VerificationServer 的所有功能
  - 场景支持 VDI 平台操作和验证步骤

### 2025-12-31 (v0.4.2)
- **代码审查: atp-application/cli/src**
  - 发现 5 个未完成命令实现 (keyboard, mouse, command)
  - 识别 unwrap() 调用风险 (scenario.rs:32,64,110,236)
  - 识别 StorageManager 重复创建模式
  - 识别 PowerShell 路径硬编码问题
  - 评估: 代码质量 8/10

- **代码审查: atp-core**
  - **安全问题**: SSH Host Key 验证禁用 (MITM 风险)
  - **安全问题**: MD5 密码哈希 (弱加密)
  - **内存问题**: executor/runner.rs 中 .leak() 导致内存泄漏
  - **功能问题**: 字符到 QKeyCode 映射不完整
  - 识别清理时静默忽略错误
  - 识别 Custom 动作未实现
  - 识别 Storage Repository 访问器待添加
  - 评估: 整体质量 7.4/10

- **更新任务优先级**
  - 新增安全问题修复为高优先级
  - 新增内存泄漏修复为高优先级
  - 新增 CLI 代码质量改进为中优先级
  - 新增文档完善为低优先级

### 2025-12-31 (v0.4.1)
- **项目文档全面审查和更新**
  - 修复 README.md 中指向 docs 目录的文件引用路径
  - 移除对不存在文件的引用 (VDI_CONNECTIVITY_TEST_SUMMARY.md)
  - 更新 docs/README.md 的版本号和统计数据
  - 整理文档结构，确保所有链接正确
- **文档版本同步**
  - 统一所有文档版本号为 v0.4.1
  - 更新最后修改日期为 2025-12-31

### 2025-12-11 (v0.4.1)
- **运行集成测试**
  - 配置本地 libvirtd 测试环境
  - 运行全部 158 个测试，150 个通过
  - 8 个 E2E 测试需要实际虚拟机环境已标记为 ignored
- **修复测试问题**
  - 修复 E2E 测试中缺失的辅助函数 `get_test_vm_name()` 和 `get_test_host_uri()`
  - 修复 `setup_test_runner()` 返回元组解构问题
  - 修复 SPICE packed struct 引用对齐问题 (types.rs)
  - 修复 VirtioSerial XML 解析顺序问题 (source/target 顺序无关)
  - 修复 protocol_tests.rs 中 API 调用问题
- **VDI 平台批量操作功能** (新增)
  - 批量启动/关闭/重启/删除虚拟机
  - 完全克隆虚拟机
  - 从模板批量创建虚拟机 (链接克隆)
  - 批量修改 CPU/内存配置
  - 网络管理 (添加/修改/删除网卡)
  - 完整创建虚拟机 API
- **更新 TODO 文档**
  - 更新测试统计数据
  - 标记完成的测试任务
  - 更新 VDI 平台完成度 (60% → 85%)

### 2025-12-11
- **新增 PowerShell 远程执行 CLI 命令** (`atp ps`)
  - 通过 QGA (QEMU Guest Agent) 协议向 Windows 虚拟机发送命令
  - 支持向单个、多个或所有 Windows 虚拟机发送 PowerShell 命令
  - 命令通过 UTF-16LE Base64 编码传输，兼容 Windows PowerShell `-EncodedCommand`
  - 按主机分组执行，复用 libvirt 连接
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
- **版本历史**: [CHANGELOG.md](CHANGELOG.md)

---

**维护者**: OCloudView ATP Team

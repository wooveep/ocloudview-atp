# Changelog

All notable changes to the OCloudView ATP project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.1] - 2025-12-31

### Documentation
- **项目文档全面审查和更新**
  - 修复 README.md 中指向 docs 目录的文件引用路径
  - 移除对不存在文件的引用 (VDI_CONNECTIVITY_TEST_SUMMARY.md)
  - 更新 docs/README.md 的版本号和统计数据
  - 整理文档结构，确保所有链接正确
  - 删除 docs/archive 目录下的 8 个过时文档
- **文档版本同步**
  - 统一所有文档版本号为 v0.4.1
  - 更新最后修改日期为 2025-12-31
- **创建 CHANGELOG.md**
  - 从 TODO.md 提取版本历史
  - 采用标准 Keep a Changelog 格式

## [0.4.1] - 2025-12-11

### Testing
- **运行集成测试**
  - 配置本地 libvirtd 测试环境
  - 运行全部 158 个测试，150 个通过
  - 8 个 E2E 测试需要实际虚拟机环境已标记为 ignored

### Fixed
- **修复测试问题**
  - 修复 E2E 测试中缺失的辅助函数 `get_test_vm_name()` 和 `get_test_host_uri()`
  - 修复 `setup_test_runner()` 返回元组解构问题
  - 修复 SPICE packed struct 引用对齐问题 (types.rs)
  - 修复 VirtioSerial XML 解析顺序问题 (source/target 顺序无关)
  - 修复 protocol_tests.rs 中 API 调用问题

### Added
- **VDI 平台批量操作功能**
  - 批量启动/关闭/重启/删除虚拟机
  - 完全克隆虚拟机
  - 从模板批量创建虚拟机 (链接克隆)
  - 批量修改 CPU/内存配置
  - 网络管理 (添加/修改/删除网卡)
  - 完整创建虚拟机 API

### Changed
- **更新 TODO 文档**
  - 更新测试统计数据
  - 标记完成的测试任务
  - 更新 VDI 平台完成度 (60% → 85%)

## [0.4.1] - 2025-12-11

### Added
- **新增 PowerShell 远程执行 CLI 命令** (`atp ps`)
  - 通过 QGA (QEMU Guest Agent) 协议向 Windows 虚拟机发送命令
  - 支持向单个、多个或所有 Windows 虚拟机发送 PowerShell 命令
  - 命令通过 UTF-16LE Base64 编码传输，兼容 Windows PowerShell `-EncodedCommand`
  - 按主机分组执行，复用 libvirt 连接

### Changed
- 重新整理 TODO 文档结构
- 更新项目状态和各模块完成度
- 整理近期优先任务
- 更新代码统计数据

## [0.4.0] - 2025-12-08

### Added
- **VDI 平台集成** (完成度 85%)
  - 完整的 VDI 平台 API 集成
  - MD5 密码加密和 Token 认证
  - 主机和虚拟机自动发现
  - VDI + libvirt 数据同步验证
  - 支持多主机管理
- **CLI VDI 命令**
  - `atp vdi verify` - 验证 VDI 与 libvirt 虚拟机状态一致性
  - `atp vdi list-hosts` - 列出主机
  - `atp vdi list-vms` - 列出虚拟机

### Changed
- 完成 Executor/Orchestrator 合并
- E2E 测试框架完成

## [0.3.1] - 2025-12-01

### Added
- **测试配置加载模块** (TestConfig)
  - 统一配置管理系统
  - 支持环境变量、TOML/YAML/JSON 配置文件
  - 优先级: 环境变量 > 配置文件 > 默认值
- **E2E 测试框架**
  - 10 个端到端测试
  - 覆盖 QMP、QGA、SPICE 协议
  - 支持场景加载、错误处理、性能测试
- **SPICE 鼠标操作集成**
  - SPICE 协议鼠标移动和点击
  - 集成到执行器
  - 完成协议层 100% 集成

### Changed
- 项目整体进度: 75% → 78%
- 执行器完成度: 70% → 85%

## [0.3.0] - 2025-11-26

### Added
- **Guest 验证器 Windows 实现**
  - Windows 键盘验证器 (Hook API)
  - Windows 鼠标验证器 (Hook API)
  - Windows 命令执行验证器
  - 跨平台支持 (Linux + Windows)
- **单元测试框架建立**
  - Transport 模块: 24 个测试
  - Protocol 模块: 74 个测试
  - Storage 模块: 8 个测试
  - Verification Server: 4 个测试

## [0.2.5] - 2025-11-25

### Added
- **数据库层集成**
  - SQLite 数据库支持
  - StorageManager 连接管理
  - ReportRepository (测试报告)
  - ScenarioRepository (场景库)
  - BackupManager (备份恢复)
  - 36 个单元测试
- **CLI 报告命令实现**
  - `atp report list` - 列出报告
  - `atp report show` - 显示报告详情
  - `atp report export` - 导出报告
  - `atp report delete` - 删除报告
  - `atp report stats` - 报告统计
  - `atp report cleanup` - 清理旧报告
- **数据库备份命令**
  - `atp backup` - 创建备份
  - `atp restore` - 恢复备份
  - `atp backup list` - 列出备份
  - `atp backup delete` - 删除备份
  - `atp backup cleanup` - 清理旧备份

## [0.2.0] - 2025-11-20

### Added
- **传输层** (Transport Layer) - 85% 完成
  - HostConnection 连接管理
  - 自动重连逻辑 (指数退避)
  - 心跳检测机制
  - ConnectionPool 连接池
  - TransportManager 多主机管理
  - 并发任务执行
- **协议层基础** (Protocol Layer)
  - QMP 协议 100% 完成
  - QGA 协议 100% 完成
  - VirtioSerial 协议 95% 完成
  - SPICE 协议框架 (60% 完成)
- **执行器核心** (Executor)
  - ScenarioRunner 执行引擎
  - 场景加载 (YAML/JSON)
  - 协议集成 (QMP/QGA/SPICE)
  - 执行报告生成

## [0.1.0] - 2025-11-01

### Added
- **项目初始化**
  - Rust workspace 结构
  - 分层架构设计
  - 基础模块框架
- **Guest 验证器 Linux 实现**
  - Linux 键盘验证器 (evdev)
  - Linux 鼠标验证器 (evdev)
  - 命令执行验证器
  - WebSocket/TCP 传输层
- **Verification Server**
  - ClientManager (客户端管理)
  - VerificationService (事件跟踪)
  - WebSocket/TCP 服务器
  - VM ID 身份验证

### Infrastructure
- 基础 CI/CD 配置
- 项目文档框架
- 测试框架搭建

---

## 版本规划

### [0.5.0] - 计划中
- VDI 平台完整集成
- 测试覆盖率达到 80%
- SPICE 协议完善 (RSA 加密、TLS 支持)
- Custom 协议实现

### [0.6.0] - 计划中
- HTTP API (Axum 框架)
- WebSocket 实时推送
- RESTful API 端点
- Swagger 文档
- 性能优化

### [1.0.0] - 目标
- 生产级稳定性
- 完整文档
- Web 控制台
- 性能优化完成
- 完整的测试覆盖

---

## 图例

- **Added**: 新增功能
- **Changed**: 功能变更
- **Deprecated**: 已弃用功能
- **Removed**: 移除的功能
- **Fixed**: 问题修复
- **Security**: 安全性修复
- **Documentation**: 文档更新
- **Testing**: 测试相关
- **Infrastructure**: 基础设施变更

---

**维护者**: OCloudView ATP Team
**项目主页**: [README.md](README.md)
**详细任务**: [TODO.md](TODO.md)

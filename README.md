# OCloudView ATP - 虚拟机输入自动化测试平台

## 系统总体架构

本系统采用分层架构设计，用于虚拟化环境下的输入自动化测试和验证。系统包含以下核心层次：

- **应用层 (Application)**: CLI 工具、场景管理
- **执行层 (Executor)**: 测试执行引擎、编排器
- **协议层 (Protocol)**: QMP、QGA、VirtioSerial 等协议支持
- **传输层 (Transport)**: 连接池、多主机管理
- **存储层 (Storage)**: 测试报告持久化
- **验证层 (Verification)**: Guest 验证服务器和代理

## 项目结构

```
ocloudview-atp/
├── atp-core/                 # 核心框架
│   ├── protocol/            # 协议层 (QMP, QGA, VirtioSerial)
│   ├── transport/           # 传输层 (连接池, 多主机管理)
│   ├── orchestrator/        # 场景编排器
│   ├── executor/            # 测试执行器
│   ├── storage/             # 数据存储层
│   ├── verification-server/ # Guest 验证服务器
│   └── vdiplatform/         # VDI 平台 API 客户端
├── atp-application/          # 应用层
│   ├── cli/                 # 命令行工具
│   ├── scenarios/           # 测试场景
│   └── http-api/            # HTTP API (待实现)
├── guest-verifier/           # Guest 验证组件
│   ├── verifier-core/       # 验证器核心库
│   └── verifier-agent/      # Guest Agent 应用
├── examples/                 # 示例代码
├── docs/                     # 文档
└── config/                   # 配置文件
```

## 核心组件

### ATP CLI (应用层)
- **技术栈**: Rust, Clap, Tokio
- **职责**: 命令行界面、测试场景管理、配置管理

### Protocol Layer (协议层)
- **技术栈**: Rust, Async/Await
- **职责**: QMP/QGA/VirtioSerial 协议实现、键盘/鼠标事件注入

### Transport Layer (传输层)
- **技术栈**: Rust, Libvirt, Tokio
- **职责**: 连接池管理、多主机并发执行、负载均衡

### Orchestrator (编排层)
- **技术栈**: Rust, Async
- **职责**: 场景编排、步骤执行、结果收集

### Executor (执行层)
- **技术栈**: Rust, Tokio
- **职责**: 测试任务执行、并发控制、错误处理

### Guest Verifier (验证层)
- **技术栈**: Rust, WebSocket/TCP, Evdev
- **职责**: Guest 内输入事件捕获和验证

## 数据流向与验证逻辑

1. **初始化阶段**: CLI 读取配置，建立到 Libvirt 的连接池
2. **场景加载**: 加载 YAML/JSON 格式的测试场景定义
3. **编排执行**: Orchestrator 根据场景编排测试步骤
4. **协议注入**: 通过 QMP/QGA 协议向 VM 注入输入事件或执行命令
5. **Guest 验证**: Guest Verifier Agent 捕获实际输入并上报
6. **结果比对**: Executor 对比预期与实际结果
7. **报告生成**: 存储层持久化测试报告

## 快速开始

### VDI + libvirt 集成测试 🚀

测试 VDI 平台与 libvirt 的完整集成：

#### 方式 1: 使用 CLI 命令（推荐） ⭐

```bash
# 验证 VDI 与 libvirt 虚拟机状态一致性
./atp-application/target/release/atp vdi verify

# 列出主机
./atp-application/target/release/atp vdi list-hosts

# 列出虚拟机
./atp-application/target/release/atp vdi list-vms
```

#### 方式 2: 使用示例程序

```bash
cargo run --example vdi_libvirt_integration --manifest-path atp-core/executor/Cargo.toml
```

#### 配置文件 (`test.toml`)

```toml
[vdi]
base_url = "http://192.168.41.51:8088"
username = "admin"
password = "your_password"
verify_ssl = false
connect_timeout = 10
```

这将自动：
- 登录 VDI 平台获取 Token
- 从 VDI 获取主机列表
- 连接到主机的 libvirtd
- 对比 VDI 和 libvirt 的虚拟机信息

**详细文档**: [CLI VDI 命令使用指南](docs/CLI_VDI_COMMANDS.md) | [VDI_LIBVIRT_INTEGRATION.md](VDI_LIBVIRT_INTEGRATION.md)

### 前置要求
- Rust 1.70+
- QEMU/KVM
- Libvirt
- SQLite 3 (用于报告存储)

### 构建项目
```bash
# 构建所有组件
cargo build --release

# 构建 CLI
cd atp-application/cli
cargo build --release

# 构建 Guest Verifier Agent
cd guest-verifier/verifier-agent
cargo build --release
```

### 运行示例
```bash
# 运行键盘测试场景
./target/release/atp-cli scenario run --file examples/keyboard_basic.json

# 查看测试报告
./target/release/atp-cli report list
```

## 技术特性

### VDI 平台集成 ✅ (2025-12-08)
- 完整的 VDI 平台 API 集成
- MD5 密码加密和 Token 认证
- 主机和虚拟机自动发现
- VDI + libvirt 数据同步验证
- 支持多主机管理
- 详细文档: [VDI_LIBVIRT_INTEGRATION.md](VDI_LIBVIRT_INTEGRATION.md)

### 分层架构
- 清晰的职责分离
- 模块化设计
- 易于扩展和测试

### 协议支持
- QMP (QEMU Machine Protocol)
- QGA (QEMU Guest Agent)
- VirtioSerial 自定义协议
- SPICE/VNC (计划中)

### 高并发支持
- 连接池和多主机管理
- 基于 Tokio 的异步运行时
- 并发策略配置

### 可验证性
- Guest 内核层事件捕获 (evdev)
- 闭环验证架构
- 详细的测试报告和指标

## 许可证

参见 LICENSE 文件

## 文档

详细文档请参见 `docs/` 目录。

### VDI 平台集成文档
- [VDI + libvirt 集成报告](VDI_LIBVIRT_INTEGRATION.md) - 完整集成测试报告
- [VDI 连通性测试总结](VDI_CONNECTIVITY_TEST_SUMMARY.md) - API 测试和连通性验证
- [VDI 登录 API 指南](docs/VDI_LOGIN_API_GUIDE.md) - 认证和 Token 使用
- [VDI API 发现文档](docs/VDI_API_DISCOVERY.md) - Swagger API 文档
- [测试配置指南](docs/TESTING_CONFIG_GUIDE.md) - test.toml 配置说明

### 架构文档
- [系统架构设计](docs/ARCHITECTURE.md)
- [协议层设计](docs/PROTOCOL_LAYER.md)
- [传输层设计](docs/TRANSPORT_LAYER.md)

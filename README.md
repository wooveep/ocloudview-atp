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

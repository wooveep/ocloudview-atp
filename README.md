# OCloudView ATP - 虚拟机输入自动化测试平台

## 系统总体架构

本系统采用经典的"控制端-代理端"（Controller-Agent）闭环架构，用于虚拟化环境下的输入自动化测试。系统由三个核心域组成：

- **宿主机控制域（Host Controller Domain）**
- **虚拟化管理域（Hypervisor Domain）**
- **客户机验证域（Guest Verification Domain）**

## 项目结构

```
ocloudview-atp/
├── test-controller/          # Rust 控制端
│   ├── src/
│   │   ├── qmp/             # QMP 协议封装
│   │   ├── libvirt/         # Libvirt 集成
│   │   ├── keymapping/      # 键值映射模块
│   │   ├── vm_actor/        # VM Actor 任务编排
│   │   ├── orchestrator/    # 测试编排器
│   │   └── main.rs          # 主程序入口
│   └── Cargo.toml
├── guest-agent-web/          # Web Guest Agent
│   ├── server/              # WebSocket 服务端
│   └── client/              # 浏览器测试页面
├── guest-agent-native/       # Native Guest Agent
│   └── src/                 # Rust 原生代理
├── docs/                     # 文档
└── config/                   # 配置文件
```

## 核心组件

### Test Controller (Host, Rust)
- **技术栈**: Rust, Tokio, Serde, Libvirt-rs
- **职责**: 测试编排、QMP 指令注入、比对验证、结果聚合

### Libvirt Daemon (Host, C)
- **技术栈**: C, RPC
- **职责**: 虚拟机生命周期管理、Socket 路径发现、权限控制

### QEMU Process (Host, C)
- **技术栈**: C, KVM
- **职责**: 虚拟硬件模拟（PS/2, USB HID）、中断注入

### Guest Agent (Web)
- **技术栈**: HTML5, JavaScript, WebSocket
- **职责**: 浏览器层面的 DOM 事件捕获 (keydown, keyup)

### Guest Agent (Native)
- **技术栈**: Rust, WinAPI / Linux Evdev
- **职责**: 操作系统内核/用户态层面的 Scancode 捕获

## 数据流向与闭环验证逻辑

1. **连接建立阶段**: Rust Controller 通过 Libvirt API 查询 QMP Socket 路径，建立异步连接
2. **指令构造阶段**: 将字符映射为 QEMU QCode，封装为 QMP JSON 指令
3. **注入阶段**: 通过长连接发送 send-key 或 input-send-event 指令
4. **模拟与中断阶段**: QEMU 驱动虚拟键盘设备产生硬件中断
5. **捕获阶段**: Guest Agent 捕获并回传键盘事件
6. **验证阶段**: Controller 对比注入指令与捕获数据，判定测试结果

## 快速开始

### 前置要求
- Rust 1.70+
- QEMU/KVM
- Libvirt
- Node.js (用于 Web Agent)

### 构建 Test Controller
```bash
cd test-controller
cargo build --release
```

### 运行 Web Guest Agent
```bash
cd guest-agent-web/server
npm install
npm start
```

### 运行 Native Guest Agent
```bash
cd guest-agent-native
cargo build --release
```

## 技术特性

### 键值映射
支持多层级键值映射：
- 语义层 (Semantic Level)
- QEMU 内部表示层 (QCode)
- 硬件扫描码层 (Scancode)
- Guest 内核层 (Linux Input / Windows ScanCode)
- 应用层 (Browser/GUI)

### 高并发支持
- 基于 Tokio 的异步运行时
- Actor 模型实现并发测试
- 无共享架构 (Shared-Nothing Architecture)

### 可信事件验证
- Web Agent 能够验证 `isTrusted` 属性
- 完整模拟真实硬件输入
- 支持安全 API 测试（全屏、剪贴板等）

## 许可证

参见 LICENSE 文件

## 文档

详细文档请参见 `docs/` 目录。

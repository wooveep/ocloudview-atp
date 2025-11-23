# 系统总体架构设计

## 1. 概述

本系统采用经典的"控制端-代理端"（Controller-Agent）闭环架构，但在通信协议与实现技术上进行了深度的垂直优化。系统由三个核心域组成：

- **宿主机控制域（Host Controller Domain）**
- **虚拟化管理域（Hypervisor Domain）**
- **客户机验证域（Guest Verification Domain）**

## 2. 架构组件概览

| 组件名称 | 运行位置 | 核心技术栈 | 职责描述 |
|---------|---------|-----------|---------|
| Test Controller | Host (Linux) | Rust, Tokio, Serde, Libvirt-rs | 测试编排、QMP 指令注入、比对验证、结果聚合 |
| Libvirt Daemon | Host (Linux) | C, RPC | 虚拟机生命周期管理、Socket 路径发现、权限控制 |
| QEMU Process | Host (Linux) | C, KVM | 虚拟硬件模拟（PS/2, USB HID）、中断注入 |
| Guest Agent (Web) | Guest OS | HTML5, JavaScript, WebSocket | 浏览器层面的 DOM 事件捕获 (keydown, keyup) |
| Guest Agent (Native) | Guest OS | Rust, WinAPI / Linux Evdev | 操作系统内核/用户态层面的 Scancode 捕获 |

## 3. 数据流向与闭环验证逻辑

整个测试流程构成了一个严格的时序闭环：

### 3.1 连接建立阶段
Rust Controller 启动，通过 Libvirt API 查询目标 VM 的 QMP Socket 路径，并建立异步 Unix Stream 连接；同时，等待 Guest Agent 通过 TCP/WebSocket 反向注册上线。

### 3.2 指令构造阶段
测试脚本生成按键序列（例如 "Hello World"）。Controller 将字符映射为 QEMU 能够识别的 QCode，并封装为 QMP JSON 指令包。

### 3.3 注入阶段
Controller 通过长连接通道发送 `send-key` 或 `input-send-event` 指令。

### 3.4 模拟与中断阶段
QEMU 解析指令，驱动虚拟键盘设备（如 i8042 PS/2 控制器）产生硬件中断信号，注入 Guest CPU。

### 3.5 捕获阶段

**Web 场景：**
- Guest OS 将中断转换为系统消息，浏览器捕获 KeyboardEvent
- Agent 序列化事件并通过 WebSocket 回传

**Native 场景：**
- Agent 读取 `/dev/input/event*` 或使用 Windows Hook
- 获取原始 Scancode 并回传

### 3.6 验证阶段
Controller 接收回传数据，对比"注入指令"与"捕获数据"的键值一致性、时序延迟（Latency），并判定测试结果。

## 4. 核心技术实现：Rust 控制端

### 4.1 异步运行时与 QMP 协议封装

QMP 协议基于 JSON-RPC，是一套严格的请求-响应模型。在 Rust 中实现 QMP 客户端：

- 使用 `serde` 和 `serde_json` 将 JSON 报文映射为强类型结构体
- 使用 `tokio::net::UnixStream` 建立异步连接
- 实现基于换行符的帧解码器（使用 `LinesCodec` + `Framed`）

### 4.2 Libvirt 与 QMP 的混合使用策略

采用"Libvirt 发现，QMP 控制"的混合策略：

1. 使用 `virt` crate 连接 `qemu:///system`
2. 通过 `virDomainGetXMLDesc` 获取 VM 的 XML 配置
3. 解析 XML 获取 QMP Socket 路径
4. 通过 QMP Socket 直接控制 VM

### 4.3 高并发下的任务编排

- 为每一台 VM 启动一个 `VmActor` 任务
- 每个 Actor 维护自己的 QMP 连接和 WebSocket 监听端口
- 主线程（`Orchestrator`）通过 `mpsc::channel` 下发测试任务
- 采用"无共享架构"（Shared-Nothing Architecture）

## 5. 键值映射（Key Mapping）

### 5.1 映射层级

1. **语义层 (Semantic Level)**: 测试脚本意图输入字符 'a'
2. **QEMU 内部表示层 (QCode)**: 字符 'a' 对应 `Q_KEY_CODE_A`
3. **硬件扫描码层 (Scancode)**:
   - PS/2 键盘：转换为 Set 1 或 Set 2 扫描码（'A' -> 0x1E）
   - USB HID 键盘：转换为 HID Usage ID（'A' -> 0x04）
4. **Guest 内核层**: Linux `KEY_A = 30` / Windows ScanCode
5. **应用层**: 浏览器 `event.key = 'a'` 和 `event.code = 'KeyA'`

### 5.2 映射矩阵

| 字符/功能 | QEMU QCode | Linux Evdev Code | USB HID Usage | JS event.code |
|----------|-----------|-----------------|---------------|--------------|
| a | a | 30 (KEY_A) | 0x04 | "KeyA" |
| Enter | ret | 28 (KEY_ENTER) | 0x28 | "Enter" |
| Space | spc | 57 (KEY_SPACE) | 0x2C | "Space" |
| Left Shift | shift | 42 (KEY_LEFTSHIFT) | 0xE1 | "ShiftLeft" |

### 5.3 QMP 指令选择

- **send-key**: 高级指令，自动模拟按下和释放，适合基础测试
- **input-send-event**: 低级指令，允许独立发送 btn-down 和 btn-up 事件，适合复杂组合键测试

## 6. Guest 端验证代理（Agent）

### 6.1 Web Agent

**技术实现：**
- 轻量级 Web Server（Node.js + Express）
- WebSocket 服务端（ws 库）
- HTML5 测试页面

**核心优势：**
- QMP 注入的按键具有 `isTrusted = true` 属性
- 能够触发只响应可信事件的浏览器安全机制（全屏 API、剪贴板访问）

**数据结构：**
```json
{
  "type": "keydown",
  "key": "a",
  "code": "KeyA",
  "keyCode": 65,
  "ctrlKey": false,
  "shiftKey": false,
  "timeStamp": 1678892301204,
  "isTrusted": true
}
```

### 6.2 Native Agent

**技术实现：**
- Rust 跨平台实现
- Linux: evdev 读取 `/dev/input/event*`
- Windows: Windows Hook API (`SetWindowsHookEx`)

**数据捕获：**
- 原始 Scancode
- 键码（Keycode）
- 修饰键状态
- 时间戳

## 7. 关键洞察

### 7.1 流式协议处理
QMP 协议的流式特性意味着数据包可能分片到达或粘连。必须实现基于换行符的帧解码器。

### 7.2 权限控制
默认情况下，Libvirt 对 QMP Socket 施加了权限控制。测试程序需要：
- 以 root 运行，或
- 将测试用户加入 `libvirt-qemu` 组
- 确保 Socket 文件的 ACL 允许读写

### 7.3 键盘布局感知
单纯依赖 ASCII 码是错误的，因为按键的物理位置（ScanCode）与字符（Char）的关系受键盘布局影响。

例如：
- US 布局：Shift + 2 → '@'
- UK 布局：Shift + 2 → '"'

### 7.4 Actor 模型的优势
采用 Actor 模型（Tokio 任务 + Channel）确保某一台 VM 的连接超时或崩溃不会影响其他 VM 的测试进程。

## 8. 扩展性设计

### 8.1 支持 50+ VM 并发测试
- 每个 VM 独立的 Actor 任务
- 异步 I/O 避免线程阻塞
- 无共享架构确保隔离性

### 8.2 多键盘布局支持
- `LayoutMapper` 模块
- 输入：目标字符 + 键盘布局
- 输出：按键序列

### 8.3 可扩展的测试用例框架
- 测试用例定义为独立模块
- 支持自定义验证逻辑
- 结果聚合与报告生成

## 9. 性能指标

- **按键注入延迟**: < 10ms
- **事件捕获延迟**: < 5ms
- **端到端延迟**: < 20ms
- **并发 VM 数**: 50+
- **测试吞吐量**: 1000+ 按键/秒

## 10. 安全考虑

1. **QMP Socket 权限**: 严格限制访问权限
2. **WebSocket 认证**: 实现 Token 认证机制
3. **输入验证**: 防止 QMP 注入攻击
4. **日志审计**: 记录所有操作日志

## 11. 未来展望

- 支持触摸屏/鼠标输入测试
- 集成性能分析工具
- 支持云端测试集群
- AI 驱动的测试用例生成

# 开发指南

## 环境准备

### 系统要求

**Host 控制端：**
- Linux (推荐 Ubuntu 20.04+)
- Rust 1.70+
- QEMU/KVM 6.0+
- Libvirt 7.0+

**Guest 客户端：**
- Linux 或 Windows 虚拟机
- Node.js 16+ (Web Agent)
- Rust 1.70+ (Native Agent)

### 安装依赖

#### Ubuntu/Debian

```bash
# 安装 QEMU/KVM 和 Libvirt
sudo apt update
sudo apt install qemu-kvm libvirt-daemon-system libvirt-clients bridge-utils

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Node.js
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt install -y nodejs

# 将当前用户加入 libvirt 组
sudo usermod -aG libvirt $USER
sudo usermod -aG kvm $USER

# 重新登录以应用组权限
```

## 构建项目

### 1. 构建 Test Controller

```bash
cd test-controller
cargo build --release

# 运行测试
cargo test

# 运行（需要 Libvirt 环境）
sudo cargo run --release
```

### 2. 构建 Web Guest Agent

```bash
cd guest-agent-web/server
npm install

# 开发模式
npm run dev

# 生产模式
npm start
```

### 3. 构建 Native Guest Agent

```bash
cd guest-agent-native
cargo build --release

# Linux 需要 root 权限访问 /dev/input
sudo ./target/release/guest-agent-native --server ws://host-ip:8081
```

## 开发工作流

### 1. 设置虚拟机

使用 `virt-manager` 或命令行创建测试虚拟机：

```bash
# 创建虚拟机（示例）
virt-install \
  --name test-vm-01 \
  --ram 2048 \
  --disk path=/var/lib/libvirt/images/test-vm-01.qcow2,size=20 \
  --vcpus 2 \
  --os-type linux \
  --os-variant ubuntu20.04 \
  --network bridge=virbr0 \
  --graphics vnc \
  --cdrom /path/to/ubuntu-20.04.iso
```

### 2. 查找 QMP Socket

```bash
# 列出所有虚拟机
virsh list --all

# 查看虚拟机 XML 配置
virsh dumpxml test-vm-01 | grep monitor

# QMP Socket 通常位于
ls /var/lib/libvirt/qemu/domain-*-*/monitor.sock
```

### 3. 手动测试 QMP

使用 `socat` 或 `nc` 手动连接 QMP Socket：

```bash
# 使用 socat
sudo socat - UNIX-CONNECT:/var/lib/libvirt/qemu/domain-1-test-vm-01/monitor.sock

# 输入 QMP 命令
{"execute": "qmp_capabilities"}
{"execute": "query-status"}
{"execute": "send-key", "arguments": {"keys": [{"type": "qcode", "data": "a"}]}}
```

### 4. 运行完整测试流程

```bash
# 1. 启动虚拟机
virsh start test-vm-01

# 2. 在虚拟机内启动 Guest Agent
# (Web) 打开浏览器访问 http://localhost:8080/test.html
# (Native) sudo ./guest-agent-native --server ws://host-ip:8081

# 3. 在 Host 上启动 Test Controller
cd test-controller
sudo cargo run --release
```

## 代码结构

### Test Controller

```
test-controller/src/
├── main.rs                 # 主程序入口
├── qmp/                    # QMP 协议实现
│   ├── mod.rs
│   ├── client.rs          # QMP 客户端
│   └── protocol.rs        # 协议定义
├── libvirt/               # Libvirt 集成
│   ├── mod.rs
│   └── manager.rs         # Libvirt 管理器
├── keymapping/            # 键值映射
│   ├── mod.rs
│   ├── layout.rs          # 键盘布局
│   └── mapper.rs          # 映射器
├── vm_actor/              # VM Actor
│   ├── mod.rs
│   ├── actor.rs           # Actor 实现
│   └── message.rs         # 消息定义
└── orchestrator/          # 测试编排
    ├── mod.rs
    └── orchestrator.rs    # 编排器
```

### Web Guest Agent

```
guest-agent-web/
├── server/
│   ├── server.js          # WebSocket 服务器
│   └── package.json
└── client/
    ├── test.html          # 测试页面
    └── agent.js           # Agent 客户端
```

### Native Guest Agent

```
guest-agent-native/src/
├── main.rs                # 主程序
├── capture/               # 输入捕获
│   ├── mod.rs
│   ├── linux.rs           # Linux evdev
│   └── windows.rs         # Windows Hook
└── websocket/             # WebSocket 客户端
    └── mod.rs
```

## 调试技巧

### 1. 启用详细日志

```bash
# Rust 程序
RUST_LOG=debug cargo run

# 查看 Libvirt 日志
sudo journalctl -u libvirtd -f

# 查看 QEMU 日志
sudo cat /var/log/libvirt/qemu/test-vm-01.log
```

### 2. 使用 Rust 调试器

```bash
# 安装 rust-gdb
rustup component add rust-src

# 调试
rust-gdb target/debug/test-controller
```

### 3. WebSocket 调试

在浏览器开发者工具中查看 WebSocket 连接：
- Chrome/Edge: F12 → Network → WS
- Firefox: F12 → Network → WS

### 4. 监控 QMP 通信

使用 `socat` 创建 QMP 代理进行监控：

```bash
# 创建代理
sudo socat -v UNIX-LISTEN:/tmp/qmp-monitor.sock,fork \
  UNIX-CONNECT:/var/lib/libvirt/qemu/domain-1-test/monitor.sock
```

## 常见问题

### Q1: 无法连接到 QMP Socket

**解决方案：**
1. 检查虚拟机是否运行：`virsh list`
2. 检查 Socket 文件权限：`ls -l /var/lib/libvirt/qemu/domain-*/monitor.sock`
3. 将用户加入 libvirt 组：`sudo usermod -aG libvirt $USER`

### Q2: 按键注入失败

**解决方案：**
1. 检查虚拟机是否有键盘设备：`virsh dumpxml test-vm | grep input`
2. 确认 QEMU 版本支持 `send-key` 命令
3. 检查键盘布局映射是否正确

### Q3: Guest Agent 无法连接

**解决方案：**
1. 检查网络连通性：`ping host-ip`
2. 检查防火墙规则：`sudo ufw status`
3. 确认 WebSocket 端口已开放：`netstat -tlnp | grep 8081`

### Q4: Linux evdev 权限不足

**解决方案：**
```bash
# 查看输入设备
ls -l /dev/input/

# 方法1：使用 root 运行
sudo ./guest-agent-native

# 方法2：添加 udev 规则
echo 'KERNEL=="event*", MODE="0666"' | sudo tee /etc/udev/rules.d/99-input.rules
sudo udevadm control --reload-rules
```

## 性能优化

### 1. 减少延迟

- 使用 `input-send-event` 替代 `send-key` 以获得更精确的时序控制
- 调整 Tokio 运行时线程数：`tokio::runtime::Builder::new_multi_thread().worker_threads(8)`

### 2. 提高吞吐量

- 批量发送 QMP 命令
- 使用连接池管理多个 QMP 连接
- 启用 WebSocket 压缩

### 3. 内存优化

- 限制事件日志大小
- 使用循环缓冲区存储历史事件
- 定期清理已完成的测试结果

## 贡献指南

1. Fork 本仓库
2. 创建特性分支：`git checkout -b feature/my-feature`
3. 提交更改：`git commit -am 'Add some feature'`
4. 推送到分支：`git push origin feature/my-feature`
5. 创建 Pull Request

### 代码风格

- Rust: 遵循 `rustfmt` 标准
- JavaScript: 遵循 ESLint 配置
- 提交信息：遵循 Conventional Commits

### 测试要求

- 所有新功能必须包含单元测试
- 集成测试覆盖率 > 80%
- 性能测试验证关键路径

## 参考资源

- [QEMU QMP 文档](https://qemu.readthedocs.io/en/latest/interop/qmp-intro.html)
- [Libvirt API 文档](https://libvirt.org/html/index.html)
- [Tokio 异步运行时](https://tokio.rs/)
- [Linux evdev 文档](https://www.kernel.org/doc/html/latest/input/input.html)
- [Windows Hook API](https://docs.microsoft.com/en-us/windows/win32/winmsg/hooks)

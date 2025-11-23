# 快速入门指南

本指南将帮助你快速搭建并运行 OCloudView ATP 虚拟机输入自动化测试平台。

## 前置条件

- Linux Host（推荐 Ubuntu 20.04+）
- QEMU/KVM 和 Libvirt 已安装
- Rust 1.70+
- Node.js 16+
- 至少一台正在运行的虚拟机

## 第一步：克隆仓库

```bash
git clone https://github.com/your-org/ocloudview-atp.git
cd ocloudview-atp
```

## 第二步：构建 Test Controller

```bash
cd test-controller
cargo build --release
cd ..
```

## 第三步：设置 Web Guest Agent

### 在 Host 上启动 WebSocket 服务器

```bash
cd guest-agent-web/server
npm install
npm start
```

服务器将在以下端口启动：
- HTTP: `http://localhost:8080`
- WebSocket: `ws://localhost:8081`

### 在 Guest VM 中打开测试页面

1. 在虚拟机中打开浏览器
2. 访问 `http://<host-ip>:8080/test.html`
3. 确认页面显示"已连接"状态

## 第四步：运行测试

### 查看可用的虚拟机

```bash
virsh list --all
```

### 启动 Test Controller

```bash
cd test-controller
sudo cargo run --release
```

Controller 将：
1. 自动发现正在运行的虚拟机
2. 连接到每个 VM 的 QMP Socket
3. 等待 Guest Agent 连接
4. 运行测试用例

## 验证测试

在 Guest VM 的浏览器测试页面中，你应该能看到：

1. **状态指示器变为绿色**：表示 WebSocket 已连接
2. **事件日志开始填充**：显示捕获的键盘事件
3. **isTrusted = true**：表示这些是真实的硬件输入

## 示例测试场景

### 场景 1：简单文本输入

Test Controller 发送 "Hello World"，验证 Guest Agent 收到的按键序列是否匹配。

### 场景 2：组合键测试

测试 Ctrl+C, Ctrl+V 等组合键，验证修饰键状态。

### 场景 3：特殊键测试

测试 Enter, Tab, Backspace 等特殊键的功能。

## 高级选项

### 使用 Native Guest Agent

如果需要在操作系统层面捕获输入（而非浏览器层面）：

```bash
cd guest-agent-native
cargo build --release

# 在 Guest VM 中运行
sudo ./target/release/guest-agent-native --server ws://<host-ip>:8081
```

### 配置 Test Controller

编辑 `config/controller.toml` 来自定义配置：

```toml
[general]
log_level = "debug"

[keymapping]
default_layout = "en-US"
key_interval = 50

[testing]
max_concurrent_vms = 10
```

### 批量测试多个 VM

```bash
# 启动多个虚拟机
for i in {1..5}; do
    virsh start test-vm-0$i
done

# Controller 会自动发现并测试所有 VM
sudo cargo run --release
```

## 故障排查

### 问题 1: 无法连接到 QMP Socket

```bash
# 检查 Socket 权限
sudo ls -l /var/lib/libvirt/qemu/domain-*/monitor.sock

# 将用户加入 libvirt 组
sudo usermod -aG libvirt $USER
newgrp libvirt
```

### 问题 2: Guest Agent 无法连接

```bash
# 检查防火墙
sudo ufw allow 8081/tcp

# 检查网络连通性
ping <host-ip>

# 在 Host 上检查 WebSocket 服务器
netstat -tlnp | grep 8081
```

### 问题 3: 按键注入无响应

1. 确认虚拟机窗口是否获得焦点
2. 检查虚拟机是否安装了键盘设备：
   ```bash
   virsh dumpxml test-vm-01 | grep input
   ```
3. 尝试手动 QMP 测试：
   ```bash
   sudo socat - UNIX-CONNECT:/var/lib/libvirt/qemu/domain-1-test-vm-01/monitor.sock
   {"execute": "qmp_capabilities"}
   {"execute": "send-key", "arguments": {"keys": [{"type": "qcode", "data": "a"}]}}
   ```

## 查看日志

### Test Controller 日志

```bash
RUST_LOG=debug cargo run
```

### Libvirt 日志

```bash
sudo journalctl -u libvirtd -f
```

### QEMU 日志

```bash
sudo tail -f /var/log/libvirt/qemu/test-vm-01.log
```

## 下一步

- 阅读 [架构设计文档](ARCHITECTURE.md) 了解系统工作原理
- 阅读 [开发指南](DEVELOPMENT.md) 学习如何扩展功能
- 查看 [API 文档](API.md) 了解编程接口

## 获取帮助

- GitHub Issues: https://github.com/your-org/ocloudview-atp/issues
- 文档: https://ocloudview-atp.readthedocs.io

## 许可证

本项目采用 MIT 或 Apache-2.0 双重许可。详见 LICENSE 文件。

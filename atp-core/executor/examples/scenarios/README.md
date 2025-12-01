# ATP Executor 测试场景

这个目录包含用于端到端测试的场景文件。

## 场景列表

### 1. 基础键盘输入测试 (`01-basic-keyboard.yaml`)
- **功能**: 测试 QMP 协议的键盘输入
- **协议**: QMP
- **步骤**: 5 步
- **要求**: QMP socket 配置

### 2. 命令执行测试 (`02-command-execution.yaml`)
- **功能**: 测试 QGA 命令执行
- **协议**: QGA
- **步骤**: 5 步
- **要求**: qemu-guest-agent 已安装

### 3. 鼠标操作测试 (`03-mouse-operations.yaml`)
- **功能**: 测试 SPICE 鼠标控制
- **协议**: SPICE
- **步骤**: 7 步
- **要求**: SPICE 显示配置

### 4. 混合协议测试 (`04-mixed-protocols.yaml`)
- **功能**: 综合测试三种协议集成
- **协议**: QMP + QGA + SPICE
- **步骤**: 10 步
- **要求**: 所有协议配置完整

### 5. 错误处理测试 (`05-error-handling.yaml`)
- **功能**: 测试错误处理机制
- **协议**: QGA
- **步骤**: 4 步（预期在第2步失败）
- **要求**: qemu-guest-agent 已安装

## 使用方法

### 前置准备

1. **启动 libvirtd**:
   ```bash
   sudo systemctl start libvirtd
   sudo systemctl status libvirtd
   ```

2. **准备测试虚拟机**:
   - 确保虚拟机正在运行
   - 虚拟机需要配置 QMP socket
   - 虚拟机内安装 qemu-guest-agent:
     ```bash
     # Ubuntu/Debian
     sudo apt-get install qemu-guest-agent
     sudo systemctl start qemu-guest-agent

     # CentOS/RHEL
     sudo yum install qemu-guest-agent
     sudo systemctl start qemu-guest-agent
     ```
   - 配置 SPICE 显示（可选，用于鼠标测试）

3. **设置环境变量**:
   ```bash
   export ATP_TEST_VM=your-vm-name      # 你的虚拟机名称
   export ATP_TEST_HOST=qemu:///system  # libvirt URI (可选)
   ```

### 运行端到端测试

1. **运行所有 E2E 测试**:
   ```bash
   cd /home/cloudyi/ocloudview-atp/atp-core/executor
   cargo test --test e2e_tests -- --nocapture --ignored
   ```

2. **运行特定测试**:
   ```bash
   # 键盘输入测试
   cargo test --test e2e_tests test_qmp_keyboard_input -- --nocapture --ignored

   # 命令执行测试
   cargo test --test e2e_tests test_qga_command_execution -- --nocapture --ignored

   # 鼠标操作测试
   cargo test --test e2e_tests test_spice_mouse_operations -- --nocapture --ignored

   # 混合协议测试
   cargo test --test e2e_tests test_mixed_protocol_scenario -- --nocapture --ignored

   # 错误处理测试
   cargo test --test e2e_tests test_command_failure_handling -- --nocapture --ignored
   ```

3. **运行场景文件加载测试** (不需要实际虚拟机):
   ```bash
   cargo test --test e2e_tests test_load_scenario -- --nocapture
   ```

### 使用 CLI 运行场景

如果你已经构建了 ATP CLI，可以直接运行场景文件：

```bash
# 构建 CLI
cd /home/cloudyi/ocloudview-atp
cargo build --release

# 运行场景
./target/release/atp scenario run \
  --file atp-core/executor/examples/scenarios/01-basic-keyboard.yaml \
  --vm your-vm-name
```

## 自定义场景

你可以基于这些示例创建自己的测试场景：

```yaml
name: "my-custom-scenario"
description: "自定义测试场景"
target_host: "qemu:///system"
target_domain: "my-vm"

tags:
  - "custom"
  - "test"

steps:
  - name: "步骤描述"
    action:
      type: wait  # 动作类型: wait, send_key, send_text, mouse_click, exec_command
      duration: 1
    verify: false
    timeout: 10  # 可选，单位：秒
```

### 支持的动作类型

1. **send_key** - 发送单个按键
   ```yaml
   action:
     type: send_key
     key: "ret"  # QKeyCode 名称
   ```

2. **send_text** - 发送文本字符串
   ```yaml
   action:
     type: send_text
     text: "Hello World"
   ```

3. **mouse_click** - 鼠标点击
   ```yaml
   action:
     type: mouse_click
     x: 100
     y: 200
     button: "left"  # left, right, middle
   ```

4. **exec_command** - 执行 Shell 命令
   ```yaml
   action:
     type: exec_command
     command: "echo 'test'"
   ```

5. **wait** - 等待指定时间
   ```yaml
   action:
     type: wait
     duration: 5  # 秒
   ```

## 故障排查

### 常见问题

1. **libvirt 连接失败**:
   ```
   Error: Failed to connect to libvirt
   ```
   解决方案:
   - 确保 libvirtd 服务正在运行
   - 检查权限: `sudo usermod -a -G libvirt $USER`
   - 重新登录或 `newgrp libvirt`

2. **QMP 连接失败**:
   ```
   Error: QMP protocol connection failed
   ```
   解决方案:
   - 确认虚拟机配置了 QMP socket
   - 检查 socket 路径权限
   - 查看虚拟机 XML: `virsh dumpxml your-vm`

3. **QGA 命令执行失败**:
   ```
   Error: QGA exec_shell failed
   ```
   解决方案:
   - 确认 qemu-guest-agent 已安装并运行
   - 在 VM 内检查: `sudo systemctl status qemu-guest-agent`
   - 检查 libvirt 配置: `virsh qemu-agent-command your-vm '{"execute":"guest-ping"}'`

4. **SPICE 连接失败**:
   ```
   Error: SPICE protocol connection failed
   ```
   解决方案:
   - 确认虚拟机配置了 SPICE 显示
   - 检查端口是否开放
   - 查看虚拟机 XML 中的 SPICE 配置

### 调试模式

启用详细日志输出：

```bash
RUST_LOG=debug cargo test --test e2e_tests test_name -- --nocapture --ignored
```

### 查看测试报告

测试执行后会输出 JSON 格式的报告，包含：
- 总执行时间
- 每个步骤的状态和耗时
- 命令输出
- 错误信息

示例：
```json
{
  "scenario_name": "basic-keyboard-test",
  "description": "QMP 键盘输入基础测试",
  "passed": true,
  "steps_executed": 5,
  "passed_count": 5,
  "failed_count": 0,
  "duration_ms": 5234,
  "steps": [
    {
      "step_index": 0,
      "description": "1. 发送 Enter 键",
      "status": "Success",
      "duration_ms": 123,
      "error": null,
      "output": null
    }
    // ...
  ]
}
```

## 贡献

欢迎添加更多测试场景！请确保：
- 使用清晰的场景名称和描述
- 为每个步骤添加说明性名称
- 在 README 中添加场景文档
- 测试通过后再提交

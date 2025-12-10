# 鼠标操作功能使用指南

**文档版本**: v1.0
**最后更新**: 2025-12-01
**状态**: ✅ 已完成

---

## 概述

OCloudView ATP 现已完全支持鼠标操作功能，通过集成 **SPICE 协议**实现真实的鼠标移动和点击。本文档介绍如何在测试场景中使用鼠标操作。

---

## 功能特性

### ✅ 已实现功能

1. **SPICE 协议集成** - 主要方案
   - 真实的鼠标移动（绝对坐标）
   - 鼠标按键按下/释放
   - 支持左键、右键、中键
   - 自动位置更新和延迟控制

2. **QGA 备用方案** - 当 SPICE 不可用时
   - 通过 xdotool 模拟鼠标操作（Linux）
   - 需要虚拟机内安装 xdotool
   - 支持基本的点击和移动

### 📋 技术架构

```
场景文件 (YAML)
    ↓
ScenarioRunner.execute_mouse_click()
    ↓
优先使用: SPICE 协议
    - SpiceProtocol.send_mouse_move(x, y)
    - SpiceProtocol.send_mouse_click(button, pressed)
    ↓
备用方案: QGA + xdotool
    - QgaProtocol.exec_shell("xdotool ...")
```

---

## 使用方法

### 1. 场景配置

在 YAML 场景文件中添加鼠标操作步骤：

```yaml
name: "鼠标点击测试"
description: "测试虚拟机鼠标操作功能"
target_domain: "test-vm"  # 指定目标虚拟机
tags:
  - mouse
  - test

steps:
  - name: "左键点击 (100, 100)"
    action:
      type: mouse_click
      x: 100
      y: 100
      button: "left"
    timeout: 5

  - name: "右键点击 (200, 200)"
    action:
      type: mouse_click
      x: 200
      y: 200
      button: "right"
    timeout: 5

  - name: "中键点击 (150, 150)"
    action:
      type: mouse_click
      x: 150
      y: 150
      button: "middle"
    timeout: 5
```

### 2. 鼠标按键选项

支持的按键值（不区分大小写）：

| 按键值   | 说明       | SPICE        | xdotool |
|---------|-----------|--------------|---------|
| `left`  | 左键       | MouseButton::Left | 1 |
| `right` | 右键       | MouseButton::Right | 3 |
| `middle` | 中键      | MouseButton::Middle | 2 |

### 3. 坐标系统

- **SPICE 模式**：使用绝对坐标（相对于虚拟机显示器左上角）
- **xdotool 模式**：使用 X11 坐标系统

坐标示例：
```
(0, 0)     →  X 轴
  ↓
  Y 轴

左上角: (0, 0)
屏幕中心 (1024x768): (512, 384)
```

---

## 协议初始化

### SPICE 协议要求

1. **虚拟机配置**：虚拟机需要配置 SPICE 图形设备
   ```xml
   <graphics type='spice' port='5900' autoport='yes'>
     <listen type='address' address='0.0.0.0'/>
   </graphics>
   ```

2. **自动连接**：ScenarioRunner 会在初始化时自动连接：
   - 通过 libvirt 发现 SPICE 配置
   - 建立到 SPICE 服务器的连接
   - 初始化 Inputs 通道

3. **连接日志**：
   ```
   [INFO] 初始化协议连接: 虚拟机 = test-vm
   [INFO] QMP 协议连接成功
   [INFO] QGA 协议连接成功
   [INFO] SPICE 协议连接成功  ← 关键
   ```

### QGA 备用方案

如果 SPICE 连接失败，系统会自动回退到 QGA + xdotool：

1. **虚拟机准备**：
   ```bash
   # 在虚拟机内安装 xdotool
   sudo apt-get install xdotool  # Debian/Ubuntu
   sudo yum install xdotool      # RHEL/CentOS
   ```

2. **备用日志**：
   ```
   [WARN] SPICE 协议连接失败: ...
   [WARN] SPICE 协议未初始化，尝试通过 QGA 执行鼠标脚本
   [INFO] 鼠标点击: (100, 100) [QGA/xdotool]
   ```

---

## 代码实现细节

### execute_mouse_click() 流程

```rust
async fn execute_mouse_click(&mut self, x: i32, y: i32, button: &str, index: usize)
    -> Result<StepReport>
{
    // 1. 优先使用 SPICE 协议
    if let Some(spice) = &mut self.spice_protocol {
        // 1.1 移动鼠标到目标位置
        spice.send_mouse_move(x as u32, y as u32, 0).await?;
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 1.2 按下鼠标按键
        spice.send_mouse_click(mouse_button, true).await?;
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 1.3 释放鼠标按键
        spice.send_mouse_click(mouse_button, false).await?;

        return Ok(StepReport::success(...));
    }

    // 2. 备用方案：QGA + xdotool
    if let Some(qga) = &self.qga_protocol {
        let script = format!("DISPLAY=:0 xdotool mousemove {} {} click {}", x, y, button_id);
        qga.exec_shell(&script).await?;
        return Ok(StepReport::success(...));
    }

    // 3. 无可用协议
    Err(ExecutorError::ProtocolError("..."))
}
```

### 时序说明

鼠标点击操作的标准时序：

```
时间轴: ---|----50ms----|----50ms----|
操作:   移动 → 等待 → 按下 → 等待 → 释放
```

这种延迟模拟了真实用户的操作，确保虚拟机能正确处理事件。

---

## 测试场景示例

### 示例 1: 基础鼠标测试

文件：`examples/scenarios/mouse-click-test.yaml`

```yaml
name: "鼠标点击测试"
description: "测试虚拟机鼠标操作功能"
target_domain: "test-vm"
tags:
  - mouse
  - basic

steps:
  - name: "左键点击"
    action:
      type: mouse_click
      x: 100
      y: 100
      button: "left"

  - name: "等待 1 秒"
    action:
      type: wait
      duration: 1

  - name: "右键点击"
    action:
      type: mouse_click
      x: 200
      y: 200
      button: "right"
```

### 示例 2: 综合测试（键盘 + 鼠标）

```yaml
name: "键盘鼠标综合测试"
target_domain: "test-vm"

steps:
  # 1. 打开应用（点击图标）
  - name: "点击应用图标"
    action:
      type: mouse_click
      x: 100
      y: 50
      button: "left"

  # 2. 等待应用启动
  - name: "等待启动"
    action:
      type: wait
      duration: 2

  # 3. 在应用中输入文本
  - name: "输入文本"
    action:
      type: send_text
      text: "Hello World"

  # 4. 点击保存按钮
  - name: "点击保存"
    action:
      type: mouse_click
      x: 300
      y: 500
      button: "left"
```

### 示例 3: UI 自动化测试

```yaml
name: "登录流程自动化"
target_domain: "desktop-vm"

steps:
  # 点击用户名输入框
  - action:
      type: mouse_click
      x: 500
      y: 300
      button: "left"

  # 输入用户名
  - action:
      type: send_text
      text: "admin"

  # 点击密码输入框
  - action:
      type: mouse_click
      x: 500
      y: 350
      button: "left"

  # 输入密码
  - action:
      type: send_text
      text: "password123"

  # 点击登录按钮
  - action:
      type: mouse_click
      x: 500
      y: 400
      button: "left"

  # 等待登录完成
  - action:
      type: wait
      duration: 3
```

---

## 运行测试

### 方法 1: 使用 CLI

```bash
# 运行鼠标点击测试场景
cargo run --bin atp -- scenario run examples/scenarios/mouse-click-test.yaml

# 查看测试报告
cargo run --bin atp -- report list
cargo run --bin atp -- report show <id>
```

### 方法 2: 使用 API

```rust
use atp_executor::{ScenarioRunner, Scenario};
use atp_transport::TransportManager;
use atp_protocol::ProtocolRegistry;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化
    let transport = Arc::new(TransportManager::new());
    let protocol_registry = Arc::new(ProtocolRegistry::new());

    // 创建执行器
    let mut runner = ScenarioRunner::new(transport, protocol_registry);

    // 加载场景
    let scenario = Scenario::from_yaml_file("mouse-test.yaml")?;

    // 执行
    let report = runner.run(&scenario).await?;

    println!("测试结果: {}/{} 步骤通过",
        report.passed_count,
        report.steps_executed
    );

    Ok(())
}
```

---

## 故障排查

### 问题 1: SPICE 连接失败

**症状**：
```
[WARN] SPICE 协议连接失败: Connection refused
```

**解决方案**：
1. 检查虚拟机是否配置了 SPICE：
   ```bash
   virsh dumpxml <vm-name> | grep spice
   ```

2. 确认 SPICE 端口开放：
   ```bash
   netstat -tlnp | grep 5900
   ```

3. 检查防火墙规则

### 问题 2: xdotool 执行失败

**症状**：
```
[ERROR] xdotool 执行失败（可能未安装）
```

**解决方案**：
在虚拟机内安装 xdotool：
```bash
sudo apt-get install xdotool
```

### 问题 3: 鼠标位置不准确

**原因**：
- 分辨率不匹配
- 坐标系统差异

**解决方案**：
1. 确认虚拟机分辨率
2. 调整坐标值
3. 使用相对坐标系统（如果适用）

### 问题 4: 所有协议都未初始化

**症状**：
```
[ERROR] SPICE 和 QGA 协议均未初始化，无法执行鼠标操作
```

**解决方案**：
1. 确保场景文件中指定了 `target_domain`
2. 检查虚拟机是否正在运行
3. 验证 libvirt 连接正常

---

## 性能考虑

### 延迟优化

当前实现的延迟设置：
- 移动后等待：50ms
- 按下后等待：50ms
- 总点击时间：~100ms

如需优化性能，可以调整 `tokio::time::sleep` 的值，但需注意：
- 太短可能导致事件丢失
- 太长会降低测试速度

### 批量操作

对于需要大量鼠标操作的场景，建议：
1. 合并相邻的操作
2. 减少不必要的等待
3. 使用验证点而不是盲等

---

## 下一步增强

### 计划中的功能

- [ ] 鼠标拖拽操作（drag and drop）
- [ ] 鼠标滚轮支持
- [ ] 鼠标双击快捷方法
- [ ] 相对坐标移动
- [ ] 鼠标轨迹录制和回放

### 贡献

欢迎提交 PR 来增强鼠标操作功能！

---

## 参考资料

- [SPICE 协议实现](../atp-core/protocol/src/spice/)
- [执行器实现](../atp-core/executor/src/runner.rs)
- [场景示例](../examples/scenarios/)
- [SPICE 官方文档](https://www.spice-space.org/)

---

**维护者**: OCloudView ATP Team
**反馈**: 请在 GitHub 上提交 Issue

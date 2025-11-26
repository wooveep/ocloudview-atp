# 阶段 5: CLI 应用实现总结

## 实现概述

完成了 OCloudView ATP 项目的命令行工具（CLI）实现，提供了完整的虚拟机自动化测试控制接口。

## 实现内容

### 1. 核心架构

#### 1.1 配置管理系统 ([cli/src/config.rs](../atp-application/cli/src/config.rs))

实现了基于 TOML 的配置文件管理：

- **配置文件路径**: `~/.config/atp/config.toml`
- **主机配置管理**: 支持添加、删除、查询主机
- **默认主机**: 自动设置第一个主机为默认主机
- **场景目录**: 可配置的测试场景目录

```rust
pub struct CliConfig {
    pub hosts: HashMap<String, HostConfig>,
    pub default_host: Option<String>,
    pub scenario_dir: Option<String>,
    pub version: String,
}
```

#### 1.2 命令结构 ([cli/src/main.rs](../atp-application/cli/src/main.rs))

基于 `clap` 实现的命令行框架：

- **主机管理**: `atp host {add, list, remove}`
- **键盘操作**: `atp keyboard {send, text}`
- **鼠标操作**: `atp mouse {click, move}`
- **命令执行**: `atp command exec`
- **场景管理**: `atp scenario {run, list}`

### 2. 主要功能

#### 2.1 主机管理 ([cli/src/commands/host.rs](../atp-application/cli/src/commands/host.rs) - 91 行)

**功能**:
- `atp host add <ID> <HOST> [--uri URI]` - 添加主机配置
- `atp host list` - 列出所有配置的主机
- `atp host remove <ID>` - 移除主机配置

**特点**:
- 彩色输出，易于阅读
- 显示默认主机标记
- 支持自定义 Libvirt URI

**示例**:
```bash
# 添加主机
atp host add kvm1 192.168.1.100

# 列出主机
atp host list

# 移除主机
atp host remove kvm1
```

#### 2.2 场景执行 ([cli/src/commands/scenario.rs](../atp-application/cli/src/commands/scenario.rs) - 247 行)

**功能**:
- `atp scenario run <FILE>` - 执行测试场景
- `atp scenario list` - 列出所有场景

**特点**:
- 支持 YAML 和 JSON 格式场景文件
- 实时进度显示（进度条 + spinner）
- 详细的执行报告（步骤状态、耗时、输出、错误）
- 自动集成传输管理器和协议注册表
- 彩色输出区分成功/失败/跳过状态

**场景执行流程**:
```
1. 加载场景文件 (YAML/JSON)
2. 初始化传输管理器（连接配置的主机）
3. 创建场景执行器
4. 执行场景步骤（带进度条）
5. 生成详细执行报告
```

**执行报告格式**:
```
============================================================
执行报告
============================================================

场景名称: 基础键盘测试
执行时间: 1234 ms

步骤统计:
  总步骤: 5
  成功:   4
  失败:   1

步骤详情:

✓ 步骤 1: 发送按键: Enter
   耗时: 123 ms

✗ 步骤 2: 发送文本: Hello
   错误: 连接超时
   耗时: 2000 ms

============================================================
✓ 场景执行成功
============================================================
```

#### 2.3 键盘操作 ([cli/src/commands/keyboard.rs](../atp-application/cli/src/commands/keyboard.rs) - 49 行)

**功能**:
- `atp keyboard send --host <HOST> --vm <VM> --key <KEY>` - 发送单个按键
- `atp keyboard text --host <HOST> --vm <VM> <TEXT>` - 发送文本

**当前状态**: 命令框架已完成，建议通过场景文件使用

#### 2.4 鼠标操作 ([cli/src/commands/mouse.rs](../atp-application/cli/src/commands/mouse.rs) - 53 行)

**功能**:
- `atp mouse click --host <HOST> --vm <VM> --x <X> --y <Y> [--button <BUTTON>]`
- `atp mouse move --host <HOST> --vm <VM> --x <X> --y <Y>`

**当前状态**: 命令框架已完成，建议通过场景文件使用

#### 2.5 命令执行 ([cli/src/commands/command.rs](../atp-application/cli/src/commands/command.rs) - 33 行)

**功能**:
- `atp command exec --host <HOST> --vm <VM> <CMD>` - 执行 Guest 命令

**当前状态**: 命令框架已完成，建议通过场景文件使用

### 3. 用户体验优化

#### 3.1 彩色输出

使用 `colored` crate 实现:
- 绿色: 成功状态、确认信息
- 黄色: 警告、数值
- 红色: 错误信息
- 青色: 操作指示、标题
- 灰色: 次要信息

#### 3.2 进度指示

使用 `indicatif` crate 实现:
- Spinner: 加载场景、初始化连接
- 进度条: 场景执行进度
- 实时消息更新

#### 3.3 用户友好提示

- 空状态提示（如无主机时显示添加命令示例）
- 功能说明（引导用户使用场景文件）
- 错误信息清晰明确

## 技术实现

### 依赖库

```toml
# CLI 框架
clap = { version = "4.4", features = ["derive"] }

# 美化输出
colored = "2.1"
indicatif = "0.17"

# 配置文件
toml = "0.8"
dirs = "5.0"

# 核心库集成
atp-transport = { path = "../atp-core/transport" }
atp-protocol = { path = "../atp-core/protocol" }
atp-executor = { path = "../atp-core/executor" }
```

### 与核心库集成

CLI 充分利用了已实现的核心组件:

1. **传输层** (`atp-transport`): 管理 libvirt 连接
2. **协议层** (`atp-protocol`): QMP、QGA、SPICE 协议支持
3. **执行器** (`atp-executor`): 场景加载和执行

### 错误处理

使用 `anyhow` 提供上下文丰富的错误信息:

```rust
config.get_host(host_id)
    .with_context(|| format!("主机 {} 不存在", host_id))?
```

## 使用示例

### 1. 配置主机

```bash
# 添加 KVM 主机
atp host add kvm1 192.168.1.100

# 添加带自定义 URI 的主机
atp host add kvm2 192.168.1.101 --uri "qemu+ssh://root@192.168.1.101/system"

# 列出所有主机
atp host list
```

### 2. 执行测试场景

```bash
# 运行 YAML 场景
atp scenario run examples/scenarios/keyboard_test.yaml

# 运行 JSON 场景
atp scenario run examples/scenarios/mouse_test.json

# 列出所有场景
atp scenario list
```

### 3. 场景文件示例

**keyboard_test.yaml**:
```yaml
name: "基础键盘测试"
description: "测试键盘输入功能"
tags:
  - keyboard
  - input
steps:
  - name: "发送 Enter 键"
    action:
      type: send_key
      key: "Enter"
    timeout: 5

  - name: "发送文本"
    action:
      type: send_text
      text: "Hello, World!"
    timeout: 10

  - name: "等待 2 秒"
    action:
      type: wait
      duration: 2
```

## 代码统计

| 模块 | 文件 | 行数 | 描述 |
|------|------|------|------|
| main.rs | 1 | 192 | 主程序和命令定义 |
| config.rs | 1 | 224 | 配置管理 |
| host.rs | 1 | 91 | 主机管理命令 |
| scenario.rs | 1 | 247 | 场景执行命令 |
| keyboard.rs | 1 | 49 | 键盘操作命令 |
| mouse.rs | 1 | 53 | 鼠标操作命令 |
| command.rs | 1 | 33 | 命令执行 |
| **总计** | **7** | **~889 行** | |

## 已实现功能

✅ CLI 框架搭建 (clap)
✅ 配置文件管理 (TOML)
✅ 主机管理命令 (add, list, remove)
✅ 场景执行命令 (run, list)
✅ 键盘/鼠标/命令框架
✅ 彩色输出
✅ 进度条和 Spinner
✅ 详细执行报告
✅ 错误处理和用户提示

## 待实现功能

📝 并发执行支持 (`--concurrent`)
📝 循环执行支持 (`--loop`)
📝 交互式模式
📝 配置初始化命令 (`atp config init`)
📝 场景验证命令 (`atp scenario validate`)
📝 完整的键盘/鼠标/命令直接执行（当前建议通过场景）

## 已知问题

1. **Libvirt 依赖**: 编译需要系统安装 libvirt-dev
   - 解决方案: 在目标系统安装 libvirt 开发库
   - Ubuntu/Debian: `sudo apt-get install libvirt-dev`
   - CentOS/RHEL: `sudo yum install libvirt-devel`

2. **键盘/鼠标/命令直接执行**: 当前为框架实现
   - 建议: 通过场景文件使用这些功能
   - 原因: 场景执行器提供更完整的上下文管理

## 下一步计划

1. **功能增强**:
   - 添加并发执行支持
   - 添加循环执行支持
   - 实现交互式模式

2. **用户体验**:
   - 添加命令自动完成
   - 添加配置验证
   - 改进错误消息

3. **集成测试**:
   - 端到端 CLI 测试
   - 场景执行测试
   - 配置管理测试

## 技术挑战与解决方案

### 1. 配置文件管理

**挑战**: 跨平台配置文件路径
**解决方案**: 使用 `dirs` crate 获取用户配置目录

### 2. 进度显示

**挑战**: 异步场景执行中的进度更新
**解决方案**: 使用 `indicatif` 的异步 API，手动更新进度

### 3. 彩色输出

**挑战**: 终端兼容性
**解决方案**: `colored` crate 自动检测终端能力

## 总结

阶段 5 成功实现了完整的 CLI 应用框架，为用户提供了友好的命令行接口。CLI 充分集成了传输层、协议层和执行器的功能，特别是场景执行命令提供了完整的测试流程支持。

虽然部分直接操作命令（键盘、鼠标、命令）建议通过场景文件使用，但这种设计使得测试更加可重复、可维护。彩色输出和进度指示大大提升了用户体验。

## 文件结构

```
atp-application/cli/
├── Cargo.toml
└── src/
    ├── main.rs          # 主程序和命令定义
    ├── config.rs        # 配置管理
    └── commands/
        ├── mod.rs       # 命令模块
        ├── host.rs      # 主机管理
        ├── scenario.rs  # 场景执行
        ├── keyboard.rs  # 键盘操作
        ├── mouse.rs     # 鼠标操作
        └── command.rs   # 命令执行
```

## 参考资料

- [Clap Documentation](https://docs.rs/clap/)
- [Indicatif Documentation](https://docs.rs/indicatif/)
- [Colored Documentation](https://docs.rs/colored/)
- [TOML Format](https://toml.io/)

# VDI 测试场景示例

本目录包含 OCloudView ATP VDI 平台测试场景示例。

## 目录结构

```
vdi-scenarios/
├── basic/              # 基础场景
│   ├── create_desk_pool.yaml
│   ├── start_domain.yaml
│   └── shutdown_domain.yaml
├── integration/        # 集成场景
│   ├── desk_pool_keyboard_test.yaml
│   └── user_workflow.yaml
└── stress/            # 压力测试场景
    ├── concurrent_start_50vms.yaml
    └── loop_create_delete.yaml
```

## 基础场景 (basic/)

### create_desk_pool.yaml
创建桌面池的基础场景。

**用途：**
- 测试 VDI 平台桌面池创建功能
- 验证桌面池创建流程

**执行方式：**
```bash
atp scenario run examples/vdi-scenarios/basic/create_desk_pool.yaml
```

### start_domain.yaml
启动虚拟机的基础场景。

**用途：**
- 测试虚拟机启动功能
- 验证虚拟机状态转换

### shutdown_domain.yaml
关闭虚拟机的基础场景。

**用途：**
- 测试虚拟机关闭功能
- 验证虚拟机优雅关闭流程

## 集成场景 (integration/)

### desk_pool_keyboard_test.yaml
桌面池创建与键盘输入测试的端到端场景。

**用途：**
- 完整的端到端测试
- 验证 VDI 平台与虚拟化层的集成
- 测试键盘输入功能

**测试流程：**
1. 创建桌面池
2. 启用桌面池
3. 启动虚拟机
4. 建立虚拟化层连接
5. 发送键盘输入
6. 清理资源

### user_workflow.yaml
用户完整工作流场景。

**用途：**
- 模拟真实用户使用场景
- 测试用户登录、使用虚拟桌面的完整流程
- 验证用户操作的各个环节

**测试流程：**
1. 用户登录
2. 获取用户虚拟机
3. 启动虚拟机
4. 连接虚拟机
5. 执行用户操作（打开应用、编辑文档）
6. 用户注销

## 压力测试场景 (stress/)

### concurrent_start_50vms.yaml
并发启动 50 个虚拟机的压力测试。

**用途：**
- 测试系统在高负载下的表现
- 验证并发处理能力
- 测试大规模虚拟机管理

**特点：**
- 创建 50 个虚拟机的桌面池
- 并发启动所有虚拟机
- 验证所有虚拟机状态

### loop_create_delete.yaml
循环创建删除桌面池的稳定性测试。

**用途：**
- 测试系统稳定性
- 验证资源清理机制
- 检测内存泄漏等问题

**特点：**
- 循环 10 次创建删除操作
- 测试资源管理的正确性

## 场景格式说明

### 基本结构
```yaml
name: "场景名称"
description: "场景描述"
tags:
  - tag1
  - tag2
timeout: 600  # 超时时间（秒）

steps:
  - type: step_type
    # 步骤参数
```

### 步骤类型

#### 1. VDI 平台操作
```yaml
- type: vdi_action
  action: create_desk_pool
  name: "桌面池名称"
  template_id: "template-001"
  count: 2
  capture_output: "desk_pool"
```

#### 2. 虚拟化层操作
```yaml
- type: virtualization_action
  action: send_keyboard
  text: "Hello World"
  verify: true
```

#### 3. 等待
```yaml
- type: wait
  duration: 10s
```

#### 4. 验证
```yaml
- type: verify
  condition: domain_status
  domain_id: "vm-001"
  expected_status: "running"
  timeout: 30s
```

## 使用示例

### 执行单个场景
```bash
atp scenario run examples/vdi-scenarios/basic/create_desk_pool.yaml
```

### 执行多个场景
```bash
atp scenario run examples/vdi-scenarios/basic/*.yaml
```

### 执行指定标签的场景
```bash
atp scenario run --tags integration,keyboard
```

### 生成测试报告
```bash
atp scenario run examples/vdi-scenarios/integration/desk_pool_keyboard_test.yaml \
  --report-format json \
  --report-output report.json
```

## 最佳实践

1. **场景命名**：使用清晰的描述性名称
2. **添加标签**：为场景添加适当的标签以便分类和筛选
3. **设置超时**：为可能耗时较长的操作设置合理的超时时间
4. **资源清理**：确保场景执行后清理创建的资源
5. **步骤描述**：为关键步骤添加注释说明

## 扩展场景

可以基于这些示例创建自定义场景：

1. 复制现有场景文件
2. 修改场景名称和描述
3. 调整测试步骤
4. 添加适当的标签
5. 测试并验证场景

## 贡献

欢迎贡献更多的测试场景示例！

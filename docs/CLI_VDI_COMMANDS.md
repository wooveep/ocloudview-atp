# ATP CLI - VDI 平台管理命令使用指南

## 概述

ATP CLI 提供了一套完整的 VDI 平台管理命令，包括虚拟机状态验证、批量操作、磁盘存储查询等功能。

## 命令概览

```bash
atp vdi <COMMAND>
```

### 可用命令

| 命令 | 说明 |
|------|------|
| `verify` | 验证 VDI 平台与 libvirt 虚拟机状态一致性 |
| `list-hosts` | 列出 VDI 平台的所有主机 |
| `list-vms` | 列出 VDI 平台的所有虚拟机 |
| `sync-hosts` | 同步 VDI 主机到本地数据库 |
| `disk-location` | 查询虚拟机磁盘存储位置（支持 Gluster） |
| `start` | 批量启动虚拟机 |
| `assign` | 批量分配虚拟机给用户 |
| `rename` | 批量重命名虚拟机为绑定用户名 |
| `set-auto-join-domain` | 批量设置虚拟机自动加域 |

## 快速开始

### 1. 配置文件

创建或编辑 `config/atp.toml` 配置文件：

```toml
[vdi]
base_url = "http://192.168.41.51:8088"
username = "admin"
password = "your_password"
verify_ssl = false
connect_timeout = 10
```

### 2. 验证 VDI 与 libvirt 一致性

```bash
# 基本使用
atp vdi verify

# 只显示不一致的虚拟机
atp vdi verify --only-diff

# 使用 JSON 格式输出
atp vdi verify --format json

# 使用自定义配置文件
atp vdi verify --config /path/to/config.toml
```

**输出示例**:

```
╔════════════════════════════════════════════════════════════════╗
║         VDI 与 libvirt 虚拟机状态一致性验证                   ║
╚════════════════════════════════════════════════════════════════╝

📋 步骤 1/4: 登录 VDI 平台...
   ✅ VDI 登录成功

📋 步骤 2/4: 获取 VDI 主机列表...
   ✅ 找到 1 个主机

📋 步骤 3/4: 获取 VDI 虚拟机列表...
   ✅ VDI 虚拟机数量: 4

📋 步骤 4/4: 连接 libvirt 并比对虚拟机状态...

   🔗 连接主机: ocloud (192.168.41.51)
   ✅ 连接成功: qemu+tcp://192.168.41.51/system
   📊 libvirt 虚拟机数量: 4

╔════════════════════════════════════════════════════════════════╗
║                      验证结果汇总                              ║
╚════════════════════════════════════════════════════════════════╝

📊 统计信息:
   总虚拟机数: 4
   一致: 4 ✅
   不一致: 0 ❌
   一致性: 100.0%

📋 详细对比结果:

虚拟机名称                主机              VDI状态                libvirt状态       一致性
--------------------------------------------------------------------------------
ocloud02             ocloud          运行中                  1                    ✅
win10_22h2001        ocloud          运行中                  1                    ✅
ocloud01             ocloud          运行中                  1                    ✅
lic                  ocloud          运行中                  1                    ✅
```

### 3. 列出主机

```bash
atp vdi list-hosts
```

**输出示例**:

```
📋 VDI 平台主机列表

主机名                  IP地址                 状态         CPU(核)          内存(GB)
--------------------------------------------------------------------------------
ocloud               192.168.41.51        在线 ✅       32              125.47

总计: 1 个主机
```

### 4. 列出虚拟机

```bash
# 列出所有虚拟机
atp vdi list-vms

# 只列出特定主机上的虚拟机
atp vdi list-vms --host ocloud
```

**输出示例**:

```
📋 VDI 平台虚拟机列表

虚拟机名称                     主机                   状态              CPU(核)     内存(GB)
------------------------------------------------------------------------------------------
lic                       ocloud               运行中 ✅           4          8.00
ocloud01                  ocloud               运行中 ✅           16         24.00
ocloud02                  ocloud               运行中 ✅           16         24.00
win10_22h2001             ocloud               运行中 ✅           16         32.00

总计: 4 个虚拟机
```

### 5. 同步主机

```bash
# 同步 VDI 主机到数据库
atp vdi sync-hosts

# 同时测试 libvirt 连接
atp vdi sync-hosts --test-connection
```

**输出示例**:

```
🔄 同步 VDI 主机到数据库

📊 发现 2 个主机:

  1. ocloud (192.168.41.51) - 在线 ✅ [已保存]
  2. node2 (192.168.41.52) - 在线 ✅ [已保存]

✅ 已保存 2 个主机到数据库
💡 提示: 使用 `atp host update-ssh <id>` 更新主机 SSH 配置
```

### 6. 查询磁盘存储位置

```bash
# 基本查询
atp vdi disk-location --vm myvm

# 启用 SSH 查询 Gluster 实际 brick 位置
atp vdi disk-location --vm myvm --ssh

# 指定 SSH 认证
atp vdi disk-location --vm myvm --ssh --ssh-user root --ssh-key ~/.ssh/id_rsa

# JSON 格式输出
atp vdi disk-location --vm myvm --ssh --format json
```

**输出示例**:

```
╔════════════════════════════════════════════════════════════════╗
║              虚拟机磁盘存储位置查询                            ║
╚════════════════════════════════════════════════════════════════╝

📋 步骤 1/3: 登录 VDI 平台...
   ✅ VDI 登录成功

📋 步骤 2/3: 查找虚拟机 myvm...
   ✅ 找到虚拟机: myvm (abc123)

📋 步骤 3/3: 获取磁盘信息...
   ✅ 找到 2 个磁盘

╔════════════════════════════════════════════════════════════════╗
║                      磁盘存储位置详情                          ║
╚════════════════════════════════════════════════════════════════╝

虚拟机: myvm

📀 磁盘 1 - disk1 [启动盘]

   文件名:     myvm.qcow2
   逻辑路径:   /gluster/vol1/myvm.qcow2
   存储池:     pool1 (gluster)
   存储类型:   Gluster 分布式存储
   大小:       100 GB
   总线类型:   virtio

   🔍 Gluster 实际存储位置:
      卷名:    vol1
      副本数:  2
      副本 1: 192.168.41.51:/data/brick1/myvm.qcow2
      副本 2: 192.168.41.52:/data/brick1/myvm.qcow2
```

### 7. 批量启动虚拟机

```bash
# 预览将要启动的虚拟机
atp vdi start --pattern "test*" --dry-run

# 启动所有匹配的关机虚拟机
atp vdi start --pattern "test*"

# 启动并通过 QGA 验证
atp vdi start --pattern "test*" --verify

# 启动所有虚拟机
atp vdi start --pattern "*"
```

**模式匹配规则**:

| 模式 | 说明 | 示例 |
|------|------|------|
| `*` | 匹配全部 | 所有虚拟机 |
| `prefix*` | 前缀匹配 | `test*` 匹配 test01, test02 |
| `*suffix` | 后缀匹配 | `*-prod` 匹配 vm1-prod |
| `*middle*` | 包含匹配 | `*dev*` 匹配 mydev01 |
| `exact` | 精确匹配 | `vm01` 只匹配 vm01 |

**输出示例**:

```
╔════════════════════════════════════════════════════════════════╗
║                    批量启动虚拟机                              ║
╚════════════════════════════════════════════════════════════════╝

✅ VDI 登录成功

🔍 匹配模式: test*

📋 找到 3 个关机虚拟机:

虚拟机名称                    主机                 状态
----------------------------------------------------------------------
test01                       ocloud               关机
test02                       ocloud               关机
test03                       node2                关机

🚀 正在启动虚拟机...

✅ 批量启动命令已发送
```

**使用 `--verify` 验证启动**:

启用 QGA 验证后，CLI 会等待虚拟机启动并通过 QEMU Guest Agent 确认：

```
╔════════════════════════════════════════════════════════════════╗
║                    QGA 启动验证                                ║
╚════════════════════════════════════════════════════════════════╝

⏳ 等待虚拟机启动 (30秒)...
🔍 开始并行验证 3 个虚拟机...

╔════════════════════════════════════════════════════════════════╗
║                    验证结果报告                                ║
╚════════════════════════════════════════════════════════════════╝

📊 验证统计:
   总数: 3
   成功: 2 ✅
   失败: 1 ❌

❌ 未成功启动的虚拟机列表:
虚拟机名称                    主机                 错误原因
--------------------------------------------------------------------------------
test03                       node2                QGA 验证失败 (已重试 3 次)
```

### 8. 批量分配虚拟机

```bash
# 预览分配计划（按用户列表）
atp vdi assign --pattern "vm*" --users "user1,user2,user3" --dry-run

# 预览分配计划（按组织单位）
atp vdi assign --pattern "vm*" --group "IT部门" --dry-run

# 执行分配
atp vdi assign --pattern "vm*" --group "IT部门"

# 强制重新分配（覆盖已绑定用户）
atp vdi assign --pattern "vm*" --users "newuser1,newuser2" --force
```

**输出示例**:

```
╔════════════════════════════════════════════════════════════════╗
║                    批量分配虚拟机                              ║
╚════════════════════════════════════════════════════════════════╝

✅ VDI 登录成功

🔍 匹配模式: vm*

📋 组织单位: IT部门

👥 找到 5 个目标用户
💻 找到 5 个未分配虚拟机

虚拟机                        分配给用户            状态
-----------------------------------------------------------------
vm01                         zhangsan              新分配
vm02                         lisi                  新分配
vm03                         wangwu                新分配
vm04                         zhaoliu               新分配
vm05                         sunqi                 新分配

📝 预览模式 - 不执行实际操作
```

### 9. 批量重命名虚拟机

将虚拟机重命名为其绑定用户的用户名：

```bash
# 预览重命名计划
atp vdi rename --pattern "vm*" --dry-run

# 执行重命名
atp vdi rename --pattern "vm*"

# JSON 格式输出
atp vdi rename --pattern "*" --dry-run --format json
```

**输出示例**:

```
╔════════════════════════════════════════════════════════════════╗
║                    批量重命名虚拟机                            ║
╚════════════════════════════════════════════════════════════════╝

✅ VDI 登录成功

🔍 匹配模式: vm*

📋 找到 3 个需要重命名的虚拟机:

当前名称                       新名称
-----------------------------------------------------------------
vm01                          zhangsan
vm02                          lisi
vm03                          wangwu

📝 预览模式 - 不执行实际操作
```

### 10. 批量设置自动加域

```bash
# 预览启用自动加域
atp vdi set-auto-join-domain --pattern "win*" --enable --dry-run

# 执行启用
atp vdi set-auto-join-domain --pattern "win*" --enable

# 禁用自动加域
atp vdi set-auto-join-domain --pattern "test*" --disable
```

**输出示例**:

```
╔════════════════════════════════════════════════════════════════╗
║                 批量设置自动加域 (autoJoinDomain)              ║
╚════════════════════════════════════════════════════════════════╝

✅ VDI 登录成功

🔍 匹配模式: win*
🎯 操作: 启用 自动加域

📋 找到 4 个匹配的虚拟机:

虚拟机名称                    主机                 操作
----------------------------------------------------------------------
win10_01                     ocloud               启用
win10_02                     ocloud               启用
win11_01                     node2                启用
win11_02                     node2                启用

⚙️  正在设置 autoJoinDomain...

📊 设置结果: 成功 4, 失败 0
```

## 命令详解

### verify - 虚拟机状态一致性验证

这是最核心的命令，用于自动化检测 VDI 平台与 libvirt 底层虚拟化的数据一致性。

**工作流程**:

1. **登录 VDI 平台**
   - 使用 MD5 加密密码
   - 获取 Token 认证

2. **获取 VDI 主机列表**
   - 列出所有在线主机
   - 建立主机 ID 到名称的映射

3. **获取 VDI 虚拟机列表**
   - 列出所有虚拟机及其状态
   - 记录每个虚拟机所在的主机

4. **连接 libvirt**
   - 自动尝试 TCP 和 SSH 连接
   - 获取每个主机上的虚拟机列表

5. **状态比对**
   - 比对虚拟机名称
   - 比对运行状态（运行中/关机）
   - 生成一致性报告

**选项**:

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `-c, --config` | 配置文件路径 | `config/atp.toml` |
| `-o, --only-diff` | 只显示不一致的虚拟机 | `false` |
| `-f, --format` | 输出格式 (table/json/yaml) | `table` |

**退出码**:

- `0`: 所有虚拟机状态一致
- `1`: 存在不一致的虚拟机

**使用场景**:

1. **日常监控**: 定期检查数据一致性
2. **故障排查**: 发现 VDI 和 libvirt 数据不同步
3. **CI/CD 集成**: 自动化测试流程中验证环境状态
4. **维护前检查**: 在执行维护操作前确认当前状态

**JSON 输出示例**:

```bash
atp vdi verify --format json
```

```json
[
  {
    "vm_name": "lic",
    "host": "ocloud",
    "vdi_status": "运行中",
    "libvirt_status": "1",
    "consistent": true
  },
  {
    "vm_name": "ocloud01",
    "host": "ocloud",
    "vdi_status": "运行中",
    "libvirt_status": "1",
    "consistent": true
  }
]
```

### list-hosts - 列出主机

快速查看 VDI 平台上的所有主机及其状态。

**显示信息**:

- 主机名
- IP 地址
- 在线/离线状态
- CPU 核心数
- 内存大小（GB）

**使用场景**:

- 快速了解基础设施状态
- 确认主机在线情况
- 获取主机配置信息

### list-vms - 列出虚拟机

查看 VDI 平台上的所有虚拟机及其详细信息。

**选项**:

| 选项 | 说明 |
|------|------|
| `-H, --host` | 只显示指定主机上的虚拟机 |

**显示信息**:

- 虚拟机名称
- 所在主机
- 运行状态（运行中/关机/未知）
- CPU 核心数
- 内存大小（GB）

**使用场景**:

- 查看虚拟机分布
- 确认虚拟机状态
- 资源规划

### sync-hosts - 同步主机

从 VDI 平台同步主机信息到本地数据库。

**选项**:

| 选项 | 说明 |
|------|------|
| `-t, --test-connection` | 同时测试 libvirt 连接 |

**使用场景**:

- 发现新添加的主机
- 验证主机连通性
- 更新本地配置

### disk-location - 查询磁盘存储位置

查询虚拟机磁盘的存储位置，支持 Gluster 分布式存储的 brick 定位。

**选项**:

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `--vm` | 虚拟机 ID 或名称 | 必需 |
| `--ssh` | 启用 SSH 查询 Gluster brick 位置 | `false` |
| `--ssh-user` | SSH 用户名 | `root` |
| `--ssh-password` | SSH 密码 | - |
| `--ssh-key` | SSH 私钥路径 | - |
| `-f, --format` | 输出格式 (table/json) | `table` |

**使用场景**:

- 排查存储问题
- 确认数据副本位置
- 存储迁移规划

### start - 批量启动虚拟机

批量启动匹配模式的关机虚拟机。

**选项**:

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `-p, --pattern` | 虚拟机名称匹配模式 | 必需 |
| `--dry-run` | 预览模式，不执行操作 | `false` |
| `--verify` | 启动后通过 QGA 验证 | `false` |
| `-f, --format` | 输出格式 (table/json) | `table` |

**使用场景**:

- 批量部署虚拟机
- 灾难恢复
- 每日自动启动

### assign - 批量分配虚拟机

批量将虚拟机分配给用户。

**选项**:

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `-p, --pattern` | 虚拟机名称匹配模式 | 必需 |
| `--users` | 用户名列表（逗号分隔） | - |
| `--group` | 组织单位名称 | - |
| `--dry-run` | 预览模式 | `false` |
| `--force` | 强制重新分配 | `false` |
| `-f, --format` | 输出格式 (table/json) | `table` |

**使用场景**:

- 批量部署新员工桌面
- 部门虚拟机分配
- 虚拟机重新分配

### rename - 批量重命名虚拟机

将虚拟机重命名为其绑定用户的用户名。

**选项**:

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `-p, --pattern` | 虚拟机名称匹配模式 | 必需 |
| `--dry-run` | 预览模式 | `false` |
| `-f, --format` | 输出格式 (table/json) | `table` |

**使用场景**:

- 标准化虚拟机命名
- 简化管理

### set-auto-join-domain - 批量设置自动加域

批量设置虚拟机的 autoJoinDomain 属性。

**选项**:

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `-p, --pattern` | 虚拟机名称匹配模式 | 必需 |
| `--enable` | 启用自动加域 | - |
| `--disable` | 禁用自动加域 | - |
| `--dry-run` | 预览模式 | `false` |
| `-f, --format` | 输出格式 (table/json) | `table` |

**使用场景**:

- Windows 域环境部署
- 批量配置虚拟机

## 高级用法

### 1. 定时监控脚本

```bash
#!/bin/bash
# vdi-monitor.sh - VDI 状态监控脚本

LOG_FILE="/var/log/atp-vdi-monitor.log"

echo "$(date): 开始 VDI 状态检查" >> $LOG_FILE

if /usr/local/bin/atp vdi verify --only-diff >> $LOG_FILE 2>&1; then
    echo "$(date): ✅ 所有虚拟机状态一致" >> $LOG_FILE
else
    echo "$(date): ❌ 发现虚拟机状态不一致" >> $LOG_FILE
    # 发送告警通知
    /usr/local/bin/send-alert.sh "VDI状态不一致告警"
fi
```

**添加到 crontab**:

```bash
# 每小时检查一次
0 * * * * /home/admin/vdi-monitor.sh
```

### 2. CI/CD 集成

```yaml
# .gitlab-ci.yml
vdi-consistency-check:
  stage: test
  script:
    - atp vdi verify --format json > vdi-status.json
    - if [ $? -ne 0 ]; then exit 1; fi
  artifacts:
    paths:
      - vdi-status.json
    when: always
```

### 3. 批量检查多个环境

```bash
#!/bin/bash
# check-all-envs.sh

ENVS=("dev" "staging" "prod")

for env in "${ENVS[@]}"; do
    echo "检查 $env 环境..."
    atp vdi verify --config "config-${env}.toml" --only-diff
    echo "---"
done
```

### 4. 导出报告

```bash
# 生成 JSON 报告
atp vdi verify --format json > vdi-report-$(date +%Y%m%d).json

# 生成 YAML 报告
atp vdi verify --format yaml > vdi-report-$(date +%Y%m%d).yaml
```

### 5. 自动化部署流程

```bash
#!/bin/bash
# deploy-vms.sh - 批量部署虚拟机

PATTERN="newvm*"
GROUP="新员工"

# 1. 预览分配计划
echo "=== 分配计划预览 ==="
atp vdi assign --pattern "$PATTERN" --group "$GROUP" --dry-run

read -p "确认执行分配? (y/n): " confirm
if [ "$confirm" != "y" ]; then
    echo "已取消"
    exit 0
fi

# 2. 执行分配
echo "=== 执行分配 ==="
atp vdi assign --pattern "$PATTERN" --group "$GROUP"

# 3. 重命名虚拟机
echo "=== 重命名虚拟机 ==="
atp vdi rename --pattern "$PATTERN"

# 4. 启用自动加域
echo "=== 启用自动加域 ==="
atp vdi set-auto-join-domain --pattern "$PATTERN" --enable

# 5. 启动虚拟机并验证
echo "=== 启动虚拟机 ==="
atp vdi start --pattern "$PATTERN" --verify

echo "=== 部署完成 ==="
```

## 故障排查

### 问题 1: 连接失败

**症状**: 无法连接到 VDI 平台或 libvirt

**解决方法**:

1. 检查配置文件中的 VDI 地址和凭据
2. 确认网络连通性: `ping 192.168.41.51`
3. 检查 libvirt 服务状态: `systemctl status libvirtd`
4. 尝试手动连接: `virsh -c qemu+tcp://192.168.41.51/system list`

### 问题 2: 认证失败

**症状**: 提示 "VDI 登录失败"

**解决方法**:

1. 确认用户名和密码正确
2. 检查用户是否有权限访问 VDI API
3. 尝试在浏览器中登录 VDI 平台

### 问题 3: 虚拟机状态不一致

**症状**: `verify` 命令报告状态不一致

**可能原因**:

1. VDI 数据库和 libvirt 实际状态不同步
2. 虚拟机正在进行状态转换（启动/关机中）
3. libvirt 域名与 VDI 中的名称不匹配

**排查步骤**:

```bash
# 1. 查看详细状态
atp vdi list-vms

# 2. 手动检查 libvirt
virsh -c qemu+tcp://192.168.41.51/system list --all

# 3. 重新同步VDI数据（通过VDI管理后台）
```

### 问题 4: QGA 验证失败

**症状**: `start --verify` 报告验证失败

**可能原因**:

1. QEMU Guest Agent 未安装或未运行
2. 虚拟机启动时间过长
3. libvirt 连接问题

**排查步骤**:

```bash
# 1. 检查虚拟机状态
virsh -c qemu+tcp://host/system dominfo vmname

# 2. 检查 QGA 通道
virsh -c qemu+tcp://host/system qemu-agent-command vmname '{"execute":"guest-ping"}'
```

## 最佳实践

### 1. 定期检查

建议每小时自动检查一次虚拟机状态一致性，及时发现问题。

### 2. 告警机制

当发现状态不一致时，及时发送告警通知运维人员。

### 3. 日志归档

保留历史检查日志，便于追踪问题和分析趋势。

### 4. 权限控制

使用专门的服务账号进行监控，限制权限范围。

### 5. 多环境隔离

为不同环境准备独立的配置文件，避免误操作。

### 6. 使用预览模式

执行批量操作前，始终使用 `--dry-run` 预览变更。

### 7. 备份配置

定期备份本地数据库：

```bash
atp db backup
```

## 技术实现

### 核心特性

1. **自动重连**: 自动尝试多种连接方式（TCP, SSH）
2. **并发处理**: 支持多主机并发检查和 QGA 验证
3. **容错机制**: 单个主机失败不影响其他主机检查
4. **多格式输出**: 支持表格、JSON、YAML 格式
5. **模式匹配**: 灵活的虚拟机名称匹配规则
6. **预览模式**: 所有批量操作支持 dry-run

### 性能指标

- 单主机检查时间: < 5秒
- 支持同时检查: 10+ 主机
- 内存占用: < 50MB
- 支持虚拟机数量: 1000+

## 更新日志

### v0.2.0 (2026-01-08)

- ✅ 新增 `disk-location` 命令 - 查询虚拟机磁盘存储位置
- ✅ 新增 `start` 命令 - 批量启动虚拟机（支持 QGA 验证）
- ✅ 新增 `assign` 命令 - 批量分配虚拟机给用户
- ✅ 新增 `rename` 命令 - 批量重命名虚拟机
- ✅ 新增 `set-auto-join-domain` 命令 - 批量设置自动加域
- ✅ `sync-hosts` 改为同步到数据库
- ✅ 支持 Gluster 分布式存储 brick 定位
- ✅ 支持组织单位批量分配
- ✅ 配置文件默认路径改为 `config/atp.toml`

### v0.1.0 (2025-12-08)

- ✅ 实现 `verify` 命令 - 虚拟机状态一致性验证
- ✅ 实现 `list-hosts` 命令 - 列出主机
- ✅ 实现 `list-vms` 命令 - 列出虚拟机
- ✅ 实现 `sync-hosts` 命令 - 同步主机
- ✅ 支持多种输出格式（table/json/yaml）
- ✅ 自动 MD5 密码加密
- ✅ Token 认证机制
- ✅ 多种 libvirt 连接方式（TCP/SSH）

---

**维护者**: OCloudView ATP Team
**更新时间**: 2026-01-08
**版本**: v0.2.0

# ATP 连通性测试使用指南

## 概述

本测试程序用于验证 VDI 平台和 Libvirt 的连接性。

## 前置要求

### 1. 安装 libvirt 开发库

在 Ubuntu/Debian 系统上：
```bash
sudo apt-get update
sudo apt-get install libvirt-dev pkg-config
```

在 CentOS/RHEL 系统上：
```bash
sudo yum install libvirt-devel pkgconfig
```

### 2. 配置测试环境

编辑配置文件 `/home/cloudyi/ocloudview-atp/test.toml`：

```toml
# VDI 平台配置
[vdi]
base_url = "http://192.168.41.51:8088"
username = "admin"
password = "11111111"
verify_ssl = false
connect_timeout = 10

# Libvirt 配置
[libvirt]
uri = "qemu:///system"  # 本地连接
# 或者远程连接: "qemu+ssh://root@HOST_IP/system"
```

## 运行测试

### 方式 1: 使用配置文件
```bash
cd /home/cloudyi/ocloudview-atp/atp-core/executor
cargo run --example test_connectivity
```

### 方式 2: 使用环境变量覆盖
```bash
export ATP_VDI_BASE_URL="http://192.168.41.51:8088"
export ATP_VDI_USERNAME="admin"
export ATP_VDI_PASSWORD="11111111"
export ATP_TEST_HOST="qemu:///system"

cd /home/cloudyi/ocloudview-atp/atp-core/executor
cargo run --example test_connectivity
```

## 测试内容

### 1. VDI 平台连接测试
- ✅ 检查 VDI 平台服务可访问性
- ✅ 测试登录认证
- ✅ 探测主机列表 API

### 2. Libvirt 连接测试
- ✅ 连接到 libvirtd
- ✅ 验证连接状态
- ✅ 获取虚拟机列表
- ✅ 显示虚拟机状态
- ✅ 显示主机信息和 libvirt 版本

## 预期输出

```
╔════════════════════════════════════════════════════════════════╗
║         ATP 连通性测试 - VDI 平台 & Libvirt                   ║
╚════════════════════════════════════════════════════════════════╝

📋 步骤 1/3: 加载测试配置...
   ✅ 配置加载成功

📡 步骤 2/3: 测试 VDI 平台连接...
   📌 VDI 平台地址: http://192.168.41.51:8088
   📌 用户名: admin
   ✅ VDI 平台服务可访问
   ✅ VDI 平台连接测试通过

🔌 步骤 3/3: 测试 Libvirt 连接...
   📌 Libvirt URI: qemu:///system
   ✅ 连接建立成功
   ✅ 连接状态正常
   📊 虚拟机总数: 5
   📋 虚拟机列表 (前5个):
      1. vm-001 (状态: Running)
      2. vm-002 (状态: Shutoff)
      ...
   🖥️  主机名: hostname
   📦 libvirt 版本: 8.0.0

╔════════════════════════════════════════════════════════════════╗
║                        测试总结                                ║
╠════════════════════════════════════════════════════════════════╣
║  VDI 平台连接:     ✅ 成功                                     ║
║  Libvirt 连接:     ✅ 成功                                     ║
╚════════════════════════════════════════════════════════════════╝

✅ 所有测试通过！
```

## 故障排查

### 问题 1: libvirt 库未找到
```
error: linking with `cc` failed
rust-lld: error: undefined symbol: virConnectOpen
```

**解决方法**:
```bash
sudo apt-get install libvirt-dev pkg-config
```

### 问题 2: VDI 平台连接失败
```
❌ VDI 平台连接失败: 无法连接到 VDI 平台
```

**检查清单**:
1. 确认 VDI 平台地址正确
2. 确认网络可达: `ping 192.168.41.51`
3. 确认端口开放: `telnet 192.168.41.51 8088`
4. 检查防火墙设置

### 问题 3: Libvirt 连接失败
```
❌ Libvirt 连接失败: 连接失败
```

**检查清单**:
1. 确认 libvirtd 服务运行: `sudo systemctl status libvirtd`
2. 启动服务: `sudo systemctl start libvirtd`
3. 检查权限: 当前用户是否在 libvirt 组
   ```bash
   sudo usermod -a -G libvirt $USER
   newgrp libvirt
   ```

### 问题 4: 远程 SSH 连接失败
```
❌ 连接失败: SSH 连接错误
```

**解决方法**:
1. 配置 SSH 密钥认证:
   ```bash
   ssh-keygen -t rsa
   ssh-copy-id root@HOST_IP
   ```
2. 测试 SSH 连接:
   ```bash
   ssh root@HOST_IP
   ```

## 下一步

连通性测试成功后，可以:
1. 从 VDI 平台获取主机列表
2. 连接到各个主机的 libvirtd
3. 运行完整的 E2E 测试场景

## 相关文档

- [测试配置指南](../docs/TESTING_CONFIG_GUIDE.md)
- [快速开始](../TEST_CONFIG_README.md)
- [E2E 测试指南](../docs/E2E_TESTING_GUIDE.md)

---

**创建日期**: 2025-12-08
**维护者**: OCloudView ATP Team

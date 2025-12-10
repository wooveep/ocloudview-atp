# ATP 连通性测试

## ✅ 已完成的工作

### 1. 配置文件更新
- 更新了 `test.toml` 配置文件
- 配置了 VDI 平台信息:
  - IP: 192.168.41.51
  - 用户名: admin  
  - 密码: 11111111

### 2. 连通性测试程序
创建了 `/home/cloudyi/ocloudview-atp/atp-core/executor/examples/test_connectivity.rs`

**功能**:
- ✅ VDI 平台 HTTP 连接测试
- ✅ VDI 平台登录认证测试
- ✅ VDI 平台 API 探测
- ✅ Libvirt 连接测试
- ✅ 虚拟机列表获取
- ✅ 主机信息显示

### 3. 文档
- ✅ `docs/CONNECTIVITY_TEST_GUIDE.md` - 详细使用指南

## 📋 运行测试前的准备

### 安装依赖

Ubuntu/Debian:
```bash
sudo apt-get update
sudo apt-get install libvirt-dev pkg-config
```

CentOS/RHEL:
```bash
sudo yum install libvirt-devel pkgconfig
```

### 启动 libvirtd
```bash
sudo systemctl start libvirtd
sudo systemctl enable libvirtd
```

### 配置权限 (可选)
```bash
sudo usermod -a -G libvirt $USER
newgrp libvirt
```

## 🚀 运行测试

```bash
cd /home/cloudyi/ocloudview-atp/atp-core/executor
cargo run --example test_connectivity
```

## 📊 预期结果

成功输出示例:
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
   📊 虚拟机总数: X
   🖥️  主机名: xxx
   📦 libvirt 版本: x.x.x

╔════════════════════════════════════════════════════════════════╗
║                        测试总结                                ║
╠════════════════════════════════════════════════════════════════╣
║  VDI 平台连接:     ✅ 成功                                     ║
║  Libvirt 连接:     ✅ 成功                                     ║
╚════════════════════════════════════════════════════════════════╝

✅ 所有测试通过！
```

## 🔧 故障排查

### VDI 平台连接失败
1. 检查网络: `ping 192.168.41.51`
2. 检查端口: `telnet 192.168.41.51 8088`
3. 检查防火墙设置

### Libvirt 连接失败
1. 检查服务: `sudo systemctl status libvirtd`
2. 检查权限: 确保用户在 libvirt 组
3. 查看日志: `journalctl -u libvirtd -n 50`

## 📚 更多文档

- [详细测试指南](docs/CONNECTIVITY_TEST_GUIDE.md)
- [测试配置说明](TEST_CONFIG_README.md)
- [测试配置指南](docs/TESTING_CONFIG_GUIDE.md)

---
**创建日期**: 2025-12-08

# ATP 测试配置使用说明

本项目现已支持灵活的测试配置管理!

## 快速开始

### 方式1: 使用环境变量 (最快)

```bash
export ATP_TEST_VM=my-test-vm
export ATP_TEST_HOST=qemu:///system

cd /home/cloudyi/ocloudview-atp/atp-core/executor
cargo test --test e2e_tests -- --nocapture
```

### 方式2: 使用配置文件 (推荐)

```bash
# 1. 创建配置文件
cp test.toml.example test.toml

# 2. 修改配置
vi test.toml  # 修改 vm.name 为您的测试虚拟机

# 3. 运行测试
cargo test --test e2e_tests -- --nocapture
```

### 方式3: 组合使用 (环境变量优先级更高)

```bash
# 配置文件中设置了 vm.name = "test-vm"
# 但可以用环境变量临时覆盖
export ATP_TEST_VM=another-vm

cargo test --test e2e_tests -- --nocapture
# 实际使用: another-vm
```

## 配置文件位置

测试会按以下优先级查找配置文件:

1. `$ATP_TEST_CONFIG` 环境变量指定的路径
2. `./test.toml` (当前目录)
3. `./tests/config.toml` (tests 目录)
4. `~/.config/atp/test.toml` (用户配置目录)
5. `/etc/atp/test.toml` (系统配置目录)

## 支持的环境变量

### 通用配置
- `ATP_TEST_MODE` - 测试模式 (unit/integration/e2e)
- `ATP_LOG_LEVEL` - 日志级别 (debug/info/warn/error)

### Libvirt 配置
- `ATP_TEST_HOST` - libvirt URI
- `ATP_CONNECT_TIMEOUT` - 连接超时 (秒)

### 虚拟机配置
- `ATP_TEST_VM` - 测试虚拟机名称
- `ATP_TEST_VM_USER` - VM 登录用户
- `ATP_TEST_VM_PASSWORD` - VM 登录密码

### 协议配置
- `ATP_QMP_SOCKET` - QMP Socket 路径前缀
- `ATP_SPICE_HOST` - SPICE 服务器地址
- `ATP_SPICE_PORT` - SPICE 端口

### VDI 平台配置
- `ATP_VDI_BASE_URL` - VDI 平台 API 地址
- `ATP_VDI_USERNAME` - VDI 平台用户名
- `ATP_VDI_PASSWORD` - VDI 平台密码
- `ATP_VDI_VERIFY_SSL` - 是否验证 SSL (true/false)

### 测试行为
- `ATP_TEST_TIMEOUT` - 测试超时 (秒)
- `ATP_TEST_RETRY` - 失败重试次数
- `ATP_TEST_SKIP_SLOW` - 跳过慢速测试 (true/false)

## 配置示例

### 最小配置 (test.toml)

```toml
[vm]
name = "test-vm"
```

### 完整配置 (参考 test.toml.example)

```toml
[environment]
mode = "e2e"
log_level = "debug"

[libvirt]
uri = "qemu:///system"

[vm]
name = "ubuntu-test-vm"
user = "test"
password = "test123"

[protocols.qmp]
socket_prefix = "/var/lib/libvirt/qemu/"
timeout = 30

[vdi]
base_url = "http://192.168.1.11:8088"
username = "admin"
password = "admin123"

[test]
timeout = 120
retry = 2
```

## 验证配置

创建一个简单的测试程序验证配置加载:

```rust
use atp_executor::TestConfig;

#[test]
fn test_load_config() {
    let config = TestConfig::load().unwrap();
    println!("VM name: {}", config.vm.name);
    println!("Libvirt URI: {}", config.libvirt.uri);
}
```

## 完整文档

- [测试配置指南](docs/TESTING_CONFIG_GUIDE.md) - 详细的配置指南
- [测试配置实施方案](docs/TEST_CONFIG_IMPLEMENTATION.md) - 技术实现细节

## 常见问题

### Q: 配置文件不生效?

检查文件路径和文件名:
```bash
# 确认配置文件存在
ls -la test.toml

# 查看日志确认配置是否加载
RUST_LOG=debug cargo test --test e2e_tests test_basic_scenario -- --nocapture
```

### Q: 环境变量覆盖不生效?

确认环境变量已设置:
```bash
echo $ATP_TEST_VM
printenv | grep ATP_
```

### Q: 如何只配置必要的项?

配置文件只需包含要修改的项,其他使用默认值:
```toml
# 只配置 VM 名称,其他使用默认值
[vm]
name = "my-vm"
```

---

**创建日期**: 2025-12-01
**维护者**: OCloudView ATP Team

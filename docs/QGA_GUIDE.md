# QEMU Guest Agent (QGA) 使用指南

## 概述

QEMU Guest Agent (QGA) 是一个运行在 Guest 操作系统内部的守护进程，它允许 Host 端通过 Libvirt 在 Guest 内执行命令、读写文件等操作，而无需通过网络（SSH）连接。

## 工作原理

### 通信机制

1. **Virtio-Serial 设备**
   - QGA 使用 virtio-serial 设备进行通信
   - Host 端：UNIX Domain Socket（通常位于 `/var/lib/libvirt/qemu/`）
   - Guest 端：
     - Linux: `/dev/virtio-ports/org.qemu.guest_agent.0`
     - Windows: 特定的串行设备句柄

2. **协议**
   - 基于 JSON 的 QMP 子集
   - 请求-响应模型
   - 不需要网络配置

3. **架构**
```
┌─────────────────┐
│ Test Controller │
│    (Rust)       │
└────────┬────────┘
         │ Libvirt API
         │ (virDomainQemuAgentCommand)
         ▼
┌─────────────────┐
│  Libvirt Daemon │
└────────┬────────┘
         │ UNIX Socket
         ▼
┌─────────────────┐
│  QEMU Process   │
└────────┬────────┘
         │ Virtio-Serial
         ▼
┌─────────────────┐
│  Guest Agent    │
│  (qemu-ga)      │
└─────────────────┘
```

## 前置条件

### 1. Guest 端安装 QGA

#### Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install qemu-guest-agent
sudo systemctl enable qemu-guest-agent
sudo systemctl start qemu-guest-agent
```

#### Linux (RHEL/CentOS)
```bash
sudo yum install qemu-guest-agent
sudo systemctl enable qemu-guest-agent
sudo systemctl start qemu-guest-agent
```

#### Windows
1. 下载 QEMU Guest Agent for Windows
2. 运行安装程序
3. 确认服务已启动：
```powershell
Get-Service QEMU-GA
```

### 2. 配置 Virtio-Serial 设备

编辑虚拟机 XML 配置（或在创建时添加）：

```xml
<domain type='kvm'>
  ...
  <devices>
    ...
    <channel type='unix'>
      <source mode='bind' path='/var/lib/libvirt/qemu/channel/target/domain-VM_NAME/org.qemu.guest_agent.0'/>
      <target type='virtio' name='org.qemu.guest_agent.0'/>
      <address type='virtio-serial' controller='0' bus='0' port='1'/>
    </channel>
  </devices>
</domain>
```

或使用 virsh 命令：
```bash
virsh edit your-vm-name
```

### 3. 验证 QGA 可用性

#### Host 端测试
```bash
# 使用 virsh
virsh qemu-agent-command your-vm-name '{"execute":"guest-ping"}'

# 应返回：
# {"return":{}}
```

#### Guest 端检查

**Linux:**
```bash
# 检查服务状态
systemctl status qemu-guest-agent

# 检查设备文件
ls -l /dev/virtio-ports/org.qemu.guest_agent.0
```

**Windows:**
```powershell
# 检查服务
Get-Service QEMU-GA

# 查看服务日志
Get-EventLog -LogName Application -Source "qemu-ga" -Newest 10
```

## Rust API 使用

### 基本用法

```rust
use ocloudview_atp::libvirt::LibvirtManager;
use ocloudview_atp::qga::QgaClient;

// 连接到 Libvirt
let libvirt = LibvirtManager::connect()?;

// 查找虚拟机
let domain = libvirt.lookup_domain_by_name("test-vm")?;

// 创建 QGA 客户端
let qga = QgaClient::new(&domain);

// 测试连通性
qga.ping()?;
```

### 执行命令

#### 方式 1: 简单 Shell 命令
```rust
// 自动根据 OS 选择 Shell（Linux: /bin/sh, Windows: cmd.exe）
let result = qga.exec_shell("ls -la /tmp")?;

if let Some(stdout) = result.decode_stdout() {
    println!("输出:\n{}", stdout);
}

if let Some(exit_code) = result.exit_code {
    println!("退出码: {}", exit_code);
}
```

#### 方式 2: 手动构造命令
```rust
use ocloudview_atp::qga::GuestExecCommand;

// Linux 示例
let cmd = GuestExecCommand::simple(
    "/usr/bin/python3",
    vec!["-c".to_string(), "print('Hello QGA')".to_string()],
);

let pid = qga.exec(cmd)?;

// 轮询状态
loop {
    let status = qga.exec_status(pid)?;
    if status.exited {
        break;
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
}
```

#### 方式 3: 执行并等待完成
```rust
use std::time::Duration;

let cmd = GuestExecCommand::simple(
    "/bin/sleep",
    vec!["5".to_string()],
);

// 自动轮询直到完成
let result = qga.exec_and_wait(cmd, Duration::from_millis(500))?;
```

### 文件操作

#### 读取文件
```rust
// 方式 1: 直接读取整个文件
let content = qga.read_file("/etc/hostname")?;
println!("主机名: {}", content.trim());

// 方式 2: 手动控制（适合大文件）
let handle = qga.file_open("/var/log/syslog", Some("r"))?;

loop {
    let result = qga.file_read(handle, Some(4096))?;

    if let Ok(data) = base64::decode(&result.buf_b64) {
        // 处理数据
        println!("读取 {} 字节", data.len());
    }

    if result.eof {
        break;
    }
}

qga.file_close(handle)?;
```

#### 写入文件
```rust
// 方式 1: 直接写入
qga.write_file("/tmp/test.txt", "Hello from QGA")?;

// 方式 2: 手动控制
let handle = qga.file_open("/tmp/output.log", Some("w"))?;
qga.file_write(handle, b"Line 1\n")?;
qga.file_write(handle, b"Line 2\n")?;
qga.file_close(handle)?;
```

### 获取系统信息

```rust
// Guest Agent 信息
let info = qga.get_info()?;
println!("QGA 版本: {}", info.version);
println!("支持的命令:");
for cmd in info.supported_commands {
    if cmd.enabled {
        println!("  - {}", cmd.name);
    }
}

// 操作系统信息
let os_info = qga.get_osinfo()?;
println!("操作系统: {:?}", os_info.pretty_name);
println!("内核版本: {:?}", os_info.kernel_release);
println!("架构: {:?}", os_info.machine);
```

## 实际应用场景

### 场景 1: 自动化测试环境准备

```rust
fn setup_test_environment(domain: &Domain) -> Result<()> {
    let qga = QgaClient::new(domain);

    // 1. 检查 QGA 可用性
    qga.ping()?;

    // 2. 创建测试目录
    qga.exec_shell("mkdir -p /tmp/test-env")?;

    // 3. 复制测试脚本
    let test_script = include_str!("test_script.sh");
    qga.write_file("/tmp/test-env/run.sh", test_script)?;

    // 4. 设置执行权限
    qga.exec_shell("chmod +x /tmp/test-env/run.sh")?;

    // 5. 运行测试
    let result = qga.exec_shell("/tmp/test-env/run.sh")?;

    Ok(())
}
```

### 场景 2: 收集诊断信息

```rust
fn collect_diagnostics(domain: &Domain) -> Result<String> {
    let qga = QgaClient::new(domain);
    let mut report = String::new();

    // 系统信息
    let os_info = qga.get_osinfo()?;
    report.push_str(&format!("OS: {:?}\n", os_info.pretty_name));

    // CPU 信息
    let cpu_info = qga.exec_shell("lscpu")?;
    if let Some(output) = cpu_info.decode_stdout() {
        report.push_str(&format!("\nCPU Info:\n{}\n", output));
    }

    // 内存使用
    let mem_info = qga.exec_shell("free -h")?;
    if let Some(output) = mem_info.decode_stdout() {
        report.push_str(&format!("\nMemory:\n{}\n", output));
    }

    // 读取日志
    let logs = qga.read_file("/var/log/syslog")?;
    report.push_str(&format!("\nRecent Logs:\n{}\n",
        logs.lines().rev().take(50).collect::<Vec<_>>().join("\n")));

    Ok(report)
}
```

### 场景 3: 配置文件管理

```rust
fn update_config(domain: &Domain, config_path: &str, new_config: &str) -> Result<()> {
    let qga = QgaClient::new(domain);

    // 1. 备份原配置
    let backup_cmd = format!("cp {} {}.bak", config_path, config_path);
    qga.exec_shell(&backup_cmd)?;

    // 2. 写入新配置
    qga.write_file(config_path, new_config)?;

    // 3. 验证配置
    let verify_result = qga.exec_shell(&format!("cat {}", config_path))?;

    // 4. 重启服务（如果需要）
    qga.exec_shell("systemctl reload nginx")?;

    Ok(())
}
```

### 场景 4: 跨平台脚本执行

```rust
fn execute_cross_platform(domain: &Domain, task: &str) -> Result<()> {
    let qga = QgaClient::new(domain);

    // 获取操作系统类型
    let os_info = qga.get_osinfo()?;

    let command = match os_info.id.as_deref() {
        Some("windows") => {
            // Windows 命令
            match task {
                "check_service" => "sc query QEMU-GA",
                "restart_service" => "net stop QEMU-GA && net start QEMU-GA",
                _ => return Err(anyhow::anyhow!("Unknown task")),
            }
        }
        _ => {
            // Linux 命令
            match task {
                "check_service" => "systemctl status qemu-guest-agent",
                "restart_service" => "systemctl restart qemu-guest-agent",
                _ => return Err(anyhow::anyhow!("Unknown task")),
            }
        }
    };

    let result = qga.exec_shell(command)?;
    println!("命令输出:\n{:?}", result.decode_stdout());

    Ok(())
}
```

## 最佳实践

### 1. 错误处理

```rust
fn robust_execution(domain: &Domain) -> Result<()> {
    let qga = QgaClient::new(domain)
        .with_timeout(60); // 设置超时

    // 带重试的执行
    let max_retries = 3;
    for attempt in 1..=max_retries {
        match qga.exec_shell("some-command") {
            Ok(result) => {
                if result.exit_code == Some(0) {
                    return Ok(());
                }
            }
            Err(e) if attempt < max_retries => {
                eprintln!("尝试 {} 失败: {}, 重试中...", attempt, e);
                std::thread::sleep(std::time::Duration::from_secs(2));
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Err(anyhow::anyhow!("所有重试均失败"))
}
```

### 2. 大文件处理

```rust
fn read_large_file(domain: &Domain, path: &str) -> Result<Vec<u8>> {
    let qga = QgaClient::new(domain);

    let handle = qga.file_open(path, Some("r"))?;
    let mut buffer = Vec::new();

    loop {
        let result = qga.file_read(handle, Some(1024 * 1024))?; // 1MB chunks

        if let Ok(data) = base64::decode(&result.buf_b64) {
            buffer.extend_from_slice(&data);
        }

        if result.eof {
            break;
        }
    }

    qga.file_close(handle)?;

    Ok(buffer)
}
```

### 3. 权限管理

```rust
// 某些操作需要 root 权限
fn execute_with_sudo(domain: &Domain, command: &str) -> Result<()> {
    let qga = QgaClient::new(domain);

    // 方式 1: 使用 sudo（需要无密码 sudo）
    let sudo_cmd = format!("sudo {}", command);
    qga.exec_shell(&sudo_cmd)?;

    // 方式 2: 使用 su（不推荐）
    let su_cmd = format!("su -c '{}'", command);
    qga.exec_shell(&su_cmd)?;

    Ok(())
}
```

## 故障排查

### 问题 1: QGA 命令超时

**原因:**
- Guest Agent 未运行
- Virtio-Serial 设备未配置
- 网络延迟（虽然 QGA 不使用网络）

**解决方案:**
```bash
# Host 端检查
virsh qemu-agent-command vm-name '{"execute":"guest-ping"}'

# Guest 端检查
systemctl status qemu-guest-agent  # Linux
Get-Service QEMU-GA                # Windows
```

### 问题 2: 命令执行失败

**检查退出码:**
```rust
let result = qga.exec_shell("false")?;
if let Some(code) = result.exit_code {
    if code != 0 {
        eprintln!("命令失败，退出码: {}", code);
        if let Some(stderr) = result.decode_stderr() {
            eprintln!("错误信息: {}", stderr);
        }
    }
}
```

### 问题 3: 文件权限错误

```rust
// 检查文件是否可读
let test_read = qga.exec_shell("test -r /path/to/file && echo 'readable' || echo 'not readable'")?;

// 修改权限
qga.exec_shell("chmod 644 /path/to/file")?;
```

## 安全考虑

1. **权限控制**: QGA 以 root 权限运行，需要严格控制访问
2. **命令注入**: 永远不要直接拼接用户输入到命令中
3. **文件访问**: 限制可访问的文件路径
4. **审计日志**: 记录所有 QGA 操作

```rust
// 安全的参数处理示例
fn safe_command_execution(domain: &Domain, user_input: &str) -> Result<()> {
    // 白名单验证
    let allowed_commands = vec!["status", "info", "version"];

    if !allowed_commands.contains(&user_input) {
        return Err(anyhow::anyhow!("不允许的命令"));
    }

    // 记录审计日志
    info!("执行 QGA 命令: {}", user_input);

    let qga = QgaClient::new(domain);
    qga.exec_shell(user_input)?;

    Ok(())
}
```

## 参考资源

- [QEMU Guest Agent 官方文档](https://qemu.readthedocs.io/en/latest/interop/qemu-ga.html)
- [Libvirt Guest Agent 支持](https://libvirt.org/formatdomain.html#qemu-guest-agent)
- [QMP 协议规范](https://qemu.readthedocs.io/en/latest/interop/qmp-spec.html)

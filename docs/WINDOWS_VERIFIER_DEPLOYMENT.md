# Windows Guest 验证器部署指南

## 概述

本文档描述如何在 Windows Guest OS 中部署和运行 Guest 验证器 Agent。

## 系统要求

### 最低要求
- **操作系统**: Windows 10/11 或 Windows Server 2016/2019/2022
- **架构**: x86_64 (64位)
- **.NET Framework**: 无需（Rust 原生应用）
- **内存**: 最小 50MB
- **磁盘空间**: 10MB

### 推荐配置
- **操作系统**: Windows 10 21H2+ 或 Windows 11
- **内存**: 100MB
- **网络**: 稳定的网络连接到 ATP 验证服务器

## 构建

### 在 Windows 上构建

#### 1. 安装 Rust 工具链

下载并安装 Rust：
```powershell
# 下载 rustup-init.exe
# https://www.rust-lang.org/tools/install

# 运行安装程序
.\rustup-init.exe

# 添加 Rust 到 PATH（安装程序会自动添加）
# 重启 PowerShell 或 CMD
```

验证安装：
```powershell
rustc --version
cargo --version
```

#### 2. 安装 Visual Studio Build Tools

需要 MSVC 工具链来编译 Windows API 绑定：

```powershell
# 下载 Visual Studio Build Tools
# https://visualstudio.microsoft.com/downloads/

# 安装时选择：
# - "C++ build tools"
# - "Windows 10 SDK" 或 "Windows 11 SDK"
```

#### 3. 克隆项目

```powershell
git clone https://github.com/your-org/ocloudview-atp.git
cd ocloudview-atp/guest-verifier
```

#### 4. 构建 Release 版本

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

构建产物位于：
```
target\x86_64-pc-windows-msvc\release\verifier-agent.exe
```

### 交叉编译（从 Linux）

#### 1. 安装交叉编译工具链

```bash
# 添加 Windows 目标
rustup target add x86_64-pc-windows-gnu

# 安装 MinGW 交叉编译器
sudo apt-get install mingw-w64
```

#### 2. 配置 Cargo

创建或编辑 `~/.cargo/config.toml`:

```toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"
```

#### 3. 构建

```bash
cd guest-verifier
cargo build --release --target x86_64-pc-windows-gnu
```

**注意**: 使用 MinGW 编译时，Windows Hooks API 可能有兼容性问题。推荐使用 MSVC 工具链。

## 安装

### 手动安装

#### 1. 复制可执行文件

```powershell
# 创建安装目录
New-Item -ItemType Directory -Force -Path "C:\Program Files\ATP\GuestVerifier"

# 复制可执行文件
Copy-Item "target\release\verifier-agent.exe" -Destination "C:\Program Files\ATP\GuestVerifier\"
```

#### 2. 配置防火墙（如果需要）

```powershell
# 允许出站连接到验证服务器
New-NetFirewallRule -DisplayName "ATP Guest Verifier" `
    -Direction Outbound `
    -Program "C:\Program Files\ATP\GuestVerifier\verifier-agent.exe" `
    -Action Allow
```

### 作为 Windows 服务安装（可选）

#### 使用 NSSM (Non-Sucking Service Manager)

1. 下载 NSSM：https://nssm.cc/download

2. 安装服务：

```powershell
# 解压 NSSM
Expand-Archive -Path nssm-2.24.zip -DestinationPath C:\Tools\

# 安装服务
C:\Tools\nssm-2.24\win64\nssm.exe install ATPGuestVerifier `
    "C:\Program Files\ATP\GuestVerifier\verifier-agent.exe" `
    --server ws://192.168.1.100:8765 `
    --vm-id your-vm-id `
    --log-level info

# 设置服务描述
C:\Tools\nssm-2.24\win64\nssm.exe set ATPGuestVerifier Description "ATP Guest Verifier Agent"

# 设置自动启动
C:\Tools\nssm-2.24\win64\nssm.exe set ATPGuestVerifier Start SERVICE_AUTO_START

# 启动服务
Start-Service ATPGuestVerifier

# 查看服务状态
Get-Service ATPGuestVerifier
```

3. 查看日志：

```powershell
# NSSM 默认将日志输出到 Windows 事件查看器
# 或配置日志文件路径：
C:\Tools\nssm-2.24\win64\nssm.exe set ATPGuestVerifier `
    AppStdout "C:\ProgramData\ATP\verifier-agent.log"
C:\Tools\nssm-2.24\win64\nssm.exe set ATPGuestVerifier `
    AppStderr "C:\ProgramData\ATP\verifier-agent-error.log"
```

## 运行

### 基本用法

```powershell
# 最简单的方式（自动检测 VM ID）
.\verifier-agent.exe --server ws://192.168.1.100:8765

# 指定 VM ID
.\verifier-agent.exe --server ws://192.168.1.100:8765 --vm-id windows-vm-01

# 指定日志级别
.\verifier-agent.exe --server ws://192.168.1.100:8765 --log-level debug

# 禁用自动重连
.\verifier-agent.exe --server ws://192.168.1.100:8765 --auto-reconnect false
```

### 命令行选项

```
Options:
  -s, --server <SERVER>
          服务器地址 (例如: localhost:8080 或 ws://localhost:8080)
          [default: localhost:8080]

      --vm-id <VM_ID>
          虚拟机 ID（用于标识客户端）
          如果不指定，会自动尝试获取（从主机名）

  -t, --transport <TRANSPORT>
          传输类型 [websocket, tcp]
          [default: websocket]

  -v, --verifiers <VERIFIERS>
          启用的验证器类型 (可多次指定)
          [可选值: keyboard, mouse, command, all]
          [默认: all]

  -l, --log-level <LOG_LEVEL>
          日志级别 [trace, debug, info, warn, error]
          [default: info]

      --auto-reconnect
          自动重连
          [default: true]

      --reconnect-interval <RECONNECT_INTERVAL>
          重连间隔（秒）
          [default: 5]

  -h, --help
          显示帮助信息

  -V, --version
          显示版本信息
```

### 后台运行（不使用服务）

使用 PowerShell Job:

```powershell
# 启动后台作业
Start-Job -Name ATPVerifier -ScriptBlock {
    & "C:\Program Files\ATP\GuestVerifier\verifier-agent.exe" `
        --server ws://192.168.1.100:8765 `
        --vm-id windows-vm-01
}

# 查看作业状态
Get-Job -Name ATPVerifier

# 查看输出
Receive-Job -Name ATPVerifier -Keep

# 停止作业
Stop-Job -Name ATPVerifier
Remove-Job -Name ATPVerifier
```

## 权限要求

### Low-Level Hooks

Windows Guest 验证器使用 Low-Level Keyboard/Mouse Hooks (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`)，这些 API 的权限要求：

#### 标准用户权限
- ✅ **可以使用** Low-Level Hooks
- ✅ 不需要管理员权限
- ✅ 可以监听全局键盘/鼠标事件

#### 特殊情况
某些场景可能需要管理员权限：
- 🔒 UAC 提示窗口的事件可能无法捕获
- 🔒 以管理员权限运行的应用程序事件
- 🔒 安全桌面（Secure Desktop）上的事件

**建议**: 如果需要完整覆盖，以管理员权限运行：

```powershell
# 以管理员身份运行 PowerShell
# 右键点击 PowerShell -> "以管理员身份运行"

# 然后运行 Agent
.\verifier-agent.exe --server ws://192.168.1.100:8765
```

### 安全软件冲突

某些杀毒软件或安全工具可能会拦截 Hook 行为：

1. **Windows Defender**
   - 通常不会拦截 Low-Level Hooks
   - 如果被拦截，添加排除项

2. **第三方杀毒软件**
   - 可能检测到 "Hook Behavior" 并拦截
   - 添加 `verifier-agent.exe` 到白名单

3. **配置 Windows Defender 排除项**:

```powershell
# 添加程序排除
Add-MpPreference -ExclusionProcess "verifier-agent.exe"

# 添加路径排除
Add-MpPreference -ExclusionPath "C:\Program Files\ATP\GuestVerifier"
```

## VM ID 自动检测

### Windows 环境下的 VM ID 检测

在 Windows 上，Agent 使用以下方法自动获取 VM ID：

1. **计算机名** (默认方法)
   ```powershell
   hostname
   ```

2. **手动指定** (推荐)
   ```powershell
   .\verifier-agent.exe --vm-id "windows-vm-01"
   ```

### 配置计算机名

确保计算机名与 ATP 平台使用的 VM 名称一致：

```powershell
# 查看当前计算机名
$env:COMPUTERNAME

# 修改计算机名（需要重启）
Rename-Computer -NewName "windows-vm-01" -Restart

# 或通过系统设置修改：
# 设置 -> 系统 -> 关于 -> 重命名这台电脑
```

## 故障排查

### 1. Hook 安装失败

**错误**: `Failed to set keyboard/mouse hook`

**原因**:
- 消息循环异常
- 权限不足
- 安全软件拦截

**解决方案**:
```powershell
# 1. 以管理员权限运行
# 2. 检查安全软件日志
# 3. 添加排除项
# 4. 启用调试日志查看详情
.\verifier-agent.exe --server ws://192.168.1.100:8765 --log-level debug
```

### 2. 无法连接到服务器

**错误**: `连接到服务器失败`

**检查步骤**:
```powershell
# 1. 测试网络连通性
Test-NetConnection -ComputerName 192.168.1.100 -Port 8765

# 2. 检查防火墙
Get-NetFirewallRule -DisplayName "ATP*"

# 3. 尝试使用 TCP 而非 WebSocket
.\verifier-agent.exe --server 192.168.1.100:8765 --transport tcp
```

### 3. 事件未被检测到

**问题**: Agent 运行正常，但按键/鼠标事件未被验证

**检查**:
```powershell
# 1. 确认验证器已启用
# 查看日志中的 "启用键盘验证器" 等消息

# 2. 确认不是在 UAC 提示或管理员窗口中操作
# 这些窗口的事件需要管理员权限才能捕获

# 3. 检查日志中的 "检测到按键" 消息
# 如果没有，说明 Hook 未生效
```

### 4. 性能问题

**问题**: CPU 占用高或响应慢

**解决方案**:
```powershell
# 1. 降低日志级别
.\verifier-agent.exe --server ws://192.168.1.100:8765 --log-level warn

# 2. 只启用需要的验证器
.\verifier-agent.exe --server ws://192.168.1.100:8765 -v keyboard -v mouse

# 3. 检查事件队列是否堆积（查看日志）
```

### 5. 查看详细日志

```powershell
# 启用最详细日志
.\verifier-agent.exe --server ws://192.168.1.100:8765 --log-level trace

# 将日志输出到文件
.\verifier-agent.exe --server ws://192.168.1.100:8765 2>&1 | Tee-Object -FilePath agent.log
```

## 卸载

### 停止和删除服务

```powershell
# 停止服务
Stop-Service ATPGuestVerifier

# 删除服务
C:\Tools\nssm-2.24\win64\nssm.exe remove ATPGuestVerifier confirm
```

### 删除文件

```powershell
# 删除程序文件
Remove-Item -Recurse -Force "C:\Program Files\ATP\GuestVerifier"

# 删除日志文件
Remove-Item -Recurse -Force "C:\ProgramData\ATP"

# 删除防火墙规则
Remove-NetFirewallRule -DisplayName "ATP Guest Verifier"

# 删除 Windows Defender 排除项
Remove-MpPreference -ExclusionProcess "verifier-agent.exe"
Remove-MpPreference -ExclusionPath "C:\Program Files\ATP\GuestVerifier"
```

## 性能优化

### 1. 事件队列大小

默认队列大小为 100 个事件。如果测试频率很高，可能需要调整代码中的队列大小限制。

### 2. 轮询间隔

当前实现使用 10ms 的轮询间隔。可以根据延迟要求调整：

- **更低延迟**: 减小到 5ms（增加 CPU 占用）
- **更低 CPU 占用**: 增加到 20ms（略微增加延迟）

### 3. 系统资源

推荐配置：
- **CPU**: < 5% (单核)
- **内存**: < 50MB
- **网络**: < 1 Mbps

## 最佳实践

### 1. VM 镜像准备

在创建 Windows VM 镜像时预装 Agent：

```powershell
# 1. 构建 Agent
cargo build --release

# 2. 安装到标准位置
New-Item -ItemType Directory -Force -Path "C:\Program Files\ATP\GuestVerifier"
Copy-Item "target\release\verifier-agent.exe" -Destination "C:\Program Files\ATP\GuestVerifier\"

# 3. 创建启动脚本
$script = @"
Start-Process -FilePath "C:\Program Files\ATP\GuestVerifier\verifier-agent.exe" `
    -ArgumentList "--server ws://192.168.1.100:8765" `
    -NoNewWindow
"@
Set-Content -Path "C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Startup\ATP-Verifier.ps1" -Value $script

# 4. 创建快照/模板
```

### 2. 自动化部署

使用 Ansible 或 PowerShell Remoting:

```powershell
# ansible playbook 示例
# - name: Install ATP Guest Verifier
#   win_copy:
#     src: verifier-agent.exe
#     dest: C:\Program Files\ATP\GuestVerifier\
#
# - name: Install as service
#   win_nssm:
#     name: ATPGuestVerifier
#     application: C:\Program Files\ATP\GuestVerifier\verifier-agent.exe
#     app_parameters: --server ws://{{ verifier_server }} --vm-id {{ inventory_hostname }}
```

### 3. 监控和告警

监控 Agent 状态：

```powershell
# 检查进程是否运行
Get-Process -Name verifier-agent -ErrorAction SilentlyContinue

# 检查服务状态（如果安装为服务）
Get-Service -Name ATPGuestVerifier | Select-Object Status, StartType

# 检查网络连接
Get-NetTCPConnection -OwningProcess (Get-Process -Name verifier-agent).Id
```

## 安全考虑

### 1. Hook 安全

- Low-Level Hooks 运行在用户空间，无法访问内核
- 无法读取其他进程的内存
- 只能接收键盘/鼠标事件，不能修改或阻止

### 2. 网络安全

- 使用 WebSocket over TLS (wss://) 加密传输
- 验证服务器证书
- 使用防火墙限制连接目标

### 3. 数据隐私

- Agent 只记录按键名称（如 "A", "ENTER"），不记录完整文本
- 鼠标事件只记录按键类型，不记录敏感坐标
- 命令执行记录输出，但可以配置排除敏感命令

## 已知限制

1. **UAC 提示**: 无法捕获 UAC 提示窗口的输入事件（需要管理员权限）
2. **安全桌面**: 无法捕获 Ctrl+Alt+Del 安全桌面的事件
3. **虚拟键码**: 某些特殊按键可能无映射
4. **性能**: 高频率输入可能有轻微延迟（< 20ms）

## 参考资源

- [Windows Hooks - Microsoft Docs](https://docs.microsoft.com/en-us/windows/win32/winmsg/hooks)
- [NSSM - Non-Sucking Service Manager](https://nssm.cc/)
- [Rust on Windows](https://doc.rust-lang.org/book/ch01-01-installation.html#installing-rustup-on-windows)

---

**文档版本**: 1.0
**更新日期**: 2025-12-01
**作者**: OCloudView ATP Team

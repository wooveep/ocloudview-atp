# Windows Guest 验证器实现指南

## 概述

本文档描述 Windows Guest 验证器的实现方案，包括键盘、鼠标和命令执行验证器。

## 架构设计

### 1. 键盘验证器 (Windows Hook API)

Windows 键盘验证器使用 Windows Low-Level Keyboard Hook (`WH_KEYBOARD_LL`) 来监听全局键盘事件。

**技术要点**:
- 使用 `SetWindowsHookEx` 设置键盘钩子
- 使用 `GetMessage` 消息循环接收事件
- 多线程架构：Hook 线程 + Tokio 异步运行时
- 使用 `Arc<Mutex<VecDeque>>` 作为事件队列

**实现步骤**:
1. 创建 Hook 线程 (Windows 消息循环)
2. 在 Hook 回调中捕获键盘事件
3. 将事件推送到线程安全队列
4. 异步验证方法从队列读取并匹配事件

### 2. 鼠标验证器 (Windows Hook API)

Windows 鼠标验证器使用 Windows Low-Level Mouse Hook (`WH_MOUSE_LL`) 来监听全局鼠标事件。

**技术要点**:
- 使用 `SetWindowsHookEx` 设置鼠标钩子
- 支持鼠标按键事件 (左键、右键、中键)
- 支持鼠标移动事件
- 多线程架构同键盘验证器

**实现步骤**:
1. 创建 Hook 线程 (Windows 消息循环)
2. 在 Hook 回调中捕获鼠标事件
3. 区分按键事件 (WM_LBUTTONDOWN, WM_RBUTTONDOWN 等)
4. 区分移动事件 (WM_MOUSEMOVE)
5. 异步验证匹配

### 3. 命令执行验证器

命令执行验证器在 Windows 上需要适配 Windows 命令行环境。

**技术要点**:
- 使用 `cmd.exe` 或 `powershell.exe` 执行命令
- 当前实现已支持跨平台，只需确认 Windows 兼容性

## Windows API 依赖

### Cargo.toml 配置

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.52", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
] }
```

### 主要 API

#### 键盘钩子
- `SetWindowsHookExW` - 设置钩子
- `UnhookWindowsHookEx` - 移除钩子
- `CallNextHookEx` - 调用下一个钩子
- `GetMessageW` - 获取消息
- `TranslateMessage` / `DispatchMessageW` - 处理消息

#### 鼠标钩子
- 同键盘钩子 API
- 消息类型: `WM_LBUTTONDOWN`, `WM_RBUTTONDOWN`, `WM_MBUTTONDOWN`, `WM_MOUSEMOVE`

## 实现细节

### 键盘验证器实现框架

```rust
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::Foundation::*;

pub struct WindowsKeyboardVerifier {
    event_queue: Arc<Mutex<VecDeque<KeyEvent>>>,
    hook_handle: Arc<Mutex<Option<HHOOK>>>,
}

impl WindowsKeyboardVerifier {
    pub fn new() -> Result<Self> {
        let event_queue = Arc::new(Mutex::new(VecDeque::new()));
        let hook_handle = Arc::new(Mutex::new(None));

        // 启动 Hook 线程
        let queue_clone = event_queue.clone();
        let handle_clone = hook_handle.clone();

        std::thread::spawn(move || {
            Self::hook_thread(queue_clone, handle_clone);
        });

        Ok(Self {
            event_queue,
            hook_handle,
        })
    }

    fn hook_thread(
        event_queue: Arc<Mutex<VecDeque<KeyEvent>>>,
        hook_handle: Arc<Mutex<Option<HHOOK>>>,
    ) {
        unsafe {
            // 设置钩子
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                None,
                0,
            ).expect("Failed to set keyboard hook");

            *hook_handle.lock().unwrap() = Some(hook);

            // 消息循环
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).into() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // 清理
            UnhookWindowsHookEx(hook);
        }
    }
}

// Hook 回调函数
unsafe extern "system" fn keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        // 处理键盘事件
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);

        // wparam 表示消息类型 (WM_KEYDOWN, WM_KEYUP)
        if wparam.0 as u32 == WM_KEYDOWN {
            // 提取虚拟键码
            let vk_code = kb.vkCode;

            // 推送到队列
            // (需要通过全局变量或其他方式访问 event_queue)
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}
```

### 关键技术挑战

#### 1. 全局状态管理

Hook 回调函数是 `extern "system"` 函数，无法直接访问 Rust 结构体。

**解决方案**:
- 使用全局静态变量 (`lazy_static` 或 `once_cell`)
- 使用线程本地存储 (TLS)
- 通过 `SetWindowsHookEx` 的用户数据参数传递上下文 (有限)

**推荐方案**: 使用 `lazy_static` + `Arc<Mutex<>>`

```rust
use lazy_static::lazy_static;

lazy_static! {
    static ref KEYBOARD_EVENTS: Arc<Mutex<VecDeque<KeyEvent>>> =
        Arc::new(Mutex::new(VecDeque::new()));
}

unsafe extern "system" fn keyboard_proc(...) -> LRESULT {
    // 可以直接访问 KEYBOARD_EVENTS
    if let Ok(mut queue) = KEYBOARD_EVENTS.lock() {
        queue.push_back(event);
    }
    // ...
}
```

#### 2. 虚拟键码映射

Windows 使用虚拟键码 (Virtual-Key Code)，需要映射到可读的按键名称。

**常用映射**:
```rust
fn vk_code_to_key_name(vk_code: u32) -> Option<String> {
    match vk_code {
        0x41..=0x5A => Some(format!("{}", (vk_code as u8) as char)), // A-Z
        0x30..=0x39 => Some(format!("{}", (vk_code - 0x30))), // 0-9
        VK_RETURN => Some("ENTER".to_string()),
        VK_SPACE => Some("SPACE".to_string()),
        VK_ESCAPE => Some("ESC".to_string()),
        VK_TAB => Some("TAB".to_string()),
        // ... 更多映射
        _ => None,
    }
}
```

#### 3. 线程安全和生命周期

Hook 线程需要在整个验证器生命周期内保持运行。

**解决方案**:
- 在 `Drop` trait 中发送退出消息
- 使用 `PostThreadMessageW` 发送 `WM_QUIT`

```rust
impl Drop for WindowsKeyboardVerifier {
    fn drop(&mut self) {
        unsafe {
            // 向 Hook 线程发送退出消息
            if let Some(hook) = *self.hook_handle.lock().unwrap() {
                PostQuitMessage(0);
            }
        }
    }
}
```

## 测试策略

### 单元测试

由于 Hook 需要真实的 Windows 环境，单元测试主要测试：
1. 验证器创建和初始化
2. 虚拟键码映射函数
3. 事件匹配逻辑

### 集成测试

需要手动测试：
1. 启动 Agent
2. 发送测试事件
3. 手动按键/点击鼠标
4. 验证结果返回

### 自动化测试

可以使用 `SendInput` API 模拟输入：

```rust
#[cfg(test)]
mod tests {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

    #[test]
    fn test_keyboard_hook() {
        let verifier = WindowsKeyboardVerifier::new().unwrap();

        // 模拟按键
        unsafe {
            let mut input = INPUT::default();
            input.r#type = INPUT_KEYBOARD;
            input.Anonymous.ki.wVk = VK_A;

            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }

        // 等待并验证
        // ...
    }
}
```

## 部署指南

### 构建

```bash
# 在 Windows 系统上
cd guest-verifier
cargo build --release --target x86_64-pc-windows-msvc
```

### 权限要求

Windows 不需要特殊权限来设置 Low-Level Hooks。但是：
- 某些安全软件可能会拦截 Hook 行为
- UAC 环境下可能需要管理员权限

### 安装为 Windows 服务

可以使用 `windows-service` crate 将 Agent 安装为 Windows 服务：

```rust
use windows_service::{
    define_windows_service,
    service_dispatcher,
    service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType},
};

define_windows_service!(ffi_service_main, service_main);

fn service_main(args: Vec<OsString>) {
    // 启动 Agent
}
```

## 依赖项添加

需要在 `Cargo.toml` 中添加：

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.52", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
] }
lazy_static = "1.4"
```

## 已知问题和限制

1. **Hook 延迟**: Low-Level Hooks 在高负载下可能导致系统延迟
2. **安全软件冲突**: 某些杀毒软件会阻止 Hook 安装
3. **管理员权限**: 某些场景需要管理员权限
4. **Unicode 支持**: 虚拟键码映射可能无法完全覆盖所有字符

## 后续改进

1. 实现更完善的虚拟键码映射表
2. 添加错误重试机制
3. 支持修饰键 (Ctrl, Alt, Shift) 检测
4. 实现鼠标坐标记录
5. 添加性能监控和统计

## 参考文档

- [Windows Hooks - Microsoft Docs](https://docs.microsoft.com/en-us/windows/win32/winmsg/hooks)
- [SetWindowsHookEx - Microsoft Docs](https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw)
- [Virtual-Key Codes - Microsoft Docs](https://docs.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes)
- [windows-rs Crate](https://github.com/microsoft/windows-rs)

---

**文档版本**: 1.0
**更新日期**: 2025-12-01
**作者**: OCloudView ATP Team

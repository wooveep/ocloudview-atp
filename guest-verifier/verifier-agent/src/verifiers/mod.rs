//! 验证器实现模块

pub mod keyboard;
pub mod mouse;

// 命令验证器模块 (保留用于兼容)
#[allow(dead_code)]
pub mod command;

// 导出命令验证器（跨平台）(保留用于兼容)
#[allow(unused_imports)]
pub use command::CommandVerifier;

// Linux 平台验证器
#[cfg(target_os = "linux")]
pub use keyboard::LinuxKeyboardVerifier;
#[cfg(target_os = "linux")]
pub use mouse::LinuxMouseVerifier;

// Windows 平台验证器
#[cfg(target_os = "windows")]
pub use keyboard::WindowsKeyboardVerifier;
#[cfg(target_os = "windows")]
pub use mouse::WindowsMouseVerifier;

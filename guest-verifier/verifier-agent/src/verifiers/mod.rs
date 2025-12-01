//! 验证器实现模块

pub mod keyboard;
pub mod mouse;
pub mod command;

// 导出命令验证器（跨平台）
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

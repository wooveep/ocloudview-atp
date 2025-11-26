//! 验证器实现模块

pub mod keyboard;
pub mod mouse;
pub mod command;

pub use keyboard::LinuxKeyboardVerifier;
pub use mouse::LinuxMouseVerifier;
pub use command::CommandVerifier;

#[cfg(target_os = "windows")]
pub use keyboard::WindowsKeyboardVerifier;
#[cfg(target_os = "windows")]
pub use mouse::WindowsMouseVerifier;

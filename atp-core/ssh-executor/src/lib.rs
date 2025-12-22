//! ATP SSH 执行器
//!
//! 提供 SSH 远程命令执行能力，支持：
//! - 密码认证
//! - SSH 密钥认证
//! - 命令执行和输出捕获
//! - 连接池管理（可选）
//!
//! # 示例
//!
//! ```ignore
//! use atp_ssh_executor::{SshClient, SshConfig, AuthMethod};
//!
//! // 使用密码认证
//! let config = SshConfig::with_password("192.168.1.100", "root", "password");
//! let client = SshClient::connect(config).await?;
//! let output = client.execute("ls -la").await?;
//! println!("{}", output.stdout);
//!
//! // 使用密钥认证
//! let config = SshConfig::with_key("192.168.1.100", "root", "~/.ssh/id_rsa");
//! let client = SshClient::connect(config).await?;
//! let output = client.execute("hostname").await?;
//! ```

mod client;
mod config;
mod error;

pub use client::SshClient;
pub use config::{AuthMethod, SshConfig};
pub use error::{SshError, Result};

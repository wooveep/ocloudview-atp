//! ATP Gluster 工具库
//!
//! 提供 Gluster 存储文件定位能力，支持：
//! - 通过 getfattr 查询文件的实际 brick 位置
//! - 解析 Gluster pathinfo 信息
//! - 查询 Gluster 卷信息
//!
//! # 示例
//!
//! ```ignore
//! use atp_gluster::{GlusterClient, GlusterFileLocation};
//! use atp_ssh_executor::{SshClient, SshConfig};
//!
//! // 创建 SSH 客户端
//! let ssh = SshClient::connect(SshConfig::with_default_key("gluster-node", "root")).await?;
//!
//! // 创建 Gluster 客户端
//! let gluster = GlusterClient::new(ssh);
//!
//! // 查询文件位置
//! let location = gluster.get_file_location("/mnt/gluster/volume/disk.qcow2").await?;
//! for replica in &location.replicas {
//!     println!("主机: {}, Brick: {}", replica.host, replica.brick_path);
//! }
//! ```

mod client;
mod error;
mod models;
mod pathinfo;

pub use client::GlusterClient;
pub use error::{GlusterError, Result};
pub use models::{GlusterBrick, GlusterFileLocation, GlusterReplica, GlusterVolume};
pub use pathinfo::parse_pathinfo;

//! ATP Gluster 工具库
//!
//! 提供 Gluster 存储文件定位能力，支持：
//! - 通过 getfattr 查询文件的实际 brick 位置
//! - 解析 Gluster pathinfo 信息
//! - 查询 Gluster 卷信息
//! - **脑裂检测与修复**
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
//!
//! // 检查脑裂状态
//! let split_brain = gluster.check_split_brain("gv0").await?;
//! if split_brain.has_split_brain() {
//!     println!("发现 {} 个脑裂文件", split_brain.entry_count());
//! }
//! ```

mod client;
mod error;
mod models;
mod parser;
mod pathinfo;

pub use client::GlusterClient;
pub use error::{GlusterError, Result};
pub use models::{
    BrickLocation, FileStat, GlusterBrick, GlusterFileLocation, GlusterReplica, GlusterVolume,
    SplitBrainEntry, SplitBrainEntryType, SplitBrainInfo,
};
pub use parser::{parse_afr_attributes, parse_split_brain_count, parse_split_brain_info, parse_stat_output};
pub use pathinfo::parse_pathinfo;

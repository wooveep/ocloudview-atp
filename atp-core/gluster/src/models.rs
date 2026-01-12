//! Gluster 数据模型

use serde::{Deserialize, Serialize};

/// Gluster Brick 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlusterBrick {
    /// 主机名/IP
    pub host: String,
    /// Brick 路径
    pub brick_path: String,
    /// 状态（Online/Offline）
    #[serde(default)]
    pub status: Option<String>,
}

impl GlusterBrick {
    /// 创建新的 Brick 信息
    pub fn new(host: impl Into<String>, brick_path: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            brick_path: brick_path.into(),
            status: None,
        }
    }

    /// 获取完整的 brick 标识（host:path 格式）
    pub fn full_path(&self) -> String {
        format!("{}:{}", self.host, self.brick_path)
    }
}

/// Gluster 卷信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlusterVolume {
    /// 卷名
    pub name: String,
    /// 卷类型（Replicate/Distributed/Distributed-Replicate 等）
    pub volume_type: String,
    /// 副本数
    #[serde(default)]
    pub replica_count: u32,
    /// 分布数
    #[serde(default)]
    pub disperse_count: u32,
    /// 状态
    pub status: String,
    /// Brick 列表
    pub bricks: Vec<GlusterBrick>,
    /// 挂载点路径（如果已知）
    #[serde(default)]
    pub mount_point: Option<String>,
}

impl GlusterVolume {
    /// 创建新的卷信息
    pub fn new(name: impl Into<String>, volume_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            volume_type: volume_type.into(),
            replica_count: 0,
            disperse_count: 0,
            status: "Unknown".to_string(),
            bricks: Vec::new(),
            mount_point: None,
        }
    }

    /// 是否为副本卷
    pub fn is_replicated(&self) -> bool {
        self.replica_count > 1 || self.volume_type.to_lowercase().contains("replicate")
    }

    /// 是否为分布卷
    pub fn is_distributed(&self) -> bool {
        self.volume_type.to_lowercase().contains("distribute")
    }
}

/// Gluster 副本信息（文件实际存储位置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlusterReplica {
    /// 主机名或 IP
    pub host: String,
    /// Brick 根路径
    pub brick_path: String,
    /// 文件在 brick 中的完整路径
    pub file_path: String,
}

impl GlusterReplica {
    /// 创建新的副本信息
    pub fn new(
        host: impl Into<String>,
        brick_path: impl Into<String>,
        file_path: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            brick_path: brick_path.into(),
            file_path: file_path.into(),
        }
    }

    /// 获取完整的文件位置（host:file_path 格式）
    pub fn full_location(&self) -> String {
        format!("{}:{}", self.host, self.file_path)
    }
}

/// Gluster 文件位置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlusterFileLocation {
    /// 文件逻辑路径（挂载点路径）
    pub logical_path: String,
    /// 卷名（如果能解析到）
    #[serde(default)]
    pub volume_name: Option<String>,
    /// 副本列表（文件实际存储在这些位置）
    pub replicas: Vec<GlusterReplica>,
}

impl GlusterFileLocation {
    /// 创建新的文件位置信息
    pub fn new(logical_path: impl Into<String>) -> Self {
        Self {
            logical_path: logical_path.into(),
            volume_name: None,
            replicas: Vec::new(),
        }
    }

    /// 添加副本
    pub fn add_replica(&mut self, replica: GlusterReplica) {
        self.replicas.push(replica);
    }

    /// 获取副本数量
    pub fn replica_count(&self) -> usize {
        self.replicas.len()
    }

    /// 是否有副本信息
    pub fn has_replicas(&self) -> bool {
        !self.replicas.is_empty()
    }

    /// 获取所有涉及的主机
    pub fn hosts(&self) -> Vec<&str> {
        self.replicas.iter().map(|r| r.host.as_str()).collect()
    }
}

// ============================================
// 脑裂修复相关数据结构
// ============================================

/// 脑裂条目类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitBrainEntryType {
    /// 普通文件（以 / 开头的路径）
    RegularFile,
    /// 分片文件（格式为 <gfid:UUID>.N）
    ShardFile,
}

impl std::fmt::Display for SplitBrainEntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SplitBrainEntryType::RegularFile => write!(f, "普通文件"),
            SplitBrainEntryType::ShardFile => write!(f, "分片文件"),
        }
    }
}

/// Brick 位置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrickLocation {
    /// 主机名或 IP
    pub host: String,
    /// Brick 根路径
    pub brick_path: String,
    /// 文件在 Brick 中的完整路径
    pub full_path: String,
}

impl BrickLocation {
    /// 创建新的 Brick 位置信息
    pub fn new(
        host: impl Into<String>,
        brick_path: impl Into<String>,
        full_path: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            brick_path: brick_path.into(),
            full_path: full_path.into(),
        }
    }

    /// 获取完整位置标识（host:full_path 格式）
    pub fn full_location(&self) -> String {
        format!("{}:{}", self.host, self.full_path)
    }
}

/// 脑裂条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitBrainEntry {
    /// 条目类型
    pub entry_type: SplitBrainEntryType,
    /// 原始路径或 GFID 标识
    /// - 普通文件: /images/vm-disk.qcow2
    /// - 分片文件: <gfid:307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b>.15
    pub path: String,
    /// 解析后的实际文件路径（对于分片文件，是映射后的原始文件路径）
    #[serde(default)]
    pub resolved_path: Option<String>,
    /// 分片序号（仅分片文件有效）
    #[serde(default)]
    pub shard_index: Option<u32>,
    /// GFID（仅分片文件有效）
    #[serde(default)]
    pub gfid: Option<String>,
    /// 涉及的 Brick 位置列表
    pub brick_locations: Vec<BrickLocation>,
}

impl SplitBrainEntry {
    /// 创建普通文件条目
    pub fn regular_file(path: impl Into<String>, brick_locations: Vec<BrickLocation>) -> Self {
        Self {
            entry_type: SplitBrainEntryType::RegularFile,
            path: path.into(),
            resolved_path: None,
            shard_index: None,
            gfid: None,
            brick_locations,
        }
    }

    /// 创建分片文件条目
    pub fn shard_file(
        path: impl Into<String>,
        gfid: impl Into<String>,
        shard_index: u32,
        brick_locations: Vec<BrickLocation>,
    ) -> Self {
        Self {
            entry_type: SplitBrainEntryType::ShardFile,
            path: path.into(),
            resolved_path: None,
            shard_index: Some(shard_index),
            gfid: Some(gfid.into()),
            brick_locations,
        }
    }

    /// 是否为分片文件
    pub fn is_shard(&self) -> bool {
        self.entry_type == SplitBrainEntryType::ShardFile
    }

    /// 获取有效文件路径（优先返回解析后的路径）
    pub fn effective_path(&self) -> &str {
        self.resolved_path.as_deref().unwrap_or(&self.path)
    }

    /// 设置解析后的路径
    pub fn set_resolved_path(&mut self, path: impl Into<String>) {
        self.resolved_path = Some(path.into());
    }
}

/// 脑裂信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitBrainInfo {
    /// Gluster 卷名
    pub volume_name: String,
    /// 脑裂条目列表（已去重）
    pub entries: Vec<SplitBrainEntry>,
    /// 原始条目总数（去重前）
    pub raw_count: usize,
}

impl SplitBrainInfo {
    /// 创建新的脑裂信息
    pub fn new(volume_name: impl Into<String>) -> Self {
        Self {
            volume_name: volume_name.into(),
            entries: Vec::new(),
            raw_count: 0,
        }
    }

    /// 添加条目
    pub fn add_entry(&mut self, entry: SplitBrainEntry) {
        self.entries.push(entry);
    }

    /// 获取去重后的条目数量
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// 是否有脑裂文件
    pub fn has_split_brain(&self) -> bool {
        !self.entries.is_empty()
    }

    /// 获取普通文件条目
    pub fn regular_files(&self) -> Vec<&SplitBrainEntry> {
        self.entries
            .iter()
            .filter(|e| e.entry_type == SplitBrainEntryType::RegularFile)
            .collect()
    }

    /// 获取分片文件条目
    pub fn shard_files(&self) -> Vec<&SplitBrainEntry> {
        self.entries
            .iter()
            .filter(|e| e.entry_type == SplitBrainEntryType::ShardFile)
            .collect()
    }

    /// 按 GFID 分组的分片文件（同一文件的所有分片）
    pub fn shards_by_gfid(&self) -> std::collections::HashMap<&str, Vec<&SplitBrainEntry>> {
        let mut map = std::collections::HashMap::new();
        for entry in &self.entries {
            if let Some(ref gfid) = entry.gfid {
                map.entry(gfid.as_str()).or_insert_with(Vec::new).push(entry);
            }
        }
        map
    }
}

/// 文件统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStat {
    /// 文件大小（字节）
    pub size: u64,
    /// 修改时间
    pub mtime: String,
    /// 访问时间
    #[serde(default)]
    pub atime: Option<String>,
    /// 权限模式
    #[serde(default)]
    pub mode: Option<String>,
    /// 所有者
    #[serde(default)]
    pub owner: Option<String>,
    /// 所属组
    #[serde(default)]
    pub group: Option<String>,
}

impl FileStat {
    /// 创建新的文件统计信息
    pub fn new(size: u64, mtime: impl Into<String>) -> Self {
        Self {
            size,
            mtime: mtime.into(),
            atime: None,
            mode: None,
            owner: None,
            group: None,
        }
    }

    /// 获取人类可读的文件大小
    pub fn size_human(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if self.size >= TB {
            format!("{:.2} TB", self.size as f64 / TB as f64)
        } else if self.size >= GB {
            format!("{:.2} GB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.2} MB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.2} KB", self.size as f64 / KB as f64)
        } else {
            format!("{} B", self.size)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gluster_brick() {
        let brick = GlusterBrick::new("node1", "/data/brick1");
        assert_eq!(brick.full_path(), "node1:/data/brick1");
    }

    #[test]
    fn test_gluster_replica() {
        let replica = GlusterReplica::new(
            "node1",
            "/data/brick1",
            "/data/brick1/volumes/vol1/file.qcow2",
        );
        assert_eq!(
            replica.full_location(),
            "node1:/data/brick1/volumes/vol1/file.qcow2"
        );
    }

    #[test]
    fn test_gluster_file_location() {
        let mut location = GlusterFileLocation::new("/mnt/gluster/file.qcow2");
        location.add_replica(GlusterReplica::new("node1", "/brick1", "/brick1/file.qcow2"));
        location.add_replica(GlusterReplica::new("node2", "/brick2", "/brick2/file.qcow2"));

        assert_eq!(location.replica_count(), 2);
        assert!(location.has_replicas());
        assert_eq!(location.hosts(), vec!["node1", "node2"]);
    }

    #[test]
    fn test_gluster_volume() {
        let vol = GlusterVolume {
            name: "vol1".to_string(),
            volume_type: "Distributed-Replicate".to_string(),
            replica_count: 2,
            disperse_count: 0,
            status: "Started".to_string(),
            bricks: vec![],
            mount_point: Some("/mnt/gluster".to_string()),
        };

        assert!(vol.is_replicated());
        assert!(vol.is_distributed());
    }

    #[test]
    fn test_split_brain_entry_regular() {
        let locations = vec![
            BrickLocation::new("node1", "/data/brick1", "/data/brick1/images/disk.qcow2"),
            BrickLocation::new("node2", "/data/brick1", "/data/brick1/images/disk.qcow2"),
        ];
        let entry = SplitBrainEntry::regular_file("/images/disk.qcow2", locations);

        assert_eq!(entry.entry_type, SplitBrainEntryType::RegularFile);
        assert!(!entry.is_shard());
        assert_eq!(entry.effective_path(), "/images/disk.qcow2");
        assert_eq!(entry.brick_locations.len(), 2);
    }

    #[test]
    fn test_split_brain_entry_shard() {
        let locations = vec![
            BrickLocation::new("node1", "/data/brick1", "/data/brick1/.shard/xxx.15"),
        ];
        let entry = SplitBrainEntry::shard_file(
            "<gfid:307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b>.15",
            "307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b",
            15,
            locations,
        );

        assert_eq!(entry.entry_type, SplitBrainEntryType::ShardFile);
        assert!(entry.is_shard());
        assert_eq!(entry.shard_index, Some(15));
        assert_eq!(entry.gfid.as_deref(), Some("307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b"));
    }

    #[test]
    fn test_split_brain_info() {
        let mut info = SplitBrainInfo::new("gv0");

        let entry1 = SplitBrainEntry::regular_file("/images/disk1.qcow2", vec![]);
        let entry2 = SplitBrainEntry::shard_file(
            "<gfid:xxx>.1",
            "xxx",
            1,
            vec![],
        );

        info.add_entry(entry1);
        info.add_entry(entry2);
        info.raw_count = 4;

        assert!(info.has_split_brain());
        assert_eq!(info.entry_count(), 2);
        assert_eq!(info.regular_files().len(), 1);
        assert_eq!(info.shard_files().len(), 1);
    }

    #[test]
    fn test_file_stat_size_human() {
        assert_eq!(FileStat::new(500, "").size_human(), "500 B");
        assert_eq!(FileStat::new(1024, "").size_human(), "1.00 KB");
        assert_eq!(FileStat::new(1024 * 1024, "").size_human(), "1.00 MB");
        assert_eq!(FileStat::new(1024 * 1024 * 1024, "").size_human(), "1.00 GB");
        assert_eq!(FileStat::new(1024 * 1024 * 1024 * 1024, "").size_human(), "1.00 TB");
    }
}

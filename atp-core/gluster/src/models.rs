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
}

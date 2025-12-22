//! Gluster 客户端

use atp_ssh_executor::SshClient;
use tracing::{debug, info};

use crate::error::{GlusterError, Result};
use crate::models::{GlusterBrick, GlusterFileLocation, GlusterVolume};
use crate::pathinfo::{
    is_file_not_found, is_getfattr_not_found, is_not_gluster_fs, parse_pathinfo,
};

/// Gluster 客户端
///
/// 通过 SSH 连接到 Gluster 节点执行查询命令
pub struct GlusterClient {
    ssh: SshClient,
}

impl GlusterClient {
    /// 创建新的 Gluster 客户端
    ///
    /// # Arguments
    /// * `ssh` - 已连接的 SSH 客户端
    pub fn new(ssh: SshClient) -> Self {
        Self { ssh }
    }

    /// 获取文件在 Gluster 中的实际存储位置
    ///
    /// 使用 getfattr 查询 trusted.glusterfs.pathinfo 属性
    ///
    /// # Arguments
    /// * `file_path` - 文件在 Gluster 挂载点中的路径
    ///
    /// # Returns
    /// 文件的实际存储位置信息
    ///
    /// # Example
    /// ```ignore
    /// let location = client.get_file_location("/mnt/gluster/disk.qcow2").await?;
    /// for replica in &location.replicas {
    ///     println!("{}:{}", replica.host, replica.file_path);
    /// }
    /// ```
    pub async fn get_file_location(&self, file_path: &str) -> Result<GlusterFileLocation> {
        info!("查询文件位置: {}", file_path);

        // 执行 getfattr 命令
        let cmd = format!(
            "getfattr -n trusted.glusterfs.pathinfo -e text '{}'",
            file_path
        );

        let output = self.ssh.execute(&cmd).await?;

        // 检查错误情况
        if is_getfattr_not_found(&output.stderr) || is_getfattr_not_found(&output.stdout) {
            return Err(GlusterError::GetfattrNotFound);
        }

        if is_file_not_found(&output.stderr) || is_file_not_found(&output.stdout) {
            return Err(GlusterError::FileNotFound(file_path.to_string()));
        }

        if is_not_gluster_fs(&output.stderr) || is_not_gluster_fs(&output.stdout) {
            return Err(GlusterError::NotGlusterFs(file_path.to_string()));
        }

        if !output.is_success() {
            return Err(GlusterError::CommandError(format!(
                "getfattr 执行失败 (退出码 {:?}): {}",
                output.exit_code,
                output.combined_output()
            )));
        }

        // 解析 pathinfo
        let location = parse_pathinfo(&output.stdout)?;

        info!(
            "文件 {} 有 {} 个副本",
            file_path,
            location.replica_count()
        );

        Ok(location)
    }

    /// 批量获取文件位置
    ///
    /// # Arguments
    /// * `file_paths` - 文件路径列表
    ///
    /// # Returns
    /// 每个文件的位置信息（成功或失败）
    pub async fn get_files_location(
        &self,
        file_paths: &[&str],
    ) -> Vec<(String, Result<GlusterFileLocation>)> {
        let mut results = Vec::new();

        for path in file_paths {
            let result = self.get_file_location(path).await;
            results.push((path.to_string(), result));
        }

        results
    }

    /// 获取 Gluster 卷列表
    ///
    /// 执行 `gluster volume list` 命令
    pub async fn list_volumes(&self) -> Result<Vec<String>> {
        info!("获取 Gluster 卷列表");

        let output = self.ssh.execute("gluster volume list").await?;

        if !output.is_success() {
            if output.stderr.contains("command not found") {
                return Err(GlusterError::GlusterNotRunning);
            }
            return Err(GlusterError::CommandError(format!(
                "gluster volume list 失败: {}",
                output.combined_output()
            )));
        }

        let volumes: Vec<String> = output
            .stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with("No volumes"))
            .collect();

        info!("找到 {} 个卷", volumes.len());
        Ok(volumes)
    }

    /// 获取 Gluster 卷详细信息
    ///
    /// 执行 `gluster volume info <volume_name>` 命令
    pub async fn get_volume_info(&self, volume_name: &str) -> Result<GlusterVolume> {
        info!("获取卷信息: {}", volume_name);

        let cmd = format!("gluster volume info {}", volume_name);
        let output = self.ssh.execute(&cmd).await?;

        if !output.is_success() {
            return Err(GlusterError::CommandError(format!(
                "获取卷信息失败: {}",
                output.combined_output()
            )));
        }

        parse_volume_info(&output.stdout, volume_name)
    }

    /// 检查 Gluster 服务状态
    pub async fn check_gluster_status(&self) -> Result<bool> {
        let output = self.ssh.execute("gluster peer status 2>/dev/null").await?;
        Ok(output.is_success())
    }

    /// 检查 getfattr 是否可用
    pub async fn check_getfattr(&self) -> Result<bool> {
        let output = self.ssh.execute("which getfattr 2>/dev/null").await?;
        Ok(output.is_success() && !output.stdout.is_empty())
    }

    /// 获取内部 SSH 客户端的引用
    pub fn ssh(&self) -> &SshClient {
        &self.ssh
    }

    /// 消费自身并返回 SSH 客户端
    pub fn into_ssh(self) -> SshClient {
        self.ssh
    }
}

/// 解析 `gluster volume info` 输出
fn parse_volume_info(output: &str, volume_name: &str) -> Result<GlusterVolume> {
    let mut volume = GlusterVolume::new(volume_name, "Unknown");
    let _current_brick_host = String::new();

    for line in output.lines() {
        let line = line.trim();

        if line.starts_with("Type:") {
            volume.volume_type = line.trim_start_matches("Type:").trim().to_string();
        } else if line.starts_with("Status:") {
            volume.status = line.trim_start_matches("Status:").trim().to_string();
        } else if line.starts_with("Number of Bricks:") {
            // 解析 "Number of Bricks: 1 x 2 = 2" 格式
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(pos) = parts.iter().position(|&p| p == "x") {
                if let Some(replica_str) = parts.get(pos + 1) {
                    volume.replica_count = replica_str.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("Brick") && line.contains(':') {
            // 解析 "Brick1: node1:/data/brick1" 格式
            if let Some(brick_info) = line.split_once(':') {
                let brick_path = brick_info.1.trim();
                if let Some((host, path)) = brick_path.split_once(':') {
                    volume.bricks.push(GlusterBrick::new(host, path));
                }
            }
        }
    }

    debug!(
        "卷 {} 解析完成: 类型={}, 状态={}, Brick数={}",
        volume_name,
        volume.volume_type,
        volume.status,
        volume.bricks.len()
    );

    Ok(volume)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_volume_info() {
        let output = r#"
Volume Name: gv0
Type: Replicate
Volume ID: 12345678-1234-1234-1234-123456789012
Status: Started
Number of Bricks: 1 x 2 = 2
Transport-type: tcp
Bricks:
Brick1: node1:/data/brick1
Brick2: node2:/data/brick2
Options Reconfigured:
transport.address-family: inet
"#;

        let volume = parse_volume_info(output, "gv0").unwrap();

        assert_eq!(volume.name, "gv0");
        assert_eq!(volume.volume_type, "Replicate");
        assert_eq!(volume.status, "Started");
        assert_eq!(volume.replica_count, 2);
        assert_eq!(volume.bricks.len(), 2);
        assert_eq!(volume.bricks[0].host, "node1");
        assert_eq!(volume.bricks[0].brick_path, "/data/brick1");
        assert_eq!(volume.bricks[1].host, "node2");
    }
}

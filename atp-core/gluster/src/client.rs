//! Gluster 客户端

use atp_ssh_executor::SshClient;
use tracing::{debug, info, warn};

use crate::error::{GlusterError, Result};
use crate::models::{FileStat, GlusterBrick, GlusterFileLocation, GlusterVolume, SplitBrainInfo};
use crate::parser::{parse_afr_attributes, parse_split_brain_count, parse_split_brain_info, parse_stat_output};
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

    // ============================================
    // 脑裂检测与修复
    // ============================================

    /// 检查卷的脑裂状态
    ///
    /// 执行 `gluster volume heal <vol> info split-brain` 命令
    ///
    /// # Arguments
    /// * `volume_name` - Gluster 卷名
    ///
    /// # Returns
    /// 脑裂信息，包含所有处于脑裂状态的文件列表
    ///
    /// # Example
    /// ```ignore
    /// let info = client.check_split_brain("gv0").await?;
    /// if info.has_split_brain() {
    ///     println!("发现 {} 个脑裂文件", info.entry_count());
    ///     for entry in &info.entries {
    ///         println!("  - {} ({})", entry.path, entry.entry_type);
    ///     }
    /// }
    /// ```
    pub async fn check_split_brain(&self, volume_name: &str) -> Result<SplitBrainInfo> {
        info!("检查卷 {} 的脑裂状态", volume_name);

        let cmd = format!("gluster volume heal {} info split-brain", volume_name);
        let output = self.ssh.execute(&cmd).await?;

        if !output.is_success() {
            if output.stderr.contains("command not found") {
                return Err(GlusterError::GlusterNotRunning);
            }
            if output.stderr.contains("does not exist") || output.stdout.contains("does not exist")
            {
                return Err(GlusterError::VolumeNotFound(volume_name.to_string()));
            }
            return Err(GlusterError::CommandError(format!(
                "获取脑裂信息失败: {}",
                output.combined_output()
            )));
        }

        let info = parse_split_brain_info(&output.stdout, volume_name)?;

        info!(
            "卷 {} 脑裂状态: {} 个条目 (去重前 {})",
            volume_name,
            info.entry_count(),
            info.raw_count
        );

        Ok(info)
    }

    /// 获取文件的 AFR 扩展属性
    ///
    /// 执行 `getfattr -d -m . -e hex <file_path>` 命令
    ///
    /// # Arguments
    /// * `file_path` - Brick 上的文件完整路径
    ///
    /// # Returns
    /// 所有 trusted.afr.* 属性名列表
    pub async fn get_afr_attributes(&self, file_path: &str) -> Result<Vec<String>> {
        info!("获取文件 AFR 属性: {}", file_path);

        let cmd = format!("getfattr -d -m . -e hex '{}'", file_path);
        let output = self.ssh.execute(&cmd).await?;

        if is_getfattr_not_found(&output.stderr) || is_getfattr_not_found(&output.stdout) {
            return Err(GlusterError::GetfattrNotFound);
        }

        if is_file_not_found(&output.stderr) || is_file_not_found(&output.stdout) {
            return Err(GlusterError::FileNotFound(file_path.to_string()));
        }

        let attrs = parse_afr_attributes(&output.stdout);
        info!("文件 {} 有 {} 个 AFR 属性", file_path, attrs.len());

        Ok(attrs)
    }

    /// 删除指定的 AFR 扩展属性
    ///
    /// 执行 `setfattr -x <attr_name> <file_path>` 命令
    ///
    /// # Arguments
    /// * `file_path` - Brick 上的文件完整路径
    /// * `attr_name` - 要删除的属性名（如 trusted.afr.gv0-client-0）
    pub async fn remove_afr_attribute(&self, file_path: &str, attr_name: &str) -> Result<()> {
        info!("删除属性 {} 从文件 {}", attr_name, file_path);

        let cmd = format!("setfattr -x '{}' '{}'", attr_name, file_path);
        let output = self.ssh.execute(&cmd).await?;

        if !output.is_success() {
            if output.stderr.contains("command not found") {
                return Err(GlusterError::CommandError(
                    "setfattr 命令不可用".to_string(),
                ));
            }
            return Err(GlusterError::CommandError(format!(
                "删除属性失败: {}",
                output.combined_output()
            )));
        }

        Ok(())
    }

    /// 删除文件的所有 AFR 扩展属性
    ///
    /// # Arguments
    /// * `file_path` - Brick 上的文件完整路径
    ///
    /// # Returns
    /// 删除的属性数量
    pub async fn remove_all_afr_attributes(&self, file_path: &str) -> Result<usize> {
        let attrs = self.get_afr_attributes(file_path).await?;
        let count = attrs.len();

        for attr in attrs {
            if let Err(e) = self.remove_afr_attribute(file_path, &attr).await {
                warn!("删除属性 {} 失败: {}", attr, e);
            }
        }

        info!("已删除文件 {} 的 {} 个 AFR 属性", file_path, count);
        Ok(count)
    }

    /// 触发卷修复
    ///
    /// 执行 `gluster volume heal <vol>` 命令
    ///
    /// # Arguments
    /// * `volume_name` - Gluster 卷名
    pub async fn trigger_heal(&self, volume_name: &str) -> Result<()> {
        info!("触发卷 {} 修复", volume_name);

        let cmd = format!("gluster volume heal {}", volume_name);
        let output = self.ssh.execute(&cmd).await?;

        if !output.is_success() {
            return Err(GlusterError::CommandError(format!(
                "触发修复失败: {}",
                output.combined_output()
            )));
        }

        info!("卷 {} 修复已触发", volume_name);
        Ok(())
    }

    /// 获取脑裂条目数量
    ///
    /// 快速检查脑裂是否已修复
    ///
    /// # Arguments
    /// * `volume_name` - Gluster 卷名
    ///
    /// # Returns
    /// 脑裂条目总数（0 表示已修复）
    pub async fn get_split_brain_count(&self, volume_name: &str) -> Result<usize> {
        let cmd = format!("gluster volume heal {} info split-brain", volume_name);
        let output = self.ssh.execute(&cmd).await?;

        if !output.is_success() {
            return Err(GlusterError::CommandError(format!(
                "获取脑裂数量失败: {}",
                output.combined_output()
            )));
        }

        Ok(parse_split_brain_count(&output.stdout))
    }

    /// 等待脑裂修复完成
    ///
    /// 循环检测直到脑裂条目数量为 0 或超时
    ///
    /// # Arguments
    /// * `volume_name` - Gluster 卷名
    /// * `max_retries` - 最大重试次数
    /// * `interval_secs` - 检测间隔（秒）
    ///
    /// # Returns
    /// 修复成功返回 Ok(true)，超时返回 Ok(false)
    pub async fn wait_for_heal(
        &self,
        volume_name: &str,
        max_retries: u32,
        interval_secs: u64,
    ) -> Result<bool> {
        info!(
            "等待卷 {} 修复完成 (最多 {} 次, 间隔 {}s)",
            volume_name, max_retries, interval_secs
        );

        for i in 1..=max_retries {
            let count = self.get_split_brain_count(volume_name).await?;
            info!("第 {}/{} 次检测: 脑裂条目数 = {}", i, max_retries, count);

            if count == 0 {
                return Ok(true);
            }

            if i < max_retries {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
            }
        }

        warn!("卷 {} 修复超时", volume_name);
        Ok(false)
    }

    /// 通过 GFID 查找原始文件路径
    ///
    /// 用于分片场景，将 GFID 映射到实际文件
    ///
    /// # Arguments
    /// * `brick_path` - Brick 根路径（如 /data/brick1）
    /// * `gfid` - 文件的 GFID（如 307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b）
    ///
    /// # Returns
    /// 原始文件的完整路径
    pub async fn resolve_gfid_to_file(&self, brick_path: &str, gfid: &str) -> Result<String> {
        info!("解析 GFID {} 到文件路径 (brick: {})", gfid, brick_path);

        // 构造 .glusterfs 中的硬链接路径
        // Path = <Brick_Path>/.glusterfs/<UUID前2位>/<UUID第3-4位>/<UUID>
        if gfid.len() < 4 {
            return Err(GlusterError::ParseError("GFID 格式无效".to_string()));
        }

        let gfid_path = format!(
            "{}/.glusterfs/{}/{}/{}",
            brick_path,
            &gfid[0..2],
            &gfid[2..4],
            gfid
        );

        // 使用 find -samefile 查找同 inode 的文件
        let cmd = format!(
            "find '{}' -samefile '{}' ! -path '*/.glusterfs/*' ! -path '*/.shard/*' -print -quit 2>/dev/null",
            brick_path, gfid_path
        );
        let output = self.ssh.execute(&cmd).await?;

        let file_path = output.stdout.trim();
        if file_path.is_empty() {
            // 尝试备用方法：直接读取 gfid 链接
            let cmd2 = format!(
                "stat -c '%N' '{}' 2>/dev/null | grep -oP \"'[^']+' -> '\\K[^']+\"",
                gfid_path
            );
            let output2 = self.ssh.execute(&cmd2).await?;
            let file_path2 = output2.stdout.trim();

            if file_path2.is_empty() {
                return Err(GlusterError::FileNotFound(format!(
                    "无法解析 GFID {} 对应的文件",
                    gfid
                )));
            }

            info!("GFID {} 解析为文件: {}", gfid, file_path2);
            return Ok(file_path2.to_string());
        }

        info!("GFID {} 解析为文件: {}", gfid, file_path);
        Ok(file_path.to_string())
    }

    /// 获取文件统计信息
    ///
    /// 执行 `stat` 命令获取文件大小、修改时间等信息
    ///
    /// # Arguments
    /// * `file_path` - 文件路径
    ///
    /// # Returns
    /// 文件统计信息
    pub async fn get_file_stat(&self, file_path: &str) -> Result<FileStat> {
        info!("获取文件统计信息: {}", file_path);

        let cmd = format!("stat '{}'", file_path);
        let output = self.ssh.execute(&cmd).await?;

        if !output.is_success() {
            if is_file_not_found(&output.stderr) {
                return Err(GlusterError::FileNotFound(file_path.to_string()));
            }
            return Err(GlusterError::CommandError(format!(
                "获取文件信息失败: {}",
                output.combined_output()
            )));
        }

        parse_stat_output(&output.stdout)
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

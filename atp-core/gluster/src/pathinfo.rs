//! Gluster pathinfo 解析
//!
//! 解析 `getfattr -n trusted.glusterfs.pathinfo` 命令的输出
//!
//! 输出格式示例:
//! ```text
//! # file: /home/gluster3/b97923a3-dd90-4e24-82cf-74c25dca3ff5.qcow2
//! trusted.glusterfs.pathinfo="(<REPLICATE:gluster03-replicate-0> <POSIX(/home/brick1):node1:/home/brick1/b97923a3-dd90-4e24-82cf-74c25dca3ff5.qcow2> <POSIX(/home/brick2):node2:/home/brick2/b97923a3-dd90-4e24-82cf-74c25dca3ff5.qcow2>)"
//! ```

use regex::Regex;
use tracing::debug;

use crate::error::{GlusterError, Result};
use crate::models::{GlusterFileLocation, GlusterReplica};

/// 解析 getfattr pathinfo 输出
///
/// # Arguments
/// * `output` - getfattr 命令的完整输出
///
/// # Returns
/// 解析后的文件位置信息
///
/// # Example
/// ```ignore
/// let output = r#"
/// # file: /home/gluster3/disk.qcow2
/// trusted.glusterfs.pathinfo="(<REPLICATE:vol-rep-0> <POSIX(/brick1):node1:/brick1/disk.qcow2>)"
/// "#;
/// let location = parse_pathinfo(output)?;
/// ```
pub fn parse_pathinfo(output: &str) -> Result<GlusterFileLocation> {
    debug!("解析 pathinfo 输出: {} 字节", output.len());

    // 提取文件路径
    let logical_path = extract_file_path(output)?;
    let mut location = GlusterFileLocation::new(logical_path);

    // 提取卷名（如果有）
    location.volume_name = extract_volume_name(output);

    // 解析 POSIX 条目
    let replicas = extract_posix_entries(output)?;
    for replica in replicas {
        location.add_replica(replica);
    }

    if location.replicas.is_empty() {
        return Err(GlusterError::ParseError(
            "未找到 POSIX 存储位置信息".to_string(),
        ));
    }

    debug!(
        "解析完成: 文件 {}, 副本数 {}",
        location.logical_path,
        location.replica_count()
    );

    Ok(location)
}

/// 从输出中提取文件路径
fn extract_file_path(output: &str) -> Result<String> {
    // 匹配 "# file: /path/to/file" 格式
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("# file:") {
            let path = line.trim_start_matches("# file:").trim();
            if !path.is_empty() {
                return Ok(path.to_string());
            }
        }
    }

    // 尝试从 pathinfo 值中推断
    // 这种情况较少见，通常 # file: 行会存在
    Err(GlusterError::ParseError(
        "无法从输出中提取文件路径".to_string(),
    ))
}

/// 从输出中提取卷名
fn extract_volume_name(output: &str) -> Option<String> {
    // 匹配 <REPLICATE:volume-name-xxx> 或 <DISTRIBUTE:volume-name> 格式
    let re = Regex::new(r"<(REPLICATE|DISTRIBUTE|DISPERSE|STRIPE):([^>]+)>").ok()?;

    if let Some(caps) = re.captures(output) {
        let vol_info = caps.get(2)?.as_str();
        // 去掉可能的 "-replicate-0" 后缀
        let vol_name = vol_info
            .trim_end_matches(|c: char| c.is_ascii_digit() || c == '-')
            .trim_end_matches("-replicate")
            .trim_end_matches("-disperse")
            .trim_end_matches("-stripe");
        if !vol_name.is_empty() {
            return Some(vol_name.to_string());
        }
    }

    None
}

/// 从输出中提取 POSIX 条目
fn extract_posix_entries(output: &str) -> Result<Vec<GlusterReplica>> {
    // 匹配 <POSIX(brick_path):host:file_path> 格式
    // 例如: <POSIX(/home/brick1):node1:/home/brick1/xxx.qcow2>
    let re = Regex::new(r"<POSIX\(([^)]+)\):([^:]+):([^>]+)>")
        .map_err(|e| GlusterError::ParseError(format!("正则表达式错误: {}", e)))?;

    let mut replicas = Vec::new();

    for caps in re.captures_iter(output) {
        let brick_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let host = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let file_path = caps.get(3).map(|m| m.as_str()).unwrap_or("");

        if !host.is_empty() && !file_path.is_empty() {
            replicas.push(GlusterReplica::new(host, brick_path, file_path));
            debug!("找到副本: {} -> {}:{}", brick_path, host, file_path);
        }
    }

    Ok(replicas)
}

/// 检查输出是否表示文件不存在
pub fn is_file_not_found(output: &str) -> bool {
    let lower = output.to_lowercase();
    lower.contains("no such file") || lower.contains("不存在") || lower.contains("not found")
}

/// 检查输出是否表示不是 Gluster 文件系统
pub fn is_not_gluster_fs(output: &str) -> bool {
    let lower = output.to_lowercase();
    lower.contains("no data available")
        || lower.contains("operation not supported")
        || lower.contains("no such attribute")
}

/// 检查 getfattr 是否可用
pub fn is_getfattr_not_found(output: &str) -> bool {
    let lower = output.to_lowercase();
    lower.contains("command not found") || lower.contains("getfattr: 未找到命令")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pathinfo_replicate() {
        let output = r#"
# file: /home/gluster3/b97923a3-dd90-4e24-82cf-74c25dca3ff5.qcow2
trusted.glusterfs.pathinfo="(<REPLICATE:gluster03-replicate-0> <POSIX(/home/brick1):node1:/home/brick1/b97923a3-dd90-4e24-82cf-74c25dca3ff5.qcow2> <POSIX(/home/brick2):node2:/home/brick2/b97923a3-dd90-4e24-82cf-74c25dca3ff5.qcow2>)"
"#;

        let location = parse_pathinfo(output).unwrap();

        assert_eq!(
            location.logical_path,
            "/home/gluster3/b97923a3-dd90-4e24-82cf-74c25dca3ff5.qcow2"
        );
        assert_eq!(location.volume_name, Some("gluster03".to_string()));
        assert_eq!(location.replica_count(), 2);

        let r1 = &location.replicas[0];
        assert_eq!(r1.host, "node1");
        assert_eq!(r1.brick_path, "/home/brick1");
        assert_eq!(
            r1.file_path,
            "/home/brick1/b97923a3-dd90-4e24-82cf-74c25dca3ff5.qcow2"
        );

        let r2 = &location.replicas[1];
        assert_eq!(r2.host, "node2");
        assert_eq!(r2.brick_path, "/home/brick2");
    }

    #[test]
    fn test_parse_pathinfo_distribute() {
        let output = r#"
# file: /mnt/gv0/test.txt
trusted.glusterfs.pathinfo="(<DISTRIBUTE:gv0-dht> <POSIX(/data/brick):server1:/data/brick/test.txt>)"
"#;

        let location = parse_pathinfo(output).unwrap();

        assert_eq!(location.logical_path, "/mnt/gv0/test.txt");
        assert_eq!(location.replica_count(), 1);
        assert_eq!(location.replicas[0].host, "server1");
    }

    #[test]
    fn test_parse_pathinfo_multiple_replicas() {
        let output = r#"
# file: /gluster/vol/data.img
trusted.glusterfs.pathinfo="(<REPLICATE:vol-replicate-0> <POSIX(/brick1):n1:/brick1/data.img> <POSIX(/brick2):n2:/brick2/data.img> <POSIX(/brick3):n3:/brick3/data.img>)"
"#;

        let location = parse_pathinfo(output).unwrap();

        assert_eq!(location.replica_count(), 3);
        assert_eq!(location.hosts(), vec!["n1", "n2", "n3"]);
    }

    #[test]
    fn test_parse_pathinfo_empty() {
        let output = "";
        let result = parse_pathinfo(output);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_file_not_found() {
        assert!(is_file_not_found("getfattr: No such file or directory"));
        assert!(is_file_not_found("Error: 文件不存在"));
        assert!(!is_file_not_found("trusted.glusterfs.pathinfo=..."));
    }

    #[test]
    fn test_is_not_gluster_fs() {
        assert!(is_not_gluster_fs("getfattr: No data available"));
        assert!(is_not_gluster_fs(
            "trusted.glusterfs.pathinfo: No such attribute"
        ));
        assert!(!is_not_gluster_fs("trusted.glusterfs.pathinfo=..."));
    }

    #[test]
    fn test_extract_volume_name() {
        assert_eq!(
            extract_volume_name("<REPLICATE:gluster03-replicate-0>"),
            Some("gluster03".to_string())
        );
        assert_eq!(
            extract_volume_name("<DISTRIBUTE:myvolume>"),
            Some("myvolume".to_string())
        );
        assert_eq!(extract_volume_name("no volume info"), None);
    }
}

//! Gluster 脑裂解析模块
//!
//! 解析 `gluster volume heal info split-brain` 命令输出和 AFR 扩展属性

use regex::Regex;
use std::collections::{HashMap, HashSet};
use tracing::debug;

use crate::error::{GlusterError, Result};
use crate::models::{BrickLocation, FileStat, SplitBrainEntry, SplitBrainInfo};

/// 解析 `gluster volume heal <vol> info split-brain` 命令输出
///
/// # 输出格式示例
///
/// ## 场景一：普通分布式复制卷
/// ```text
/// Brick server1:/data/brick1
/// /images/vm-disk-1.qcow2
/// Status: Connected
/// Number of entries: 1
///
/// Brick server2:/data/brick1
/// /images/vm-disk-1.qcow2
/// Status: Connected
/// Number of entries: 1
/// ```
///
/// ## 场景二：分布式复制卷 + Sharding
/// ```text
/// Brick server1:/data/brick1
/// <gfid:307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b>.15
/// <gfid:307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b>.82
/// /images/vm-disk-1.qcow2
/// Status: Connected
/// Number of entries: 3
/// ```
pub fn parse_split_brain_info(output: &str, volume_name: &str) -> Result<SplitBrainInfo> {
    debug!(
        "解析脑裂信息: 卷 {}, 输出 {} 字节",
        volume_name,
        output.len()
    );

    let mut info = SplitBrainInfo::new(volume_name);

    // 按 Brick 分段解析
    let brick_sections = split_by_brick(output);
    debug!("找到 {} 个 Brick 分段", brick_sections.len());

    // 收集所有条目及其位置（用于去重）
    let mut entry_locations: HashMap<String, Vec<BrickLocation>> = HashMap::new();
    let mut raw_count = 0;

    for (brick_info, entries) in &brick_sections {
        let (host, brick_path) = parse_brick_info(brick_info)?;

        for entry in entries {
            raw_count += 1;
            let full_path = format!("{}{}", brick_path, entry);
            let location = BrickLocation::new(&host, &brick_path, &full_path);

            // 使用原始条目路径作为 key 进行去重
            entry_locations
                .entry(entry.clone())
                .or_insert_with(Vec::new)
                .push(location);
        }
    }

    info.raw_count = raw_count;

    // 去重后创建条目
    let mut seen = HashSet::new();
    for (entry_path, locations) in entry_locations {
        // 已经处理过的条目跳过
        if seen.contains(&entry_path) {
            continue;
        }
        seen.insert(entry_path.clone());

        let entry = if is_shard_entry(&entry_path) {
            // 分片文件
            if let Some((gfid, index)) = parse_shard_gfid(&entry_path) {
                SplitBrainEntry::shard_file(&entry_path, gfid, index, locations)
            } else {
                // 解析失败，当作普通文件处理
                SplitBrainEntry::regular_file(&entry_path, locations)
            }
        } else {
            // 普通文件
            SplitBrainEntry::regular_file(&entry_path, locations)
        };

        info.add_entry(entry);
    }

    debug!(
        "解析完成: 原始条目 {}, 去重后 {}",
        info.raw_count,
        info.entry_count()
    );

    Ok(info)
}

/// 按 Brick 分段输出
fn split_by_brick(output: &str) -> Vec<(String, Vec<String>)> {
    let mut sections = Vec::new();
    let mut current_brick: Option<String> = None;
    let mut current_entries = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        if line.starts_with("Brick ") {
            // 保存上一个 Brick 的内容
            if let Some(brick) = current_brick.take() {
                if !current_entries.is_empty() {
                    sections.push((brick, current_entries.clone()));
                    current_entries.clear();
                }
            }
            // 开始新的 Brick
            current_brick = Some(line.trim_start_matches("Brick ").to_string());
        } else if current_brick.is_some() {
            // 跳过状态行和空行
            if line.is_empty()
                || line.starts_with("Status:")
                || line.starts_with("Number of entries:")
                || line.starts_with("Number of entries in split-brain:")
            {
                continue;
            }

            // 这是一个文件条目
            if !line.is_empty() {
                current_entries.push(line.to_string());
            }
        }
    }

    // 保存最后一个 Brick
    if let Some(brick) = current_brick {
        if !current_entries.is_empty() {
            sections.push((brick, current_entries));
        }
    }

    sections
}

/// 解析 Brick 信息 "host:/path"
fn parse_brick_info(brick_info: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = brick_info.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(GlusterError::ParseError(format!(
            "无效的 Brick 格式: {}",
            brick_info
        )));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// 判断是否为分片条目
fn is_shard_entry(entry: &str) -> bool {
    entry.starts_with("<gfid:")
}

/// 解析分片 GFID 和索引
/// 格式: <gfid:307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b>.15
fn parse_shard_gfid(entry: &str) -> Option<(&str, u32)> {
    let re = Regex::new(r"^<gfid:([0-9a-f-]+)>\.(\d+)$").ok()?;
    let caps = re.captures(entry)?;

    let gfid = caps.get(1)?.as_str();
    let index = caps.get(2)?.as_str().parse().ok()?;

    Some((gfid, index))
}

/// 解析 `getfattr -d -m . -e hex` 输出，提取 trusted.afr.* 属性
///
/// # 输出格式示例
/// ```text
/// # file: /data/brick1/images/vm-disk.qcow2
/// trusted.afr.gv0-client-0=0x000000000000000200000000
/// trusted.afr.gv0-client-1=0x000000000000000000000000
/// trusted.glusterfs.pathinfo=...
/// ```
///
/// # Returns
/// 所有 trusted.afr.* 属性名列表
pub fn parse_afr_attributes(output: &str) -> Vec<String> {
    let mut attrs = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        // 跳过注释行和空行
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // 匹配 trusted.afr.xxx=... 格式
        if line.starts_with("trusted.afr.") {
            if let Some(eq_pos) = line.find('=') {
                let attr_name = &line[..eq_pos];
                attrs.push(attr_name.to_string());
                debug!("找到 AFR 属性: {}", attr_name);
            }
        }
    }

    attrs
}

/// 解析 `stat` 命令输出
///
/// # 输出格式示例
/// ```text
///   File: /data/brick1/images/vm-disk.qcow2
///   Size: 23068672000      Blocks: 45056000   IO Block: 4096   regular file
/// Access: (0644/-rw-r--r--)  Uid: (    0/    root)   Gid: (    0/    root)
/// Access: 2026-01-08 10:20:30.123456789 +0800
/// Modify: 2026-01-08 10:23:45.987654321 +0800
/// Change: 2026-01-08 10:23:45.987654321 +0800
///  Birth: -
/// ```
pub fn parse_stat_output(output: &str) -> Result<FileStat> {
    let mut size: Option<u64> = None;
    let mut mtime: Option<String> = None;
    let mut atime: Option<String> = None;
    let mut mode: Option<String> = None;
    let mut owner: Option<String> = None;
    let mut group: Option<String> = None;

    for line in output.lines() {
        let line = line.trim();

        // 解析文件大小
        if line.starts_with("Size:") {
            let size_re = Regex::new(r"Size:\s*(\d+)").ok();
            if let Some(re) = size_re {
                if let Some(caps) = re.captures(line) {
                    size = caps.get(1).and_then(|m| m.as_str().parse().ok());
                }
            }
        }

        // 解析权限和所有者
        if line.starts_with("Access:") && line.contains("Uid:") {
            // Access: (0644/-rw-r--r--)  Uid: (    0/    root)   Gid: (    0/    root)
            let mode_re = Regex::new(r"Access:\s*\(([^)]+)\)").ok();
            if let Some(re) = mode_re {
                if let Some(caps) = re.captures(line) {
                    mode = caps.get(1).map(|m| m.as_str().to_string());
                }
            }

            let uid_re = Regex::new(r"Uid:\s*\(\s*\d+/\s*([^)]+)\)").ok();
            if let Some(re) = uid_re {
                if let Some(caps) = re.captures(line) {
                    owner = caps.get(1).map(|m| m.as_str().trim().to_string());
                }
            }

            let gid_re = Regex::new(r"Gid:\s*\(\s*\d+/\s*([^)]+)\)").ok();
            if let Some(re) = gid_re {
                if let Some(caps) = re.captures(line) {
                    group = caps.get(1).map(|m| m.as_str().trim().to_string());
                }
            }
        }

        // 解析访问时间
        if line.starts_with("Access:") && !line.contains("Uid:") {
            // Access: 2026-01-08 10:20:30.123456789 +0800
            let time = line.trim_start_matches("Access:").trim();
            if !time.is_empty() {
                atime = Some(format_time(time));
            }
        }

        // 解析修改时间
        if line.starts_with("Modify:") {
            let time = line.trim_start_matches("Modify:").trim();
            if !time.is_empty() {
                mtime = Some(format_time(time));
            }
        }
    }

    let size = size.ok_or_else(|| GlusterError::ParseError("无法解析文件大小".to_string()))?;
    let mtime = mtime.ok_or_else(|| GlusterError::ParseError("无法解析修改时间".to_string()))?;

    let mut stat = FileStat::new(size, mtime);
    stat.atime = atime;
    stat.mode = mode;
    stat.owner = owner;
    stat.group = group;

    Ok(stat)
}

/// 格式化时间字符串（去掉纳秒部分）
fn format_time(time: &str) -> String {
    // 2026-01-08 10:20:30.123456789 +0800 -> 2026-01-08 10:20:30
    if let Some(dot_pos) = time.find('.') {
        time[..dot_pos].to_string()
    } else {
        time.to_string()
    }
}

/// 解析 `gluster volume heal <vol> info split-brain` 的条目数量
/// 用于验证修复是否成功
pub fn parse_split_brain_count(output: &str) -> usize {
    let mut total = 0;

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Number of entries:") {
            let count_str = line.trim_start_matches("Number of entries:").trim();
            if let Ok(count) = count_str.parse::<usize>() {
                total += count;
            }
        }
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_split_brain_regular() {
        let output = r#"
Brick server1:/data/brick1
/images/vm-disk-1.qcow2
Status: Connected
Number of entries: 1

Brick server2:/data/brick1
/images/vm-disk-1.qcow2
Status: Connected
Number of entries: 1
"#;

        let info = parse_split_brain_info(output, "gv0").unwrap();

        assert_eq!(info.volume_name, "gv0");
        assert_eq!(info.raw_count, 2);
        assert_eq!(info.entry_count(), 1); // 去重后只有一个
        assert!(!info.entries[0].is_shard());
        assert_eq!(info.entries[0].path, "/images/vm-disk-1.qcow2");
        assert_eq!(info.entries[0].brick_locations.len(), 2);
    }

    #[test]
    fn test_parse_split_brain_shard() {
        let output = r#"
Brick server1:/data/brick1
<gfid:307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b>.15
<gfid:307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b>.82
/images/vm-disk-1.qcow2
Status: Connected
Number of entries: 3

Brick server2:/data/brick1
<gfid:307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b>.15
<gfid:307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b>.82
/images/vm-disk-1.qcow2
Status: Connected
Number of entries: 3
"#;

        let info = parse_split_brain_info(output, "gv0").unwrap();

        assert_eq!(info.raw_count, 6);
        assert_eq!(info.entry_count(), 3); // 去重后 3 个
        assert_eq!(info.regular_files().len(), 1);
        assert_eq!(info.shard_files().len(), 2);

        let shards = info.shards_by_gfid();
        assert_eq!(shards.len(), 1);
        assert!(shards.contains_key("307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b"));
    }

    #[test]
    fn test_parse_shard_gfid() {
        let (gfid, index) =
            parse_shard_gfid("<gfid:307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b>.15").unwrap();
        assert_eq!(gfid, "307a5c9e-5f21-4f4d-8b02-d4bcdcbd1b");
        assert_eq!(index, 15);

        assert!(parse_shard_gfid("/images/disk.qcow2").is_none());
    }

    #[test]
    fn test_parse_afr_attributes() {
        let output = r#"
# file: /data/brick1/images/vm-disk.qcow2
trusted.afr.gv0-client-0=0x000000000000000200000000
trusted.afr.gv0-client-1=0x000000000000000000000000
trusted.glusterfs.pathinfo="..."
trusted.glusterfs.version=0x00000001
"#;

        let attrs = parse_afr_attributes(output);

        assert_eq!(attrs.len(), 2);
        assert!(attrs.contains(&"trusted.afr.gv0-client-0".to_string()));
        assert!(attrs.contains(&"trusted.afr.gv0-client-1".to_string()));
    }

    #[test]
    fn test_parse_stat_output() {
        let output = r#"
  File: /data/brick1/images/vm-disk.qcow2
  Size: 23068672000      Blocks: 45056000   IO Block: 4096   regular file
Access: (0644/-rw-r--r--)  Uid: (    0/    root)   Gid: (    0/    root)
Access: 2026-01-08 10:20:30.123456789 +0800
Modify: 2026-01-08 10:23:45.987654321 +0800
Change: 2026-01-08 10:23:45.987654321 +0800
 Birth: -
"#;

        let stat = parse_stat_output(output).unwrap();

        assert_eq!(stat.size, 23068672000);
        assert_eq!(stat.mtime, "2026-01-08 10:23:45");
        assert_eq!(stat.atime, Some("2026-01-08 10:20:30".to_string()));
        assert_eq!(stat.mode, Some("0644/-rw-r--r--".to_string()));
        assert_eq!(stat.owner, Some("root".to_string()));
        assert_eq!(stat.group, Some("root".to_string()));
    }

    #[test]
    fn test_parse_split_brain_count() {
        let output = r#"
Brick server1:/data/brick1
Number of entries: 3

Brick server2:/data/brick1
Number of entries: 3
"#;

        assert_eq!(parse_split_brain_count(output), 6);

        let healed = r#"
Brick server1:/data/brick1
Number of entries: 0

Brick server2:/data/brick1
Number of entries: 0
"#;

        assert_eq!(parse_split_brain_count(healed), 0);
    }
}

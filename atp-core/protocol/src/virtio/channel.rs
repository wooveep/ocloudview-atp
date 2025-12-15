//! VirtioSerial 通道管理
//!
//! 负责发现和管理 virtio-serial 通道

use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use virt::domain::Domain;
use tracing::{debug, info, warn};

use crate::ProtocolError;

/// VirtioSerial 通道信息
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    /// 通道名称（如：com.vmagent.sock）
    pub name: String,

    /// Socket 路径
    pub socket_path: PathBuf,

    /// 是否已连接
    pub connected: bool,
}

/// VirtioSerial 通道
pub struct VirtioChannel {
    /// 通道信息
    info: ChannelInfo,

    /// Unix Socket 连接
    stream: Option<tokio::net::UnixStream>,
}

impl VirtioChannel {
    /// 从 libvirt Domain 发现通道
    pub async fn discover_from_domain(
        domain: &Domain,
        channel_name: &str,
    ) -> Result<Self, ProtocolError> {
        info!("发现 virtio-serial 通道: {}", channel_name);

        // 获取 domain XML
        let xml = domain.get_xml_desc(0)
            .map_err(|e| ProtocolError::ConnectionFailed(format!("获取 XML 失败: {}", e)))?;

        // 解析 XML 查找通道路径
        let socket_path = Self::parse_channel_path(&xml, channel_name)?;

        debug!("找到通道路径: {:?}", socket_path);

        let info = ChannelInfo {
            name: channel_name.to_string(),
            socket_path: socket_path.clone(),
            connected: false,
        };

        Ok(Self {
            info,
            stream: None,
        })
    }

    /// 直接使用 socket 路径创建通道
    pub fn new(channel_name: &str, socket_path: PathBuf) -> Self {
        let info = ChannelInfo {
            name: channel_name.to_string(),
            socket_path,
            connected: false,
        };

        Self {
            info,
            stream: None,
        }
    }

    /// 连接到通道
    pub async fn connect(&mut self) -> Result<(), ProtocolError> {
        info!("连接到 virtio-serial 通道: {}", self.info.name);

        // 检查 socket 文件是否存在
        if !self.info.socket_path.exists() {
            return Err(ProtocolError::ConnectionFailed(
                format!("Socket 文件不存在: {:?}", self.info.socket_path)
            ));
        }

        // 连接到 Unix Socket
        let stream = tokio::net::UnixStream::connect(&self.info.socket_path)
            .await
            .map_err(|e| ProtocolError::ConnectionFailed(
                format!("连接失败: {}", e)
            ))?;

        self.stream = Some(stream);
        self.info.connected = true;

        info!("已连接到通道: {}", self.info.name);
        Ok(())
    }

    /// 发送原始数据
    pub async fn send_raw(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        let stream = self.stream.as_mut()
            .ok_or_else(|| ProtocolError::ConnectionFailed("未连接".to_string()))?;

        stream.write_all(data)
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;

        stream.flush()
            .await
            .map_err(|e| ProtocolError::SendFailed(e.to_string()))?;

        debug!("发送了 {} 字节数据", data.len());
        Ok(())
    }

    /// 接收原始数据
    pub async fn receive_raw(&mut self, buffer: &mut [u8]) -> Result<usize, ProtocolError> {
        let stream = self.stream.as_mut()
            .ok_or_else(|| ProtocolError::ConnectionFailed("未连接".to_string()))?;

        let n = stream.read(buffer)
            .await
            .map_err(|e| ProtocolError::ReceiveFailed(e.to_string()))?;

        debug!("接收了 {} 字节数据", n);
        Ok(n)
    }

    /// 发送字符串
    pub async fn send_string(&mut self, text: &str) -> Result<(), ProtocolError> {
        self.send_raw(text.as_bytes()).await
    }

    /// 接收字符串（直到遇到换行符或缓冲区满）
    pub async fn receive_line(&mut self) -> Result<String, ProtocolError> {
        let stream = self.stream.as_mut()
            .ok_or_else(|| ProtocolError::ConnectionFailed("未连接".to_string()))?;

        let mut buffer = Vec::new();
        let mut byte = [0u8; 1];

        loop {
            let n = stream.read(&mut byte)
                .await
                .map_err(|e| ProtocolError::ReceiveFailed(e.to_string()))?;

            if n == 0 {
                break; // EOF
            }

            if byte[0] == b'\n' {
                break;
            }

            buffer.push(byte[0]);

            // 防止无限读取
            if buffer.len() > 1024 * 1024 {
                return Err(ProtocolError::ReceiveFailed("响应过长".to_string()));
            }
        }

        String::from_utf8(buffer)
            .map_err(|e| ProtocolError::ParseError(e.to_string()))
    }

    /// 断开连接
    pub async fn disconnect(&mut self) {
        if let Some(stream) = self.stream.take() {
            drop(stream);
            self.info.connected = false;
            info!("已断开通道: {}", self.info.name);
        }
    }

    /// 获取通道信息
    pub fn info(&self) -> &ChannelInfo {
        &self.info
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.info.connected && self.stream.is_some()
    }

    /// 解析 XML 获取通道路径
    fn parse_channel_path(xml: &str, channel_name: &str) -> Result<PathBuf, ProtocolError> {
        // 简单的 XML 解析，查找 channel 元素
        // 格式：<target type='virtio' name='com.vmagent.sock'/>
        //      <source mode='bind' path='/var/lib/libvirt/qemu/channel/target/...'/>
        // 注意：source 和 target 元素顺序可能不同

        let lines: Vec<&str> = xml.lines().collect();
        let mut in_channel = false;
        let mut current_channel_name: Option<String> = None;
        let mut current_source_path: Option<String> = None;

        for (_i, line) in lines.iter().enumerate() {
            // 检查是否是 channel 开始标签
            if line.contains("<channel type='unix'>") {
                in_channel = true;
                current_channel_name = None;
                current_source_path = None;
                continue;
            }

            // 检查 channel 结束
            if line.contains("</channel>") {
                // 检查当前 channel 是否匹配
                if let (Some(ref name), Some(ref path)) = (&current_channel_name, &current_source_path) {
                    if name == channel_name {
                        return Ok(PathBuf::from(path));
                    }
                }
                in_channel = false;
                current_channel_name = None;
                current_source_path = None;
                continue;
            }

            if !in_channel {
                continue;
            }

            // 查找 target name
            if line.contains("<target") && line.contains("name=") {
                if let Some(name) = Self::extract_attribute(line, "name") {
                    current_channel_name = Some(name);
                }
            }

            // 查找 source path
            if line.contains("<source") && line.contains("path=") {
                if let Some(path) = Self::extract_attribute(line, "path") {
                    current_source_path = Some(path);
                }
            }
        }

        Err(ProtocolError::ConnectionFailed(
            format!("未找到通道: {}", channel_name)
        ))
    }

    /// 从 XML 属性中提取值
    fn extract_attribute(line: &str, attr: &str) -> Option<String> {
        let pattern = format!("{}='", attr);
        if let Some(start) = line.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = line[value_start..].find('\'') {
                return Some(line[value_start..value_start + end].to_string());
            }
        }
        None
    }
}

impl Drop for VirtioChannel {
    fn drop(&mut self) {
        if self.stream.is_some() {
            warn!("VirtioChannel 被 drop 时仍处于连接状态");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_channel_path() {
        let xml = r#"
<domain>
  <devices>
    <channel type='unix'>
      <source mode='bind' path='/var/lib/libvirt/qemu/channel/target/test-uuid'/>
      <target type='virtio' name='com.vmagent.sock' state='connected'/>
      <address type='virtio-serial' controller='0' bus='0' port='4'/>
    </channel>
  </devices>
</domain>
"#;

        let result = VirtioChannel::parse_channel_path(xml, "com.vmagent.sock");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert_eq!(path.to_str().unwrap(), "/var/lib/libvirt/qemu/channel/target/test-uuid");
    }

    #[test]
    fn test_extract_attribute() {
        let line = "  <source mode='bind' path='/var/lib/test'/>";

        let mode = VirtioChannel::extract_attribute(line, "mode");
        assert_eq!(mode, Some("bind".to_string()));

        let path = VirtioChannel::extract_attribute(line, "path");
        assert_eq!(path, Some("/var/lib/test".to_string()));
    }
}

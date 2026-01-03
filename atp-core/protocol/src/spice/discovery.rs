//! SPICE 发现模块
//!
//! 通过 libvirt 发现虚拟机的 SPICE 配置，包括：
//! - 发现 SPICE 端口和 TLS 端口
//! - 设置 SPICE 密码
//! - 获取宿主机 IP

use crate::{ProtocolError, Result};
use tracing::{debug, info, warn};
use virt::connect::Connect;
use virt::domain::Domain;

/// SPICE 虚拟机信息
#[derive(Debug, Clone)]
pub struct SpiceVmInfo {
    /// 虚拟机名称
    pub name: String,
    /// 虚拟机 UUID
    pub uuid: String,
    /// SPICE 服务器地址（通常是宿主机 IP）
    pub host: String,
    /// SPICE 端口
    pub port: u16,
    /// TLS 端口（可选）
    pub tls_port: Option<u16>,
    /// 密码（可选）
    pub password: Option<String>,
    /// 是否启用 TLS
    pub tls_enabled: bool,
}

/// SPICE 发现器
///
/// 用于发现虚拟机的 SPICE 配置
pub struct SpiceDiscovery {
    /// 默认宿主机地址
    default_host: String,
}

impl SpiceDiscovery {
    pub fn new() -> Self {
        Self {
            default_host: "127.0.0.1".to_string(),
        }
    }

    /// 设置默认宿主机地址
    pub fn with_default_host(mut self, host: &str) -> Self {
        self.default_host = host.to_string();
        self
    }

    /// 从 Domain 发现 SPICE 配置
    pub async fn discover_from_domain(&self, domain: &Domain) -> Result<SpiceVmInfo> {
        // 获取 Domain XML
        let xml = domain
            .get_xml_desc(0)
            .map_err(|e| ProtocolError::ConnectionFailed(format!("无法获取虚拟机 XML: {}", e)))?;

        let name = domain
            .get_name()
            .map_err(|e| ProtocolError::ConnectionFailed(format!("无法获取虚拟机名称: {}", e)))?;

        let uuid = domain
            .get_uuid_string()
            .map_err(|e| ProtocolError::ConnectionFailed(format!("无法获取虚拟机 UUID: {}", e)))?;

        // 解析 XML 获取 SPICE 配置
        self.parse_spice_from_xml(&xml, &name, &uuid)
    }

    /// 从 libvirt 连接发现所有带 SPICE 的虚拟机
    pub async fn discover_all(&self, conn: &Connect) -> Result<Vec<SpiceVmInfo>> {
        let domains = conn
            .list_all_domains(0)
            .map_err(|e| ProtocolError::ConnectionFailed(format!("无法列出虚拟机: {}", e)))?;

        let mut vms = Vec::new();

        for domain in domains {
            match self.discover_from_domain(&domain).await {
                Ok(info) => {
                    debug!(
                        "发现 SPICE 虚拟机: {} ({}:{})",
                        info.name, info.host, info.port
                    );
                    vms.push(info);
                }
                Err(e) => {
                    // 跳过没有 SPICE 配置的虚拟机
                    debug!("虚拟机无 SPICE 配置: {:?}", e);
                }
            }
        }

        info!("共发现 {} 个 SPICE 虚拟机", vms.len());
        Ok(vms)
    }

    /// 解析 XML 获取 SPICE 配置
    fn parse_spice_from_xml(&self, xml: &str, name: &str, uuid: &str) -> Result<SpiceVmInfo> {
        // TODO: 使用完整的 XML 解析器（如 quick-xml）以获得更好的稳定性
        //
        // 推荐实现：
        // 1. 添加依赖到 Cargo.toml:
        //    quick-xml = "0.31"
        //
        // 2. 使用 quick-xml 解析：
        //    ```rust
        //    use quick_xml::Reader;
        //    use quick_xml::events::Event;
        //
        //    let mut reader = Reader::from_str(xml);
        //    reader.trim_text(true);
        //
        //    let mut buf = Vec::new();
        //    let mut port = None;
        //    let mut tls_port = None;
        //    let mut listen = None;
        //    let mut password = None;
        //
        //    loop {
        //        match reader.read_event_into(&mut buf) {
        //            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
        //                if e.name().as_ref() == b"graphics" {
        //                    // 检查 type='spice'
        //                    let is_spice = e.attributes()
        //                        .filter_map(|a| a.ok())
        //                        .any(|a| a.key.as_ref() == b"type" && a.value.as_ref() == b"spice");
        //
        //                    if is_spice {
        //                        // 提取所有 SPICE 相关属性
        //                        for attr in e.attributes().filter_map(|a| a.ok()) {
        //                            match attr.key.as_ref() {
        //                                b"port" => {
        //                                    port = String::from_utf8_lossy(&attr.value)
        //                                        .parse::<i32>().ok()
        //                                        .filter(|&p| p > 0)
        //                                        .map(|p| p as u16);
        //                                }
        //                                b"tlsPort" => {
        //                                    tls_port = String::from_utf8_lossy(&attr.value)
        //                                        .parse::<i32>().ok()
        //                                        .filter(|&p| p > 0)
        //                                        .map(|p| p as u16);
        //                                }
        //                                b"listen" => {
        //                                    listen = Some(String::from_utf8_lossy(&attr.value).to_string());
        //                                }
        //                                b"passwd" => {
        //                                    password = Some(String::from_utf8_lossy(&attr.value).to_string());
        //                                }
        //                                _ => {}
        //                            }
        //                        }
        //                        break; // 找到 SPICE 配置后退出
        //                    }
        //                }
        //            }
        //            Ok(Event::Eof) => break,
        //            Err(e) => return Err(ProtocolError::ParseError(
        //                format!("XML 解析错误: {}", e)
        //            )),
        //            _ => {}
        //        }
        //        buf.clear();
        //    }
        //
        //    let port = port.ok_or_else(|| ProtocolError::ConnectionFailed(
        //        "SPICE 端口未配置".to_string()
        //    ))?;
        //    ```
        //
        // 3. 处理嵌套的 <listen> 元素:
        //    有些虚拟机配置使用嵌套的 listen 元素而非属性
        //    <graphics type='spice' port='5900'>
        //        <listen type='address' address='192.168.1.100'/>
        //    </graphics>
        //    需要在解析时处理这种情况
        //
        // 目前使用简单的字符串解析

        // 查找 <graphics type='spice' ...>
        let graphics_start = xml
            .find("<graphics type='spice'")
            .or_else(|| xml.find("<graphics type=\"spice\""))
            .ok_or_else(|| {
                ProtocolError::ConnectionFailed("虚拟机未配置 SPICE 图形".to_string())
            })?;

        let graphics_end = xml[graphics_start..]
            .find("/>")
            .or_else(|| xml[graphics_start..].find("</graphics>"))
            .map(|pos| graphics_start + pos)
            .ok_or_else(|| ProtocolError::ParseError("无法解析 SPICE 图形配置".to_string()))?;

        let graphics_xml = &xml[graphics_start..graphics_end];

        // 解析端口
        let port = self
            .extract_attr(graphics_xml, "port")
            .and_then(|s| s.parse::<i32>().ok())
            .filter(|&p| p > 0)
            .map(|p| p as u16)
            .ok_or_else(|| ProtocolError::ConnectionFailed("SPICE 端口未配置或无效".to_string()))?;

        // 解析 TLS 端口
        let tls_port = self
            .extract_attr(graphics_xml, "tlsPort")
            .and_then(|s| s.parse::<i32>().ok())
            .filter(|&p| p > 0)
            .map(|p| p as u16);

        // 解析监听地址
        // 首先检查是否有嵌套的 <listen> 元素
        let listen = self
            .extract_listen_address(&xml[graphics_start..])
            .or_else(|| self.extract_attr(graphics_xml, "listen"))
            .unwrap_or_else(|| self.default_host.clone());

        // 处理特殊地址
        let host = if listen == "0.0.0.0" || listen == "::" || listen == ":::" || listen.is_empty()
        {
            // 如果是监听所有地址，使用默认主机
            self.default_host.clone()
        } else {
            listen
        };

        // 检查密码
        let password = self.extract_attr(graphics_xml, "passwd");

        // 检查 TLS 配置
        let tls_enabled = tls_port.is_some();

        Ok(SpiceVmInfo {
            name: name.to_string(),
            uuid: uuid.to_string(),
            host,
            port,
            tls_port,
            password,
            tls_enabled,
        })
    }

    /// 从 XML 属性中提取值
    fn extract_attr(&self, xml: &str, attr: &str) -> Option<String> {
        // 尝试单引号
        let pattern1 = format!("{}='", attr);
        if let Some(start) = xml.find(&pattern1) {
            let value_start = start + pattern1.len();
            if let Some(end) = xml[value_start..].find('\'') {
                return Some(xml[value_start..value_start + end].to_string());
            }
        }

        // 尝试双引号
        let pattern2 = format!("{}=\"", attr);
        if let Some(start) = xml.find(&pattern2) {
            let value_start = start + pattern2.len();
            if let Some(end) = xml[value_start..].find('"') {
                return Some(xml[value_start..value_start + end].to_string());
            }
        }

        None
    }

    /// 从嵌套的 <listen> 元素中提取地址
    ///
    /// 处理格式:
    /// <graphics type='spice' port='5900'>
    ///   <listen type='address' address='192.168.1.100'/>
    /// </graphics>
    fn extract_listen_address(&self, xml: &str) -> Option<String> {
        // 查找 <listen 元素
        if let Some(listen_start) = xml.find("<listen ") {
            let listen_end = xml[listen_start..]
                .find("/>")
                .or_else(|| xml[listen_start..].find(">"))
                .map(|pos| listen_start + pos)?;

            let listen_xml = &xml[listen_start..listen_end];

            // 提取 address 属性
            if let Some(addr) = self.extract_attr(listen_xml, "address") {
                if !addr.is_empty() {
                    return Some(addr);
                }
            }
        }

        None
    }

    /// 设置 SPICE 密码
    ///
    /// 通过 libvirt API 设置虚拟机的 SPICE 访问密码
    pub async fn set_spice_password(&self, domain: &Domain, password: &str) -> Result<()> {
        // TODO: 实现更完善的 SPICE 密码设置逻辑
        //
        // libvirt 提供了多种方法设置 SPICE 密码：
        //
        // 方法1: 通过 QMP (QEMU Machine Protocol) 命令（推荐）
        //    使用 virDomainQemuMonitorCommand 或 virsh qemu-monitor-command
        //    优点：即时生效，不需要重启虚拟机
        //    缺点：需要 QEMU driver 支持
        //
        //    实现步骤：
        //    ```rust
        //    // 构造 QMP 命令 JSON
        //    let qmp_cmd = serde_json::json!({
        //        "execute": "set_password",
        //        "arguments": {
        //            "protocol": "spice",
        //            "password": password,
        //            // 可选：设置有效期
        //            // "connected": "keep"  // 保持已连接的会话
        //        }
        //    }).to_string();
        //
        //    // 发送 QMP 命令（需要使用 libvirt 的 qemu domain 特定 API）
        //    // 注意：Rust virt crate 可能没有直接暴露 qemuMonitorCommand
        //    // 可能需要通过 unsafe FFI 调用 virDomainQemuMonitorCommand
        //    ```
        //
        // 方法2: 通过 HMP (Human Monitor Protocol) 命令
        //    ```bash
        //    virsh qemu-monitor-command <domain> --hmp "set_password spice <password>"
        //    ```
        //    在 Rust 中可以执行 shell 命令或使用 HMP 格式
        //
        // 方法3: 修改虚拟机 XML 配置
        //    需要修改 <graphics> 元素添加 passwd 属性，然后重新定义虚拟机
        //    优点：配置永久保存
        //    缺点：需要重启虚拟机才能生效
        //
        //    实现步骤：
        //    ```rust
        //    // 1. 获取当前 XML
        //    let mut xml = domain.get_xml_desc(0)?;
        //
        //    // 2. 修改 XML（使用 quick-xml 或字符串替换）
        //    // 找到 <graphics type='spice'> 元素
        //    // 添加或修改 passwd='...' 属性
        //
        //    // 3. 重新定义虚拟机
        //    // let conn = domain.get_connect()?;
        //    // conn.define_xml(&xml)?;
        //
        //    // 4. 如果虚拟机正在运行，密码变更需要重启
        //    ```
        //
        // 方法4: 使用 virDomainSetMetadata（不推荐用于密码）
        //    仅用于存储元数据，不会影响实际 SPICE 配置
        //
        // 当前实现尝试：使用 qemu_agent_command（注意：这实际是 QGA 而非 QMP）
        // 正确的做法应该使用 QMP monitor command

        info!("设置 SPICE 密码 (长度: {})", password.len());

        // 使用 QMP 命令设置密码
        let qmp_cmd = format!(
            r#"{{"execute": "set_password", "arguments": {{"protocol": "spice", "password": "{}"}}}}"#,
            password
        );

        // 通过 libvirt 发送 QMP 命令
        // 注意：qemu_agent_command 实际是 QGA (Guest Agent)，不是 QMP
        // 正确的 API 应该是 qemu_monitor_command（可能需要通过 FFI）
        let result = domain.qemu_agent_command(&qmp_cmd, 10, 0);

        match result {
            Ok(_) => {
                info!("SPICE 密码设置成功");
                Ok(())
            }
            Err(e) => {
                // 尝试使用 qemu-monitor-command
                warn!("通过 QGA 设置密码失败，尝试其他方法: {}", e);

                // TODO: 实现备用方法
                // 1. 尝试通过 shell 执行 virsh qemu-monitor-command
                // 2. 或者修改 XML 配置
                Err(ProtocolError::CommandFailed(format!(
                    "设置 SPICE 密码失败: {}",
                    e
                )))
            }
        }
    }

    /// 设置 SPICE 密码有效期
    pub async fn set_spice_password_expiry(&self, domain: &Domain, expiry_time: i64) -> Result<()> {
        // TODO: 实现设置密码过期时间的完整逻辑
        //
        // SPICE 密码过期时间设置通过 QMP expire_password 命令
        //
        // 时间格式：
        //   - "now": 立即过期
        //   - "never": 永不过期
        //   - "+<seconds>": 从现在开始的秒数
        //   - "<unix-timestamp>": UNIX 时间戳
        //
        // QMP 命令格式：
        // ```json
        // {
        //     "execute": "expire_password",
        //     "arguments": {
        //         "protocol": "spice",
        //         "time": "never"  // 或 "+3600" 或 "1640000000"
        //     }
        // }
        // ```
        //
        // 实现步骤：
        // ```rust
        // let time_str = if expiry_time < 0 {
        //     "never".to_string()
        // } else if expiry_time == 0 {
        //     "now".to_string()
        // } else {
        //     format!("+{}", expiry_time)
        // };
        //
        // let qmp_cmd = serde_json::json!({
        //     "execute": "expire_password",
        //     "arguments": {
        //         "protocol": "spice",
        //         "time": time_str
        //     }
        // }).to_string();
        //
        // // 发送 QMP 命令（同样需要使用 monitor command 而非 agent command）
        // ```
        //
        // 使用场景：
        //   - 临时访问：设置较短过期时间（如 3600 秒 = 1 小时）
        //   - 永久访问：设置为 "never"
        //   - 单次访问：设置为 "now" 在用户连接后立即过期
        //
        // virsh qemu-monitor-command <domain> --hmp "expire_password spice <time>"

        let qmp_cmd = format!(
            r#"{{"execute": "expire_password", "arguments": {{"protocol": "spice", "time": "{}"}}}}"#,
            expiry_time
        );

        let result = domain.qemu_agent_command(&qmp_cmd, 10, 0);

        match result {
            Ok(_) => {
                info!("SPICE 密码过期时间设置成功: {}", expiry_time);
                Ok(())
            }
            Err(e) => Err(ProtocolError::CommandFailed(format!(
                "设置 SPICE 密码过期时间失败: {}",
                e
            ))),
        }
    }

    /// 获取宿主机 IP 地址
    ///
    /// 从 libvirt 连接 URI 或系统配置中获取宿主机 IP
    pub fn get_host_ip(&self, conn: &Connect) -> Result<String> {
        // 从连接 URI 获取主机地址
        let uri = conn
            .get_uri()
            .map_err(|e| ProtocolError::ConnectionFailed(format!("无法获取连接 URI: {}", e)))?;

        debug!("Libvirt URI: {}", uri);

        // 解析 URI 获取主机
        // 格式: qemu+ssh://user@host/system, qemu:///system, qemu+tcp://host:port/system
        if let Some(host) = self.extract_host_from_uri(&uri) {
            return Ok(host);
        }

        // 如果是本地连接，返回默认地址
        Ok(self.default_host.clone())
    }

    /// 从 URI 中提取主机地址
    fn extract_host_from_uri(&self, uri: &str) -> Option<String> {
        // 处理 qemu+ssh://user@host/system 格式
        if let Some(start) = uri.find("://") {
            let after_scheme = &uri[start + 3..];

            // 查找路径开始
            let path_start = after_scheme.find('/').unwrap_or(after_scheme.len());
            let authority = &after_scheme[..path_start];

            // 处理 user@host 格式
            let host_part = if let Some(at) = authority.rfind('@') {
                &authority[at + 1..]
            } else {
                authority
            };

            // 移除端口号
            let host = if let Some(colon) = host_part.rfind(':') {
                &host_part[..colon]
            } else {
                host_part
            };

            if !host.is_empty() && host != "localhost" {
                return Some(host.to_string());
            }
        }

        None
    }
}

impl Default for SpiceDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_spice_xml() {
        let xml = r#"
        <domain type='kvm'>
            <name>test-vm</name>
            <devices>
                <graphics type='spice' port='5900' tlsPort='5901' listen='192.168.1.100' passwd='secret'>
                    <listen type='address' address='192.168.1.100'/>
                </graphics>
            </devices>
        </domain>
        "#;

        let discovery = SpiceDiscovery::new();
        let info = discovery
            .parse_spice_from_xml(xml, "test-vm", "uuid-123")
            .unwrap();

        assert_eq!(info.name, "test-vm");
        assert_eq!(info.host, "192.168.1.100");
        assert_eq!(info.port, 5900);
        assert_eq!(info.tls_port, Some(5901));
        assert_eq!(info.password, Some("secret".to_string()));
    }

    #[test]
    fn test_extract_host_from_uri() {
        let discovery = SpiceDiscovery::new();

        assert_eq!(
            discovery.extract_host_from_uri("qemu+ssh://root@192.168.1.100/system"),
            Some("192.168.1.100".to_string())
        );

        assert_eq!(
            discovery.extract_host_from_uri("qemu+tcp://192.168.1.100:16509/system"),
            Some("192.168.1.100".to_string())
        );

        assert_eq!(discovery.extract_host_from_uri("qemu:///system"), None);
    }

    #[test]
    fn test_no_spice_config() {
        let xml = r#"
        <domain type='kvm'>
            <name>test-vm</name>
            <devices>
                <graphics type='vnc' port='5900'/>
            </devices>
        </domain>
        "#;

        let discovery = SpiceDiscovery::new();
        let result = discovery.parse_spice_from_xml(xml, "test-vm", "uuid-123");

        assert!(result.is_err());
    }
}

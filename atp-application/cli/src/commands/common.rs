//! 公共工具函数模块
//!
//! 提供各命令模块共享的功能，包括：
//! - VDI 客户端创建和登录
//! - 主机ID到主机名映射
//! - libvirt 连接管理

use anyhow::{Context, Result};
use atp_executor::VdiConfig;
use atp_transport::{HostConnection, HostInfo};
use atp_vdiplatform::{VdiClient, client::VdiConfig as VdiClientConfig};
use std::collections::HashMap;
use tracing::info;

/// VDI 主机信息
#[derive(Debug, Clone)]
pub struct VdiHostInfo {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub status: i64,
    pub cpu_size: i64,
    pub memory_gb: f64,
}

impl VdiHostInfo {
    /// 从 JSON 值解析主机信息
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        let id = value["id"].as_str()?.to_string();
        let name = value["name"].as_str()?.to_string();
        let ip = value["ip"].as_str().unwrap_or("").to_string();
        let status = value["status"].as_i64().unwrap_or(-1);
        let cpu_size = value["cpuSize"].as_i64().unwrap_or(0);
        let memory_gb = value["memory"].as_f64().unwrap_or(0.0);

        Some(Self {
            id,
            name,
            ip,
            status,
            cpu_size,
            memory_gb,
        })
    }

    /// 检查主机是否在线
    pub fn is_online(&self) -> bool {
        self.status == 1
    }

    /// 获取状态显示文本
    pub fn status_display(&self) -> &'static str {
        if self.is_online() {
            "在线 ✅"
        } else {
            "离线 ❌"
        }
    }
}

/// 创建并登录VDI客户端
pub async fn create_vdi_client(vdi_config: &VdiConfig) -> Result<VdiClient> {
    let client_config = VdiClientConfig {
        connect_timeout: vdi_config.connect_timeout,
        request_timeout: vdi_config.connect_timeout,
        max_retries: 3,
        verify_ssl: vdi_config.verify_ssl,
    };

    let mut client = VdiClient::new(&vdi_config.base_url, client_config)
        .context("创建VDI客户端失败")?;

    client
        .login(&vdi_config.username, &vdi_config.password)
        .await
        .context("VDI登录失败")?;

    Ok(client)
}

/// 从 VDI 获取主机列表并解析
pub async fn fetch_vdi_hosts(client: &VdiClient) -> Result<Vec<VdiHostInfo>> {
    let hosts_json = client.host().list_all().await?;

    let hosts: Vec<VdiHostInfo> = hosts_json
        .iter()
        .filter_map(VdiHostInfo::from_json)
        .collect();

    Ok(hosts)
}

/// 构建主机ID到主机名的映射
pub fn build_host_id_to_name_map(hosts: &[VdiHostInfo]) -> HashMap<String, String> {
    hosts
        .iter()
        .filter(|h| !h.id.is_empty() && !h.name.is_empty())
        .map(|h| (h.id.clone(), h.name.clone()))
        .collect()
}

/// 从 JSON 数组构建主机ID到主机名的映射
pub fn build_host_id_to_name_map_from_json(hosts: &[serde_json::Value]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for host in hosts {
        let host_id = host["id"].as_str().unwrap_or("").to_string();
        let host_name = host["name"].as_str().unwrap_or("").to_string();
        if !host_id.is_empty() && !host_name.is_empty() {
            map.insert(host_id, host_name);
        }
    }
    map
}

/// libvirt 连接结果
pub struct LibvirtConnectionResult {
    pub connection: HostConnection,
    pub uri: String,
}

/// 尝试连接到主机的 libvirt
///
/// 按顺序尝试 TCP 和 SSH 连接方式
pub async fn connect_libvirt(host_name: &str, host_ip: &str) -> Result<LibvirtConnectionResult> {
    let uris = vec![
        format!("qemu+tcp://{}/system", host_ip),
        format!("qemu+ssh://root@{}/system", host_ip),
    ];

    for uri in &uris {
        let host_info = HostInfo {
            id: host_name.to_string(),
            host: host_name.to_string(),
            uri: uri.clone(),
            tags: vec![],
            metadata: HashMap::new(),
        };

        let conn = HostConnection::new(host_info);
        match conn.connect().await {
            Ok(_) => {
                if conn.is_alive().await {
                    info!("连接成功: {}", uri);
                    return Ok(LibvirtConnectionResult {
                        connection: conn,
                        uri: uri.clone(),
                    });
                }
            }
            Err(e) => {
                info!("连接失败 {}: {}", uri, e);
            }
        }
    }

    anyhow::bail!("无法连接到主机 {} 的 libvirtd", host_name)
}

/// 尝试使用指定 URI 连接到主机的 libvirt
pub async fn connect_libvirt_with_uri(host_name: &str, uri: &str) -> Result<HostConnection> {
    let host_info = HostInfo {
        id: host_name.to_string(),
        host: host_name.to_string(),
        uri: uri.to_string(),
        tags: vec![],
        metadata: HashMap::new(),
    };

    let conn = HostConnection::new(host_info);
    conn.connect().await.context(format!("连接失败: {}", uri))?;

    if !conn.is_alive().await {
        anyhow::bail!("连接已建立但不可用: {}", uri);
    }

    Ok(conn)
}

/// 测试主机 libvirt 连接
pub async fn test_libvirt_connection(host_name: &str, host_ip: &str) -> bool {
    connect_libvirt(host_name, host_ip).await.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_vdi_host_info_from_json() {
        let json = json!({
            "id": "host-001",
            "name": "host1",
            "ip": "192.168.1.1",
            "status": 1,
            "cpuSize": 16,
            "memory": 64.0
        });

        let host = VdiHostInfo::from_json(&json).unwrap();
        assert_eq!(host.id, "host-001");
        assert_eq!(host.name, "host1");
        assert_eq!(host.ip, "192.168.1.1");
        assert!(host.is_online());
    }

    #[test]
    fn test_build_host_id_to_name_map() {
        let hosts = vec![
            VdiHostInfo {
                id: "id1".to_string(),
                name: "host1".to_string(),
                ip: "1.1.1.1".to_string(),
                status: 1,
                cpu_size: 8,
                memory_gb: 32.0,
            },
            VdiHostInfo {
                id: "id2".to_string(),
                name: "host2".to_string(),
                ip: "2.2.2.2".to_string(),
                status: 1,
                cpu_size: 16,
                memory_gb: 64.0,
            },
        ];

        let map = build_host_id_to_name_map(&hosts);
        assert_eq!(map.get("id1"), Some(&"host1".to_string()));
        assert_eq!(map.get("id2"), Some(&"host2".to_string()));
    }
}

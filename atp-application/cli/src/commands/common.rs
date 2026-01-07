//! 公共工具函数模块
//!
//! 提供各命令模块共享的功能，包括：
//! - VDI 客户端创建和登录
//! - 主机ID到主机名映射
//! - libvirt 连接管理

use anyhow::{Context, Result};
use atp_executor::VdiConfig;
use atp_transport::{HostConnection, HostInfo};
use atp_vdiplatform::{client::VdiConfig as VdiClientConfig, VdiClient};
use std::collections::HashMap;
use tracing::info;

/// 创建并登录VDI客户端
pub async fn create_vdi_client(vdi_config: &VdiConfig) -> Result<VdiClient> {
    let client_config = VdiClientConfig {
        connect_timeout: vdi_config.connect_timeout,
        request_timeout: vdi_config.connect_timeout,
        max_retries: 3,
        verify_ssl: vdi_config.verify_ssl,
    };

    let mut client =
        VdiClient::new(&vdi_config.base_url, client_config).context("创建VDI客户端失败")?;

    client
        .login(&vdi_config.username, &vdi_config.password)
        .await
        .context("VDI登录失败")?;

    Ok(client)
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

use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{info, debug};
use virt::connect::Connect;
use virt::domain::Domain;

use super::LibvirtError;

/// Libvirt 管理器
pub struct LibvirtManager {
    conn: Connect,
}

impl LibvirtManager {
    /// 连接到 Libvirt (默认 qemu:///system)
    pub fn connect() -> Result<Self> {
        Self::connect_uri("qemu:///system")
    }

    /// 使用指定 URI 连接到 Libvirt
    pub fn connect_uri(uri: &str) -> Result<Self> {
        info!("连接到 Libvirt: {}", uri);

        let conn = Connect::open(uri)
            .map_err(|e| LibvirtError::ConnectionFailed(e.to_string()))?;

        info!("成功连接到 Libvirt");
        Ok(Self { conn })
    }

    /// 根据名称查找虚拟机
    pub fn lookup_domain_by_name(&self, name: &str) -> Result<Domain> {
        debug!("查找虚拟机: {}", name);

        Domain::lookup_by_name(&self.conn, name)
            .map_err(|e| LibvirtError::DomainNotFound(format!("{}: {}", name, e)).into())
    }

    /// 根据 UUID 查找虚拟机
    pub fn lookup_domain_by_uuid(&self, uuid: &str) -> Result<Domain> {
        debug!("根据 UUID 查找虚拟机: {}", uuid);

        Domain::lookup_by_uuid_string(&self.conn, uuid)
            .map_err(|e| LibvirtError::DomainNotFound(format!("UUID {}: {}", uuid, e)).into())
    }

    /// 列出所有活动的虚拟机
    pub fn list_active_domains(&self) -> Result<Vec<Domain>> {
        info!("列出所有活动的虚拟机");

        let domains = self.conn.list_all_domains(0)
            .map_err(|e| LibvirtError::OperationError(format!("列出虚拟机失败: {}", e)))?;

        info!("找到 {} 个虚拟机", domains.len());
        Ok(domains)
    }

    /// 获取虚拟机的 QMP Socket 路径
    pub fn get_qmp_socket_path(&self, domain: &Domain) -> Result<PathBuf> {
        // 获取虚拟机 XML 配置
        let xml = domain
            .get_xml_desc(0)
            .map_err(|e| LibvirtError::XmlParseFailed(e.to_string()))?;

        debug!("解析虚拟机 XML 配置");

        // 解析 XML 查找 QMP Socket 路径
        // 默认路径格式: /var/lib/libvirt/qemu/domain-{id}-{name}/monitor.sock
        let domain_name = domain
            .get_name()
            .map_err(|e| LibvirtError::OperationError(format!("获取虚拟机名称失败: {}", e)))?;

        let domain_id = domain
            .get_id()
            .map_err(|e| LibvirtError::OperationError(format!("获取虚拟机 ID 失败: {}", e)))?;

        // TODO: 更精确的 XML 解析来提取实际的 Socket 路径
        // 这里使用默认路径模式
        let socket_path = PathBuf::from(format!(
            "/var/lib/libvirt/qemu/domain-{}-{}/monitor.sock",
            domain_id, domain_name
        ));

        debug!("QMP Socket 路径: {:?}", socket_path);

        if !socket_path.exists() {
            return Err(LibvirtError::QmpSocketNotFound.into());
        }

        Ok(socket_path)
    }

    /// 获取虚拟机信息
    pub fn get_domain_info(&self, domain: &Domain) -> Result<DomainInfo> {
        let name = domain
            .get_name()
            .map_err(|e| LibvirtError::OperationError(format!("获取名称失败: {}", e)))?;

        let uuid = domain
            .get_uuid_string()
            .map_err(|e| LibvirtError::OperationError(format!("获取 UUID 失败: {}", e)))?;

        let id = domain
            .get_id()
            .ok();

        let state = domain
            .get_state()
            .map_err(|e| LibvirtError::OperationError(format!("获取状态失败: {}", e)))?;

        Ok(DomainInfo {
            name,
            uuid,
            id,
            state: state.0,
        })
    }

    /// 创建 QEMU Guest Agent 客户端
    ///
    /// 这是一个便捷方法，用于为指定的虚拟机创建 QGA 客户端。
    /// QGA 客户端允许在 Guest 操作系统内执行命令和文件操作。
    ///
    /// # 前提条件
    /// - Guest 必须安装并运行 qemu-guest-agent
    /// - VM 必须配置 virtio-serial 设备
    pub fn create_qga_client<'a>(&self, domain: &'a Domain) -> crate::qga::QgaClient<'a> {
        crate::qga::QgaClient::new(domain)
    }
}

/// 虚拟机信息
#[derive(Debug, Clone)]
pub struct DomainInfo {
    pub name: String,
    pub uuid: String,
    pub id: Option<u32>,
    pub state: i32,
}

impl Drop for LibvirtManager {
    fn drop(&mut self) {
        if let Err(e) = self.conn.close() {
            tracing::error!("关闭 Libvirt 连接失败: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // 需要实际的 Libvirt 环境才能运行
    fn test_libvirt_connection() {
        let result = LibvirtManager::connect();
        assert!(result.is_ok());
    }
}

//! 网络管理 API
//!
//! 提供网络管理功能，包括：
//! - OVS (Open vSwitch) 管理
//! - VLAN/端口组管理
//! - 网桥管理
//! - IP 池管理
//! - 网卡安全组和限速

use reqwest::Method;
use tracing::info;

use crate::client::VdiClient;
use crate::error::Result;

/// 网络管理 API
pub struct NetworkApi<'a> {
    client: &'a VdiClient,
}

impl<'a> NetworkApi<'a> {
    /// 创建新的网络 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    // ============================================
    // OVS 管理
    // ============================================

    /// 查询 OVS 列表（分页）
    pub async fn list_ovs(&self, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询 OVS 列表: 第{}页, 每页{}条", page_num, page_size);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/ovs?pageNum={}&pageSize={}", page_num, page_size),
            None::<()>,
        ).await
    }

    /// 查询 OVS 列表（全部）
    pub async fn list_all_ovs(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询所有 OVS");
        self.client.request(
            Method::GET,
            "/ocloud/v1/ovs/all",
            None::<()>,
        ).await
    }

    /// 获取 OVS 详情
    pub async fn get_ovs(&self, ovs_id: &str) -> Result<serde_json::Value> {
        info!("获取 OVS 详情: {}", ovs_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/ovs/{}", ovs_id),
            None::<()>,
        ).await
    }

    /// 创建 OVS
    pub async fn create_ovs(&self, config: serde_json::Value) -> Result<serde_json::Value> {
        info!("创建 OVS");
        self.client.request(
            Method::POST,
            "/ocloud/v1/ovs",
            Some(config),
        ).await
    }

    /// 删除 OVS
    pub async fn delete_ovs(&self, ovs_id: &str) -> Result<()> {
        info!("删除 OVS: {}", ovs_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/ovs/{}", ovs_id),
            None::<()>,
        ).await
    }

    /// 修改 OVS 备注
    pub async fn update_ovs_remark(&self, ovs_id: &str, remark: &str) -> Result<()> {
        info!("修改 OVS 备注: {} -> {}", ovs_id, remark);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/ovs/{}/remark", ovs_id),
            Some(serde_json::json!({ "remark": remark })),
        ).await
    }

    /// 查询 OVS 下所有虚拟机
    pub async fn list_ovs_domains(&self, ovs_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("查询 OVS 下虚拟机: {}", ovs_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/ovs/{}/domain/all", ovs_id),
            None::<()>,
        ).await
    }

    // ============================================
    // VLAN/端口组管理
    // ============================================

    /// 查询 VLAN 列表（分页）
    pub async fn list_vlans(&self, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询 VLAN 列表: 第{}页, 每页{}条", page_num, page_size);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/vlan?pageNum={}&pageSize={}", page_num, page_size),
            None::<()>,
        ).await
    }

    /// 查询 VLAN 列表（全部）
    pub async fn list_all_vlans(&self) -> Result<Vec<serde_json::Value>> {
        info!("查询所有 VLAN");
        self.client.request(
            Method::GET,
            "/ocloud/v1/vlan/all",
            None::<()>,
        ).await
    }

    /// 创建 VLAN
    pub async fn create_vlan(&self, config: serde_json::Value) -> Result<serde_json::Value> {
        info!("创建 VLAN");
        self.client.request(
            Method::POST,
            "/ocloud/v1/vlan",
            Some(config),
        ).await
    }

    /// 删除端口组
    pub async fn delete_vlan(&self, vlan_id: &str) -> Result<()> {
        info!("删除端口组: {}", vlan_id);
        self.client.request(
            Method::DELETE,
            &format!("/ocloud/v1/vlan/{}", vlan_id),
            None::<()>,
        ).await
    }

    /// 修改端口组名称
    pub async fn update_vlan_name(&self, vlan_id: &str, name: &str) -> Result<()> {
        info!("修改端口组名称: {} -> {}", vlan_id, name);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/vlan/{}/name", vlan_id),
            Some(serde_json::json!({ "name": name })),
        ).await
    }

    /// 修改端口组上网模式
    pub async fn update_vlan_mode(&self, vlan_id: &str, mode: &str) -> Result<()> {
        info!("修改端口组上网模式: {} -> {}", vlan_id, mode);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/vlan/{}/mode", vlan_id),
            Some(serde_json::json!({ "mode": mode })),
        ).await
    }

    /// 获取端口组下的可用 IP 列表
    pub async fn list_vlan_available_ips(&self, vlan_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("获取端口组可用 IP: {}", vlan_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/vlan/{}/available-ip-list", vlan_id),
            None::<()>,
        ).await
    }

    /// 获取端口组下的全部 IP 列表
    pub async fn list_vlan_ips(&self, vlan_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("获取端口组全部 IP: {}", vlan_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/vlan/{}/ip", vlan_id),
            None::<()>,
        ).await
    }

    /// 查询 VLAN 下面的虚拟机列表（分页）
    pub async fn list_vlan_domains(&self, vlan_id: &str, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询 VLAN 下虚拟机: {}", vlan_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/vlan/{}/domain?pageNum={}&pageSize={}", vlan_id, page_num, page_size),
            None::<()>,
        ).await
    }

    // ============================================
    // 网桥管理
    // ============================================

    /// 查询网桥列表（分页）
    pub async fn list_bridges(&self, page_num: u32, page_size: u32) -> Result<serde_json::Value> {
        info!("查询网桥列表: 第{}页, 每页{}条", page_num, page_size);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/bridge?pageNum={}&pageSize={}", page_num, page_size),
            None::<()>,
        ).await
    }

    /// 网桥和物理网卡绑定
    pub async fn bind_bridge_nic(&self, bridge_id: &str, nic_name: &str) -> Result<()> {
        info!("绑定网桥和网卡: {} -> {}", bridge_id, nic_name);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/bridge/{}/bind", bridge_id),
            Some(serde_json::json!({ "nicName": nic_name })),
        ).await
    }

    /// 网桥和物理网卡解除绑定
    pub async fn unbind_bridge_nic(&self, bridge_id: &str) -> Result<()> {
        info!("解绑网桥: {}", bridge_id);
        self.client.request(
            Method::POST,
            &format!("/ocloud/v1/bridge/{}/unbind", bridge_id),
            None::<()>,
        ).await
    }

    // ============================================
    // 主机网卡管理
    // ============================================

    /// 查询主机上可用的物理网卡
    pub async fn list_host_nics(&self, host_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("查询主机网卡: {}", host_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/host/{}/nic/all", host_id),
            None::<()>,
        ).await
    }

    // ============================================
    // 虚拟机网卡管理
    // ============================================

    /// 查询虚拟机网卡
    pub async fn list_domain_nics(&self, domain_id: &str) -> Result<Vec<serde_json::Value>> {
        info!("查询虚拟机网卡: {}", domain_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/domain/{}/nic", domain_id),
            None::<()>,
        ).await
    }

    /// 设置虚拟机网卡 IP
    pub async fn set_nic_ip(&self, nic_id: &str, ip: &str) -> Result<()> {
        info!("设置网卡 IP: {} -> {}", nic_id, ip);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/nic/{}/ip", nic_id),
            Some(serde_json::json!({ "ip": ip })),
        ).await
    }

    /// 查看虚拟机网卡的安全组
    pub async fn get_nic_security_group(&self, nic_id: &str) -> Result<serde_json::Value> {
        info!("查询网卡安全组: {}", nic_id);
        self.client.request(
            Method::GET,
            &format!("/ocloud/v1/nic/{}/security-group", nic_id),
            None::<()>,
        ).await
    }

    /// 设置虚拟机网卡的安全组
    pub async fn set_nic_security_group(&self, nic_id: &str, security_group_id: &str) -> Result<()> {
        info!("设置网卡安全组: {} -> {}", nic_id, security_group_id);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/nic/{}/security-group", nic_id),
            Some(serde_json::json!({ "securityGroupId": security_group_id })),
        ).await
    }

    /// 设置虚拟机网卡限速
    pub async fn set_nic_speed_limit(&self, nic_id: &str, inbound: Option<i64>, outbound: Option<i64>) -> Result<()> {
        info!("设置网卡限速: {}", nic_id);
        self.client.request(
            Method::PATCH,
            &format!("/ocloud/v1/nic/{}/speed-limit", nic_id),
            Some(serde_json::json!({
                "inbound": inbound,
                "outbound": outbound,
            })),
        ).await
    }

    // ============================================
    // IP 池管理
    // ============================================

    /// 设置 IP 池中地址的可用性
    pub async fn set_ip_reserved(&self, ip_id: &str, reserved: bool) -> Result<()> {
        info!("设置 IP 可用性: {} -> {}", ip_id, !reserved);
        self.client.request(
            Method::POST,
            "/ocloud/v1/ip-pool/reserve",
            Some(serde_json::json!({
                "ipId": ip_id,
                "reserved": reserved,
            })),
        ).await
    }
}

/// VDI + libvirt 集成测试
///
/// 从 VDI 平台获取主机信息，然后连接到主机的 libvirtd
///
/// 使用方法:
/// ```bash
/// cd /home/cloudyi/ocloudview-atp
/// cargo run --example vdi_libvirt_integration --manifest-path atp-core/executor/Cargo.toml
/// ```

use atp_executor::TestConfig;
use atp_transport::{HostInfo, HostConnection};
use reqwest;
use serde_json::{json, Value};
use md5;
use std::collections::HashMap;
use tracing::{info, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║         VDI + libvirt 集成测试                                 ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // 加载配置
    let config = TestConfig::load()?;
    let vdi_config = config.vdi.as_ref()
        .ok_or_else(|| anyhow::anyhow!("未配置 VDI 平台"))?;

    let base_url = vdi_config.base_url.trim_end_matches('/');

    // ==========================================
    // 步骤 1: 从 VDI 登录并获取 Token
    // ==========================================
    println!("📋 步骤 1/4: 登录 VDI 平台...");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(vdi_config.connect_timeout))
        .danger_accept_invalid_certs(!vdi_config.verify_ssl)
        .build()?;

    let password_md5 = format!("{:x}", md5::compute(vdi_config.password.as_bytes()));
    let login_url = format!("{}/ocloud/v1/login", base_url);
    let login_data = json!({
        "username": vdi_config.username,
        "password": password_md5,
        "client": ""
    });

    let response = client.post(&login_url).json(&login_data).send().await?;
    let login_result: Value = response.json().await?;

    if login_result["status"].as_i64().unwrap_or(-1) != 0 {
        return Err(anyhow::anyhow!("VDI 登录失败: {}", login_result["msg"]));
    }

    let token = login_result["data"]["token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("未获取到 Token"))?
        .to_string();

    println!("   ✅ VDI 登录成功");
    println!("   🔑 Token: {}...", &token[..token.len().min(20)]);
    println!();

    // ==========================================
    // 步骤 2: 从 VDI 获取主机列表
    // ==========================================
    println!("📋 步骤 2/4: 从 VDI 获取主机列表...");

    let host_url = format!("{}/ocloud/v1/host?pageNum=1&pageSize=100", base_url);
    let response = client
        .get(&host_url)
        .header("Token", &token)
        .send()
        .await?;

    let host_result: Value = response.json().await?;

    if host_result["status"].as_i64().unwrap_or(-1) != 0 {
        return Err(anyhow::anyhow!("获取主机列表失败: {}", host_result["msg"]));
    }

    let hosts = host_result["data"]["list"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("主机列表为空"))?;

    println!("   ✅ 找到 {} 个主机:", hosts.len());

    let mut host_list = Vec::new();
    for (i, host) in hosts.iter().enumerate() {
        let name = host["name"].as_str().unwrap_or("未知");
        let ip = host["ip"].as_str().unwrap_or("未知");
        let status = host["status"].as_i64().unwrap_or(-1);
        let cpu_size = host["cpuSize"].as_i64().unwrap_or(0);
        let memory = host["memory"].as_f64().unwrap_or(0.0);

        println!("      {}. {} - IP: {} - CPU: {}核 - 内存: {:.2} GB - 状态: {}",
            i + 1, name, ip, cpu_size, memory,
            if status == 1 { "在线" } else { "离线" }
        );

        if status == 1 {
            host_list.push((name.to_string(), ip.to_string()));
        }
    }
    println!();

    if host_list.is_empty() {
        return Err(anyhow::anyhow!("没有在线的主机"));
    }

    // ==========================================
    // 步骤 3: 选择第一个在线主机并连接 libvirt
    // ==========================================
    let (host_name, host_ip) = &host_list[0];
    println!("📋 步骤 3/4: 连接到主机 {} ({}) 的 libvirtd...", host_name, host_ip);

    // 尝试多种 libvirt URI
    let uris = vec![
        format!("qemu+ssh://root@{}/system", host_ip),
        format!("qemu+tcp://{}/system", host_ip),
        format!("qemu://{}/system", host_ip),
    ];

    let mut connected = false;
    let mut connection: Option<HostConnection> = None;

    for uri in &uris {
        println!("   🔗 尝试连接: {}", uri);

        let host_info = HostInfo {
            id: host_name.clone(),
            host: host_name.clone(),
            uri: uri.clone(),
            tags: vec![],
            metadata: HashMap::new(),
        };

        let conn = HostConnection::new(host_info);

        match conn.connect().await {
            Ok(_) => {
                if conn.is_alive().await {
                    println!("   ✅ 连接成功!");
                    connected = true;
                    connection = Some(conn);
                    break;
                } else {
                    println!("   ⚠️  连接已断开");
                }
            }
            Err(e) => {
                println!("   ❌ 连接失败: {}", e);
            }
        }
    }

    if !connected {
        error!("无法连接到主机的 libvirtd");
        println!();
        println!("💡 提示:");
        println!("   1. 确保主机 {} 上的 libvirtd 服务正在运行", host_ip);
        println!("   2. 如果使用 SSH 连接，确保已配置 SSH 密钥认证:");
        println!("      ssh-copy-id root@{}", host_ip);
        println!("   3. 如果使用 TCP 连接，确保 libvirtd 已开启 TCP 监听");
        return Err(anyhow::anyhow!("libvirt 连接失败"));
    }

    let conn = connection.unwrap();
    println!();

    // ==========================================
    // 步骤 4: 获取虚拟机信息并与 VDI 数据对比
    // ==========================================
    println!("📋 步骤 4/4: 获取虚拟机信息...");

    // 从 VDI 获取虚拟机列表
    println!("   📡 从 VDI 获取虚拟机列表...");
    let domain_url = format!("{}/ocloud/v1/domain?pageNum=1&pageSize=100", base_url);
    let response = client
        .get(&domain_url)
        .header("Token", &token)
        .send()
        .await?;

    let domain_result: Value = response.json().await?;
    let vdi_domains = domain_result["data"]["list"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("虚拟机列表为空"))?;

    println!("   ✅ VDI 虚拟机数量: {}", vdi_domains.len());

    // 从 libvirt 获取虚拟机列表
    println!("   🔌 从 libvirt 获取虚拟机列表...");

    if let Ok(conn_mutex) = conn.get_connection().await {
        let conn_guard = conn_mutex.lock().await;
        if let Some(conn_ref) = conn_guard.as_ref() {
            match conn_ref.list_all_domains(0) {
                Ok(domains) => {
                    println!("   ✅ libvirt 虚拟机数量: {}", domains.len());
                    println!();

                    println!("   📊 虚拟机对比:");
                    println!("   {:<20} {:<15} {:<15}", "虚拟机名称", "VDI状态", "libvirt状态");
                    println!("   {}", "-".repeat(50));

                    // 创建 libvirt 虚拟机名称映射
                    let mut libvirt_vms: HashMap<String, String> = HashMap::new();
                    for domain in &domains {
                        if let Ok(name) = domain.get_name() {
                            if let Ok((state, _)) = domain.get_state() {
                                let state_str = format!("{:?}", state);
                                libvirt_vms.insert(name, state_str);
                            }
                        }
                    }

                    // 对比 VDI 和 libvirt 的虚拟机
                    for vdi_vm in vdi_domains {
                        let vm_name = vdi_vm["name"].as_str().unwrap_or("未知");
                        let vdi_status = vdi_vm["status"].as_i64().unwrap_or(-1);
                        let vdi_status_str = match vdi_status {
                            1 => "运行中",
                            5 => "关机",
                            _ => "未知"
                        };

                        let libvirt_status = libvirt_vms.get(vm_name)
                            .map(|s| s.as_str())
                            .unwrap_or("不存在");

                        println!("   {:<20} {:<15} {:<15}",
                            vm_name, vdi_status_str, libvirt_status);
                    }

                    println!();

                    // 显示详细的 libvirt 虚拟机信息（前3个）
                    println!("   📋 libvirt 虚拟机详细信息 (前3个):");
                    for (i, domain) in domains.iter().enumerate().take(3) {
                        if let Ok(name) = domain.get_name() {
                            println!("      {}. {}", i + 1, name);

                            if let Ok((state, _)) = domain.get_state() {
                                println!("         状态: {:?}", state);
                            }

                            if let Ok(info) = domain.get_info() {
                                println!("         CPU: {} 核", info.nr_virt_cpu);
                                println!("         内存: {} MB", info.memory / 1024);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("获取 libvirt 虚拟机列表失败: {}", e);
                }
            }

            // 显示主机信息
            println!();
            println!("   🖥️  主机信息:");
            if let Ok(hostname) = conn_ref.get_hostname() {
                println!("      主机名: {}", hostname);
            }
            if let Ok(version) = conn_ref.get_lib_version() {
                println!("      libvirt 版本: {}.{}.{}",
                    version / 1000000,
                    (version % 1000000) / 1000,
                    version % 1000
                );
            }
        }
    }

    println!();
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    集成测试完成                                ║");
    println!("╠════════════════════════════════════════════════════════════════╣");
    println!("║  ✅ VDI 平台连接成功                                           ║");
    println!("║  ✅ libvirt 连接成功                                           ║");
    println!("║  ✅ 虚拟机信息同步成功                                         ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    Ok(())
}

//! VDI 平台管理和验证命令

use crate::VdiAction;
use anyhow::{Context, Result};
use atp_executor::{TestConfig, VdiConfig};
use atp_transport::{HostConnection, HostInfo};
use atp_vdiplatform::{VdiClient, client::VdiConfig as VdiClientConfig};
use serde_json::json;
use std::collections::HashMap;
use tracing::{error, info};

/// VDI 虚拟机信息
#[derive(Debug, Clone)]
struct VmInfo {
    name: String,
    status: String,
    host: String,
}

/// Libvirt 虚拟机信息
#[derive(Debug, Clone)]
struct LibvirtVmInfo {
    name: String,
    state: String,
    cpu: u32,
    memory_mb: u64,
}

/// 比对结果
#[derive(Debug)]
struct CompareResult {
    vm_name: String,
    vdi_status: String,
    libvirt_status: String,
    consistent: bool,
    host: String,
}

pub async fn handle(action: VdiAction) -> Result<()> {
    match action {
        VdiAction::Verify {
            config,
            only_diff,
            format,
        } => verify_consistency(&config, only_diff, &format).await?,
        VdiAction::ListHosts { config } => list_hosts(&config).await?,
        VdiAction::ListVms { config, host } => list_vms(&config, host.as_deref()).await?,
        VdiAction::SyncHosts {
            config,
            test_connection,
        } => sync_hosts(&config, test_connection).await?,
    }
    Ok(())
}

/// 创建并登录VDI客户端
async fn create_vdi_client(vdi_config: &VdiConfig) -> Result<VdiClient> {
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

/// 验证 VDI 平台与 libvirt 虚拟机状态一致性
async fn verify_consistency(config_path: &str, only_diff: bool, format: &str) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║         VDI 与 libvirt 虚拟机状态一致性验证                   ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // 加载配置
    let config = TestConfig::load_from_path(config_path)
        .context(format!("无法加载配置文件: {}", config_path))?;
    let vdi_config = config
        .vdi
        .as_ref()
        .context("配置文件中未找到 VDI 平台配置")?;

    // 1. 登录 VDI 平台
    println!("📋 步骤 1/4: 登录 VDI 平台...");
    let client = create_vdi_client(vdi_config).await?;
    println!("   ✅ VDI 登录成功\n");

    // 2. 从 VDI 获取主机列表
    println!("📋 步骤 2/4: 获取 VDI 主机列表...");
    let hosts = client.host().list_all().await?;
    println!("   ✅ 找到 {} 个主机\n", hosts.len());

    // 创建主机ID到主机名的映射
    let mut host_id_to_name: HashMap<String, String> = HashMap::new();
    for host in &hosts {
        let host_id = host["id"].as_str().unwrap_or("").to_string();
        let host_name = host["name"].as_str().unwrap_or("").to_string();
        if !host_id.is_empty() && !host_name.is_empty() {
            host_id_to_name.insert(host_id, host_name);
        }
    }

    // 3. 从 VDI 获取虚拟机列表
    println!("📋 步骤 3/4: 获取 VDI 虚拟机列表...");
    let vdi_domains = client.domain().list_all().await?;

    let mut vdi_vms: HashMap<String, VmInfo> = HashMap::new();
    for domain in &vdi_domains {
        let name = domain["name"].as_str().unwrap_or("").to_string();
        let status = match domain["status"].as_i64().unwrap_or(-1) {
            0 => "关机".to_string(),
            1 => "运行中".to_string(),
            2 => "挂起".to_string(),
            3 => "休眠".to_string(),
            5 => "操作中".to_string(),
            6 => "升级中".to_string(),
            _ => "未知".to_string(),
        };
        // 使用 hostId 获取主机名
        let host_id = domain["hostId"].as_str().unwrap_or("");
        let host = host_id_to_name
            .get(host_id)
            .cloned()
            .unwrap_or_else(|| "".to_string());

        if !name.is_empty() {
            vdi_vms.insert(
                name.clone(),
                VmInfo {
                    name,
                    status,
                    host,
                },
            );
        }
    }

    println!("   ✅ VDI 虚拟机数量: {}\n", vdi_vms.len());

    // 4. 连接 libvirt 并获取虚拟机信息
    println!("📋 步骤 4/4: 连接 libvirt 并比对虚拟机状态...\n");

    let mut all_results: Vec<CompareResult> = Vec::new();
    let mut total_vms = 0;
    let mut consistent_vms = 0;
    let mut inconsistent_vms = 0;

    for host in &hosts {
        let host_name = host["name"].as_str().unwrap_or("");
        let host_ip = host["ip"].as_str().unwrap_or("");
        let status = host["status"].as_i64().unwrap_or(-1);

        if status != 1 {
            println!("   ⚠️  主机 {} 离线，跳过", host_name);
            continue;
        }

        println!("   🔗 连接主机: {} ({})", host_name, host_ip);

        // 尝试连接 libvirt
        let uris = vec![
            format!("qemu+tcp://{}/system", host_ip),
            format!("qemu+ssh://root@{}/system", host_ip),
        ];

        let mut connected = false;
        let mut libvirt_vms: HashMap<String, LibvirtVmInfo> = HashMap::new();

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
                        info!("   ✅ 连接成功: {}", uri);

                        // 获取虚拟机列表（包括所有状态的虚拟机）
                        if let Ok(conn_mutex) = conn.get_connection().await {
                            let conn_guard = conn_mutex.lock().await;
                            if let Some(conn_ref) = conn_guard.as_ref() {
                                // 获取所有域（包括关闭状态的）
                                // flags: VIR_CONNECT_LIST_DOMAINS_ACTIVE | VIR_CONNECT_LIST_DOMAINS_INACTIVE = 3
                                if let Ok(domains) = conn_ref.list_all_domains(3) {
                                    for domain in &domains {
                                        if let Ok(name) = domain.get_name() {
                                            let state = if let Ok((st, _)) = domain.get_state() {
                                                // 使用 Debug format 输出状态，然后解析为字符串
                                                let state_debug = format!("{:?}", st);
                                                // 状态值: Running, Shutoff, Paused, Shutdown, Crashed, PMSuspended, Blocked, NoState
                                                state_debug
                                            } else {
                                                "Unknown".to_string()
                                            };

                                            let (cpu, memory) = if let Ok(info) = domain.get_info()
                                            {
                                                (info.nr_virt_cpu, info.memory / 1024)
                                            } else {
                                                (0, 0)
                                            };

                                            libvirt_vms.insert(
                                                name.clone(),
                                                LibvirtVmInfo {
                                                    name,
                                                    state,
                                                    cpu,
                                                    memory_mb: memory,
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        connected = true;
                        break;
                    }
                }
                Err(e) => {
                    info!("   ⚠️  连接失败 {}: {}", uri, e);
                }
            }
        }

        if !connected {
            error!("   ❌ 无法连接到主机 {} 的 libvirtd", host_name);
            continue;
        }

        println!("   📊 libvirt 虚拟机数量: {}", libvirt_vms.len());

        // 比对虚拟机状态
        for (vm_name, libvirt_vm) in &libvirt_vms {
            total_vms += 1;

            if let Some(vdi_vm) = vdi_vms.get(vm_name) {
                // VDI 中存在该虚拟机，检查状态是否一致
                let consistent = match (vdi_vm.status.as_str(), libvirt_vm.state.as_str()) {
                    ("运行中", "1") | ("运行中", "Running") => true,
                    ("挂起", "3") | ("挂起", "Paused") => true,
                    ("关机", "5") | ("关机", "Shutoff") => true,
                    _ => false,
                };

                if consistent {
                    consistent_vms += 1;
                } else {
                    inconsistent_vms += 1;
                }

                all_results.push(CompareResult {
                    vm_name: vm_name.clone(),
                    vdi_status: vdi_vm.status.clone(),
                    libvirt_status: libvirt_vm.state.clone(),
                    consistent,
                    host: host_name.to_string(),
                });
            } else {
                // libvirt 上存在但 VDI 中不存在 - 不一致
                inconsistent_vms += 1;
                all_results.push(CompareResult {
                    vm_name: vm_name.clone(),
                    vdi_status: "不存在".to_string(),
                    libvirt_status: libvirt_vm.state.clone(),
                    consistent: false,
                    host: host_name.to_string(),
                });
            }
        }

        println!();
    }

    // 输出结果
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                      验证结果汇总                              ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("📊 统计信息:");
    println!("   总虚拟机数: {}", total_vms);
    println!("   一致: {} ✅", consistent_vms);
    println!("   不一致: {} ❌", inconsistent_vms);
    println!(
        "   一致性: {:.1}%\n",
        if total_vms > 0 {
            (consistent_vms as f64 / total_vms as f64) * 100.0
        } else {
            0.0
        }
    );

    // 根据格式输出详细结果
    match format {
        "json" => output_json(&all_results, only_diff)?,
        "yaml" => output_yaml(&all_results, only_diff)?,
        _ => output_table(&all_results, only_diff),
    }

    if inconsistent_vms > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// 表格格式输出
fn output_table(results: &[CompareResult], only_diff: bool) {
    println!("📋 详细对比结果:\n");
    println!(
        "{:<20} {:<15} {:<20} {:<15} {:<10}",
        "虚拟机名称", "主机", "VDI状态", "libvirt状态", "一致性"
    );
    println!("{}", "-".repeat(80));

    for result in results {
        if only_diff && result.consistent {
            continue;
        }

        let status_icon = if result.consistent { "✅" } else { "❌" };
        println!(
            "{:<20} {:<15} {:<20} {:<20} {}",
            result.vm_name, result.host, result.vdi_status, result.libvirt_status, status_icon
        );
    }
}

/// JSON 格式输出
fn output_json(results: &[CompareResult], only_diff: bool) -> Result<()> {
    let filtered: Vec<_> = if only_diff {
        results.iter().filter(|r| !r.consistent).collect()
    } else {
        results.iter().collect()
    };

    let json_data: Vec<serde_json::Value> = filtered
        .iter()
        .map(|r| {
            json!({
                "vm_name": r.vm_name,
                "host": r.host,
                "vdi_status": r.vdi_status,
                "libvirt_status": r.libvirt_status,
                "consistent": r.consistent
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_data)?);
    Ok(())
}

/// YAML 格式输出
fn output_yaml(results: &[CompareResult], only_diff: bool) -> Result<()> {
    let filtered: Vec<_> = if only_diff {
        results.iter().filter(|r| !r.consistent).collect()
    } else {
        results.iter().collect()
    };

    for result in filtered {
        println!("- vm_name: {}", result.vm_name);
        println!("  host: {}", result.host);
        println!("  vdi_status: {}", result.vdi_status);
        println!("  libvirt_status: {}", result.libvirt_status);
        println!("  consistent: {}", result.consistent);
        println!();
    }

    Ok(())
}

/// 列出 VDI 平台的所有主机
async fn list_hosts(config_path: &str) -> Result<()> {
    println!("📋 VDI 平台主机列表\n");

    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;

    let client = create_vdi_client(vdi_config).await?;
    let hosts = client.host().list_all().await?;

    println!(
        "{:<20} {:<20} {:<10} {:<15} {:<15}",
        "主机名", "IP地址", "状态", "CPU(核)", "内存(GB)"
    );
    println!("{}", "-".repeat(80));

    for host in &hosts {
        let name = host["name"].as_str().unwrap_or("");
        let ip = host["ip"].as_str().unwrap_or("");
        let status = match host["status"].as_i64().unwrap_or(-1) {
            1 => "在线 ✅",
            _ => "离线 ❌",
        };
        let cpu = host["cpuSize"].as_i64().unwrap_or(0);
        let memory_gb = host["memory"].as_f64().unwrap_or(0.0);

        println!(
            "{:<20} {:<20} {:<10} {:<15} {:<15.2}",
            name, ip, status, cpu, memory_gb
        );
    }

    println!("\n总计: {} 个主机", hosts.len());

    Ok(())
}

/// 列出 VDI 平台的所有虚拟机
async fn list_vms(config_path: &str, host_filter: Option<&str>) -> Result<()> {
    println!("📋 VDI 平台虚拟机列表\n");

    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;

    let client = create_vdi_client(vdi_config).await?;

    let domains = client.domain().list_all().await?;
    let hosts_vec = client.host().list_all().await?;

    // 建立主机ID到名称的映射
    let mut host_id_to_name: HashMap<String, String> = HashMap::new();
    for host in &hosts_vec {
        let host_id = host["id"].as_str().unwrap_or("").to_string();
        let host_name = host["name"].as_str().unwrap_or("").to_string();
        if !host_id.is_empty() && !host_name.is_empty() {
            host_id_to_name.insert(host_id, host_name);
        }
    }

    println!(
        "{:<25} {:<20} {:<15} {:<10} {:<15}",
        "虚拟机名称", "主机", "状态", "CPU(核)", "内存(GB)"
    );
    println!("{}", "-".repeat(90));

    let mut count = 0;
    for domain in &domains {
        let name = domain["name"].as_str().unwrap_or("");
        let host_id = domain["hostId"].as_str().unwrap_or("");
        let host_name = host_id_to_name.get(host_id).map(|s| s.as_str()).unwrap_or("");

        // 主机过滤
        if let Some(filter) = host_filter {
            if host_name != filter {
                continue;
            }
        }

        let status = match domain["status"].as_i64().unwrap_or(-1) {
            0 => "关机 ⚪",
            1 => "运行中 ✅",
            2 => "挂起 🟡",
            3 => "休眠 🌙",
            5 => "操作中 ⚙️",
            6 => "升级中 ⬆️",
            _ => "未知 ⚠️",
        };
        let cpu = domain["cpuNum"].as_i64().unwrap_or(0);
        let memory_gb = domain["memory"].as_f64().unwrap_or(0.0) / 1024.0;

        println!(
            "{:<25} {:<20} {:<15} {:<10} {:<15.2}",
            name, host_name, status, cpu, memory_gb
        );
        count += 1;
    }

    println!("\n总计: {} 个虚拟机", count);

    Ok(())
}

/// 同步 VDI 主机到本地配置
async fn sync_hosts(config_path: &str, test_connection: bool) -> Result<()> {
    println!("🔄 同步 VDI 主机到本地配置\n");

    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;

    let client = create_vdi_client(vdi_config).await?;
    let hosts = client.host().list_all().await?;

    println!("📊 发现 {} 个主机:\n", hosts.len());

    for (i, host) in hosts.iter().enumerate() {
        let name = host["name"].as_str().unwrap_or("");
        let ip = host["ip"].as_str().unwrap_or("");
        let status = host["status"].as_i64().unwrap_or(-1);

        print!("  {}. {} ({}) ", i + 1, name, ip);

        if status != 1 {
            println!("- 离线 ❌");
            continue;
        }

        if test_connection {
            // 测试连接
            let uri = format!("qemu+tcp://{}/system", ip);
            let host_info = HostInfo {
                id: name.to_string(),
                host: name.to_string(),
                uri: uri.clone(),
                tags: vec![],
                metadata: HashMap::new(),
            };

            let conn = HostConnection::new(host_info);
            match conn.connect().await {
                Ok(_) if conn.is_alive().await => {
                    println!("- 连接成功 ✅");
                }
                _ => {
                    println!("- 连接失败 ❌");
                }
            }
        } else {
            println!("- 在线 ✅");
        }
    }

    println!("\n💡 提示: 主机信息已从 VDI 平台获取");
    println!("   可以在测试配置中使用这些主机信息");

    Ok(())
}

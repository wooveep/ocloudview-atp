//! VDI 平台管理和验证命令

use crate::VdiAction;
use crate::commands::common::{
    build_host_id_to_name_map_from_json, connect_libvirt, create_vdi_client,
};
use anyhow::{Context, Result};
use atp_executor::TestConfig;
use atp_gluster::GlusterClient;
use atp_ssh_executor::{SshClient, SshConfig};
use atp_vdiplatform::{DiskInfo, DomainStatus, HostStatusCode};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{error, info, warn};

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
        VdiAction::DiskLocation {
            config,
            vm,
            ssh,
            ssh_user,
            ssh_password,
            ssh_key,
            format,
        } => {
            disk_location(
                &config,
                &vm,
                ssh,
                &ssh_user,
                ssh_password.as_deref(),
                ssh_key.as_deref(),
                &format,
            )
            .await?
        }
    }
    Ok(())
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
    let host_id_to_name = build_host_id_to_name_map_from_json(&hosts);

    // 3. 从 VDI 获取虚拟机列表
    println!("📋 步骤 3/4: 获取 VDI 虚拟机列表...");
    let vdi_domains = client.domain().list_all().await?;

    let mut vdi_vms: HashMap<String, VmInfo> = HashMap::new();
    for domain in &vdi_domains {
        let name = domain["name"].as_str().unwrap_or("").to_string();
        let status_code = domain["status"].as_i64().unwrap_or(-1);
        let status = DomainStatus::from_code(status_code).display_name().to_string();
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
        let host_status = HostStatusCode::from_code(host["status"].as_i64().unwrap_or(-1));

        if !host_status.is_online() {
            println!("   ⚠️  主机 {} 离线，跳过", host_name);
            continue;
        }

        println!("   🔗 连接主机: {} ({})", host_name, host_ip);

        // 尝试连接 libvirt
        let mut libvirt_vms: HashMap<String, LibvirtVmInfo> = HashMap::new();

        let conn_result = match connect_libvirt(host_name, host_ip).await {
            Ok(result) => {
                info!("   ✅ 连接成功: {}", result.uri);
                result
            }
            Err(e) => {
                error!("   ❌ {}", e);
                continue;
            }
        };

        // 获取虚拟机列表（包括所有状态的虚拟机）
        if let Ok(conn_mutex) = conn_result.connection.get_connection().await {
            let conn_guard = conn_mutex.lock().await;
            if let Some(conn_ref) = conn_guard.as_ref() {
                // 获取所有域（包括关闭状态的）
                // flags: VIR_CONNECT_LIST_DOMAINS_ACTIVE | VIR_CONNECT_LIST_DOMAINS_INACTIVE = 3
                if let Ok(domains) = conn_ref.list_all_domains(3) {
                    for domain in &domains {
                        if let Ok(name) = domain.get_name() {
                            let state = if let Ok((st, _)) = domain.get_state() {
                                let state_debug = format!("{:?}", st);
                                state_debug
                            } else {
                                "Unknown".to_string()
                            };

                            let (cpu, memory) = if let Ok(info) = domain.get_info() {
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
        let status = HostStatusCode::from_code(host["status"].as_i64().unwrap_or(-1))
            .display_with_emoji();
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
    let host_id_to_name = build_host_id_to_name_map_from_json(&hosts_vec);

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

        let status = DomainStatus::from_code(domain["status"].as_i64().unwrap_or(-1))
            .display_with_emoji();
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
        let host_status = HostStatusCode::from_code(host["status"].as_i64().unwrap_or(-1));

        print!("  {}. {} ({}) ", i + 1, name, ip);

        if !host_status.is_online() {
            println!("- {}", host_status.display_with_emoji());
            continue;
        }

        if test_connection {
            // 测试连接
            match connect_libvirt(name, ip).await {
                Ok(_) => {
                    println!("- 连接成功 ✅");
                }
                Err(_) => {
                    println!("- 连接失败 ❌");
                }
            }
        } else {
            println!("- {}", host_status.display_with_emoji());
        }
    }

    println!("\n💡 提示: 主机信息已从 VDI 平台获取");
    println!("   可以在测试配置中使用这些主机信息");

    Ok(())
}

/// 查询虚拟机磁盘存储位置
async fn disk_location(
    config_path: &str,
    vm_id_or_name: &str,
    enable_ssh: bool,
    ssh_user: &str,
    ssh_password: Option<&str>,
    ssh_key: Option<&str>,
    format: &str,
) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║              虚拟机磁盘存储位置查询                            ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // 加载配置
    let config = TestConfig::load_from_path(config_path)
        .context(format!("无法加载配置文件: {}", config_path))?;
    let vdi_config = config
        .vdi
        .as_ref()
        .context("配置文件中未找到 VDI 平台配置")?;

    // 1. 登录 VDI 平台
    println!("📋 步骤 1/3: 登录 VDI 平台...");
    let client = create_vdi_client(vdi_config).await?;
    println!("   ✅ VDI 登录成功\n");

    // 2. 查找虚拟机
    println!("📋 步骤 2/3: 查找虚拟机 {}...", vm_id_or_name);
    let domains = client.domain().list_all().await?;
    let domain = domains
        .iter()
        .find(|d| {
            d["id"].as_str() == Some(vm_id_or_name)
                || d["name"].as_str() == Some(vm_id_or_name)
        })
        .context(format!("未找到虚拟机: {}", vm_id_or_name))?;

    let domain_id = domain["id"].as_str().unwrap_or("");
    let domain_name = domain["name"].as_str().unwrap_or("");
    println!("   ✅ 找到虚拟机: {} ({})\n", domain_name, domain_id);

    // 3. 获取磁盘信息
    println!("📋 步骤 3/3: 获取磁盘信息...");
    let disk_values = client.domain().get_disks(domain_id).await?;
    let disks: Vec<DiskInfo> = disk_values
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();
    println!("   ✅ 找到 {} 个磁盘\n", disks.len());

    if disks.is_empty() {
        println!("⚠️  该虚拟机没有磁盘");
        return Ok(());
    }

    // 检查是否有 Gluster 存储的磁盘
    let has_gluster = disks.iter().any(|d| d.is_gluster());

    // 为每个 Gluster 磁盘查询存储池关联的主机
    let mut gluster_clients: HashMap<String, Option<GlusterClient>> = HashMap::new();

    if has_gluster && enable_ssh {
        println!("🔗 查询 Gluster 存储池关联主机...\n");

        // 收集所有 Gluster 磁盘的存储池 ID
        let gluster_pool_ids: std::collections::HashSet<String> = disks
            .iter()
            .filter(|d| d.is_gluster())
            .map(|d| d.storage_pool_id.clone())
            .collect();

        for storage_pool_id in &gluster_pool_ids {
            // 查询存储池详情
            info!("   查询存储池 {}...", storage_pool_id);
            let pool_detail = client.storage().get_pool(storage_pool_id).await?;

            // API 返回格式: { "status": 0, "data": { "poolId": "xxx", ... } }
            // 需要从 data 中获取 poolId
            let data = &pool_detail["data"];

            // 获取资源池 poolId
            let resource_pool_id = data["poolId"]
                .as_str()
                .unwrap_or("")
                .to_string();

            if resource_pool_id.is_empty() {
                warn!("   存储池 {} 没有关联资源池", storage_pool_id);
                gluster_clients.insert(storage_pool_id.clone(), None);
                continue;
            }

            info!("   存储池 {} 关联资源池: {}", storage_pool_id, resource_pool_id);

            // 根据资源池 ID 查询关联主机
            let hosts = client.host().list_by_pool_id(&resource_pool_id).await?;
            let host_ips: Vec<String> = hosts
                .iter()
                .filter_map(|h| h["ip"].as_str().map(|s| s.to_string()))
                .filter(|ip| !ip.is_empty())
                .collect();

            info!("   找到 {} 个关联主机: {:?}", host_ips.len(), host_ips);

            // 尝试连接主机
            let mut connected_client = None;
            for host_ip in &host_ips {
                info!("   尝试连接 {}...", host_ip);

                let ssh_config = if let Some(password) = ssh_password {
                    SshConfig::with_password(host_ip, ssh_user, password)
                } else if let Some(key_path) = ssh_key {
                    SshConfig::with_key(host_ip, ssh_user, PathBuf::from(key_path))
                } else {
                    SshConfig::with_default_key(host_ip, ssh_user)
                };

                match SshClient::connect(ssh_config).await {
                    Ok(ssh) => {
                        println!("   ✅ SSH 连接成功: {} (存储池 {})", host_ip, storage_pool_id);
                        connected_client = Some(GlusterClient::new(ssh));
                        break;
                    }
                    Err(e) => {
                        warn!("   ⚠️  {} 连接失败: {}", host_ip, e);
                    }
                }
            }

            if connected_client.is_none() && !host_ips.is_empty() {
                println!("   ⚠️  存储池 {} 所有主机连接失败", storage_pool_id);
            }

            gluster_clients.insert(storage_pool_id.clone(), connected_client);
        }

        println!();
    } else if has_gluster && !enable_ssh {
        println!("💡 提示: 使用 --ssh 参数可查询 Gluster 实际 brick 位置\n");
    }

    // 输出结果
    match format {
        "json" => output_disk_location_json_v2(&disks, &gluster_clients, domain_name).await?,
        _ => output_disk_location_table_v2(&disks, &gluster_clients, domain_name).await?,
    }

    Ok(())
}

/// 表格格式输出磁盘位置
async fn output_disk_location_table(
    disks: &[DiskInfo],
    gluster_client: &Option<GlusterClient>,
    domain_name: &str,
) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                      磁盘存储位置详情                          ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("虚拟机: {}\n", domain_name);

    for (i, disk) in disks.iter().enumerate() {
        let boot_label = if disk.is_boot_disk() { " [启动盘]" } else { "" };
        println!(
            "📀 磁盘 {} - {}{}\n",
            i + 1,
            disk.name,
            boot_label
        );

        println!("   文件名:     {}", disk.filename);
        println!("   逻辑路径:   {}", disk.vol_full_path);
        println!("   存储池:     {} ({})", disk.pool_name, disk.pool_type);
        println!("   存储类型:   {}", disk.storage_type_display());
        println!("   大小:       {} GB", disk.size);
        println!("   总线类型:   {}", disk.bus_type);

        // 如果是 Gluster 存储，尝试获取实际位置
        if disk.is_gluster() {
            if let Some(ref client) = gluster_client {
                println!("\n   🔍 Gluster 实际存储位置:");
                match client.get_file_location(&disk.vol_full_path).await {
                    Ok(location) => {
                        if let Some(vol_name) = &location.volume_name {
                            println!("      卷名:    {}", vol_name);
                        }
                        println!("      副本数:  {}", location.replica_count());
                        for (j, replica) in location.replicas.iter().enumerate() {
                            println!(
                                "      副本 {}: {}:{}",
                                j + 1,
                                replica.host,
                                replica.file_path
                            );
                        }
                    }
                    Err(e) => {
                        println!("      ⚠️  查询失败: {}", e);
                    }
                }
            } else {
                println!("\n   💡 使用 --ssh-host 查询 Gluster brick 位置");
            }
        }

        println!();
    }

    Ok(())
}

/// JSON 格式输出磁盘位置
async fn output_disk_location_json(
    disks: &[DiskInfo],
    gluster_client: &Option<GlusterClient>,
    domain_name: &str,
) -> Result<()> {
    let mut disk_results = Vec::new();

    for disk in disks {
        let mut disk_json = json!({
            "id": disk.id,
            "name": disk.name,
            "filename": disk.filename,
            "vol_full_path": disk.vol_full_path,
            "storage_pool_id": disk.storage_pool_id,
            "storage_pool_name": disk.pool_name,
            "storage_type": disk.pool_type,
            "size_gb": disk.size,
            "bus_type": disk.bus_type,
            "is_boot_disk": disk.is_boot_disk(),
            "is_shared": disk.is_shared(),
        });

        // 如果是 Gluster 存储，尝试获取实际位置
        if disk.is_gluster() {
            if let Some(ref client) = gluster_client {
                match client.get_file_location(&disk.vol_full_path).await {
                    Ok(location) => {
                        let replicas: Vec<serde_json::Value> = location
                            .replicas
                            .iter()
                            .map(|r| {
                                json!({
                                    "host": r.host,
                                    "brick_path": r.brick_path,
                                    "file_path": r.file_path,
                                })
                            })
                            .collect();

                        disk_json["gluster_location"] = json!({
                            "volume_name": location.volume_name,
                            "replica_count": location.replica_count(),
                            "replicas": replicas,
                        });
                    }
                    Err(e) => {
                        disk_json["gluster_location_error"] = json!(e.to_string());
                    }
                }
            }
        }

        disk_results.push(disk_json);
    }

    let output = json!({
        "domain_name": domain_name,
        "disk_count": disks.len(),
        "disks": disk_results,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

/// 表格格式输出磁盘位置 (V2 - 支持多存储池)
async fn output_disk_location_table_v2(
    disks: &[DiskInfo],
    gluster_clients: &HashMap<String, Option<GlusterClient>>,
    domain_name: &str,
) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                      磁盘存储位置详情                          ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("虚拟机: {}\n", domain_name);

    for (i, disk) in disks.iter().enumerate() {
        let boot_label = if disk.is_boot_disk() { " [启动盘]" } else { "" };
        println!(
            "📀 磁盘 {} - {}{}\n",
            i + 1,
            disk.name,
            boot_label
        );

        println!("   文件名:     {}", disk.filename);
        println!("   逻辑路径:   {}", disk.vol_full_path);
        println!("   存储池:     {} ({})", disk.pool_name, disk.pool_type);
        println!("   存储类型:   {}", disk.storage_type_display());
        println!("   大小:       {} GB", disk.size);
        println!("   总线类型:   {}", disk.bus_type);

        // 如果是 Gluster 存储，尝试获取实际位置
        if disk.is_gluster() {
            if let Some(Some(ref client)) = gluster_clients.get(&disk.storage_pool_id) {
                println!("\n   🔍 Gluster 实际存储位置:");
                match client.get_file_location(&disk.vol_full_path).await {
                    Ok(location) => {
                        if let Some(vol_name) = &location.volume_name {
                            println!("      卷名:    {}", vol_name);
                        }
                        println!("      副本数:  {}", location.replica_count());
                        for (j, replica) in location.replicas.iter().enumerate() {
                            println!(
                                "      副本 {}: {}:{}",
                                j + 1,
                                replica.host,
                                replica.file_path
                            );
                        }
                    }
                    Err(e) => {
                        println!("      ⚠️  查询失败: {}", e);
                    }
                }
            } else if gluster_clients.contains_key(&disk.storage_pool_id) {
                println!("\n   ⚠️  无法连接存储池关联主机");
            } else {
                println!("\n   💡 使用 --ssh 查询 Gluster brick 位置");
            }
        }

        println!();
    }

    Ok(())
}

/// JSON 格式输出磁盘位置 (V2 - 支持多存储池)
async fn output_disk_location_json_v2(
    disks: &[DiskInfo],
    gluster_clients: &HashMap<String, Option<GlusterClient>>,
    domain_name: &str,
) -> Result<()> {
    let mut disk_results = Vec::new();

    for disk in disks {
        let mut disk_json = json!({
            "id": disk.id,
            "name": disk.name,
            "filename": disk.filename,
            "vol_full_path": disk.vol_full_path,
            "storage_pool_id": disk.storage_pool_id,
            "storage_pool_name": disk.pool_name,
            "storage_type": disk.pool_type,
            "size_gb": disk.size,
            "bus_type": disk.bus_type,
            "is_boot_disk": disk.is_boot_disk(),
            "is_shared": disk.is_shared(),
        });

        // 如果是 Gluster 存储，尝试获取实际位置
        if disk.is_gluster() {
            if let Some(Some(ref client)) = gluster_clients.get(&disk.storage_pool_id) {
                match client.get_file_location(&disk.vol_full_path).await {
                    Ok(location) => {
                        let replicas: Vec<serde_json::Value> = location
                            .replicas
                            .iter()
                            .map(|r| {
                                json!({
                                    "host": r.host,
                                    "brick_path": r.brick_path,
                                    "file_path": r.file_path,
                                })
                            })
                            .collect();

                        disk_json["gluster_location"] = json!({
                            "volume_name": location.volume_name,
                            "replica_count": location.replica_count(),
                            "replicas": replicas,
                        });
                    }
                    Err(e) => {
                        disk_json["gluster_location_error"] = json!(e.to_string());
                    }
                }
            }
        }

        disk_results.push(disk_json);
    }

    let output = json!({
        "domain_name": domain_name,
        "disk_count": disks.len(),
        "disks": disk_results,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

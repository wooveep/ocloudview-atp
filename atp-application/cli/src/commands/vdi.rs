//! VDI 平台管理和验证命令

use crate::commands::common::{connect_libvirt, create_vdi_client};
use crate::commands::output::{output_formatted, TableRow};
use crate::VdiAction;
use anyhow::{bail, Context, Result};
use atp_executor::{
    AffectedVm,
    AutoReplicaSelector,
    BatchAssignResult,
    BatchAutoAdResult,
    BatchOpError,
    // 新增：批量操作结果
    BatchOperations,
    BatchRenameResult,
    CompareResult,
    // 新增：存储操作服务
    DiskLocationInfo,
    DiskLocationResult,
    HealReport,
    HealStrategy,
    InteractiveReplicaSelector,
    ReplicaStat,
    SshConnectionManager,
    SshParams,
    StorageOpsService,
    TestConfig,
    VdiBatchOps,
    VdiVerifyOps,
};
use atp_gluster::SplitBrainEntry;
use atp_ssh_executor::{SshClient, SshConfig};
use atp_storage::{Storage, StorageManager};
use atp_transport::TransportManager;
use atp_vdiplatform::{AssignmentPlan, DiskInfo, DomainStatus, HostStatusCode, RenamePlan};
use serde_json::json;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

// Implement TableRow for CompareResult to support table output
impl TableRow for CompareResult {
    fn headers() -> Vec<&'static str> {
        vec!["虚拟机名称", "主机", "VDI状态", "libvirt状态", "一致性"]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.vm_name.clone(),
            self.host.clone(),
            self.vdi_status.clone(),
            self.libvirt_status.clone(),
            if self.consistent {
                "✅".to_string()
            } else {
                "❌".to_string()
            },
        ]
    }
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
        VdiAction::SyncVms { config } => sync_vms(&config).await?,
        VdiAction::SyncAll { config } => sync_all(&config).await?,
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
        VdiAction::Start {
            config,
            pattern,
            dry_run,
            verify,
            format,
        } => batch_start_vms(&config, &pattern, dry_run, verify, &format).await?,
        VdiAction::Assign {
            config,
            pattern,
            users,
            group,
            dry_run,
            force,
            format,
        } => {
            batch_assign_vms(
                &config,
                &pattern,
                users.as_deref(),
                group.as_deref(),
                force,
                dry_run,
                &format,
            )
            .await?
        }
        VdiAction::Rename {
            config,
            pattern,
            dry_run,
            format,
        } => batch_rename_vms(&config, &pattern, dry_run, &format).await?,
        VdiAction::AutoAd {
            config,
            pattern,
            enable,
            disable,
            dry_run,
            format,
        } => batch_set_auto_ad(&config, &pattern, enable, disable, dry_run, &format).await?,
        VdiAction::HealSplitbrain {
            config,
            pool_id,
            ssh,
            ssh_user,
            ssh_password,
            ssh_key,
            dry_run,
            auto,
            format,
        } => {
            heal_splitbrain(
                &config,
                pool_id.as_deref(),
                ssh,
                &ssh_user,
                ssh_password.as_deref(),
                ssh_key.as_deref(),
                dry_run,
                auto,
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

    // 初始化客户端和管理器
    let client = Arc::new(create_vdi_client(vdi_config).await?);
    let transport_manager = Arc::new(TransportManager::default());

    // 创建验证操作对象
    let verify_ops = VdiVerifyOps::new(transport_manager, client);

    // 执行验证
    let verify_result = verify_ops.verify_consistency().await?;

    // 统计结果
    let total_vms = verify_result.total_vms;
    let consistent_vms = verify_result.consistent_vms;
    let inconsistent_vms = verify_result.inconsistent_vms;

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

    // 输出详细结果
    let filter = if only_diff {
        Some(&(|r: &CompareResult| !r.consistent) as &dyn Fn(&CompareResult) -> bool)
    } else {
        None
    };

    output_formatted(&verify_result.results, format, filter)?;

    if inconsistent_vms > 0 {
        std::process::exit(1);
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
        let status =
            HostStatusCode::from_code(host["status"].as_i64().unwrap_or(-1)).display_with_emoji();
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

    let client = Arc::new(create_vdi_client(vdi_config).await?);

    let domains = client.domain().list_all().await?;

    // 建立主机ID到名称的映射
    let transport_manager = Arc::new(TransportManager::default());
    let batch_ops = VdiBatchOps::new(Arc::clone(&transport_manager), Arc::clone(&client));
    let host_id_to_name = batch_ops.build_host_id_to_name_map().await?;

    println!(
        "{:<25} {:<20} {:<15} {:<10} {:<15}",
        "虚拟机名称", "主机", "状态", "CPU(核)", "内存(GB)"
    );
    println!("{}", "-".repeat(90));

    let mut count = 0;
    for domain in &domains {
        let name = domain["name"].as_str().unwrap_or("");
        let host_id = domain["hostId"].as_str().unwrap_or("");
        let host_name = host_id_to_name
            .get(host_id)
            .map(|s| s.as_str())
            .unwrap_or("");

        // 主机过滤
        if let Some(filter) = host_filter {
            if host_name != filter {
                continue;
            }
        }

        let status =
            DomainStatus::from_code(domain["status"].as_i64().unwrap_or(-1)).display_with_emoji();
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
    use atp_storage::{StorageManager, VdiCacheManager};

    println!("🔄 同步 VDI 主机到数据库\n");

    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;

    let client = create_vdi_client(vdi_config).await?;
    let hosts = client.host().list_all().await?;

    println!("📊 发现 {} 个主机:\n", hosts.len());

    // 连接数据库并创建缓存管理器
    let storage_manager = StorageManager::new("~/.config/atp/data.db")
        .await
        .context("无法连接数据库")?;
    let cache = VdiCacheManager::new(storage_manager);

    // 使用缓存管理器同步主机（包含完整的 22 个 VDI 字段）
    let saved_count = cache.sync_hosts(&hosts).await?;

    // 显示主机列表并可选测试连接
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
            println!("- {} [已同步]", host_status.display_with_emoji());
        }
    }

    println!(
        "\n✅ 已同步 {} 个主机到数据库（包含完整 VDI 字段）",
        saved_count
    );
    println!("💡 提示: 使用 `atp host update-ssh <id>` 更新主机 SSH 配置");

    Ok(())
}

/// 同步 VDI 虚拟机到本地缓存
async fn sync_vms(config_path: &str) -> Result<()> {
    use atp_storage::{StorageManager, VdiCacheManager};

    println!("🔄 同步 VDI 虚拟机到本地缓存\n");

    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;

    let client = create_vdi_client(vdi_config).await?;

    println!("📋 获取 VDI 虚拟机列表...");
    let domains = client.domain().list_all().await?;
    println!("   发现 {} 个虚拟机\n", domains.len());

    // 连接数据库并创建缓存管理器
    let storage_manager = StorageManager::new("~/.config/atp/data.db")
        .await
        .context("无法连接数据库")?;
    let cache = VdiCacheManager::new(storage_manager);

    // 使用缓存管理器同步虚拟机（包含完整的 60 个 VDI 字段）
    let saved_count = cache.sync_domains(&domains).await?;

    println!(
        "✅ 已同步 {} 个虚拟机到本地缓存（完整 60 字段）",
        saved_count
    );
    println!("💡 提示: 使用 `atp vdi list-vms` 查看虚拟机列表");

    Ok(())
}

/// 同步所有 VDI 数据到本地缓存
async fn sync_all(config_path: &str) -> Result<()> {
    use atp_storage::{StorageManager, VdiCacheManager};

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║              同步所有 VDI 数据到本地缓存                       ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;

    let client = create_vdi_client(vdi_config).await?;

    // 连接数据库并创建缓存管理器
    let storage_manager = StorageManager::new("~/.config/atp/data.db")
        .await
        .context("无法连接数据库")?;
    let cache = VdiCacheManager::new(storage_manager);

    // 1. 同步主机
    println!("📋 步骤 1/4: 同步主机...");
    let hosts = client.host().list_all().await?;
    let hosts_count = cache.sync_hosts(&hosts).await?;
    println!("   ✅ 同步 {} 个主机\n", hosts_count);

    // 2. 同步虚拟机
    println!("📋 步骤 2/4: 同步虚拟机...");
    let domains = client.domain().list_all().await?;
    let domains_count = cache.sync_domains(&domains).await?;
    println!("   ✅ 同步 {} 个虚拟机\n", domains_count);

    // 3. 同步存储池
    println!("📋 步骤 3/4: 同步存储池...");
    let storage_pools = client.storage().list_all_pools().await?;
    let storage_pools_count = cache.sync_storage_pools(&storage_pools).await?;
    println!("   ✅ 同步 {} 个存储池\n", storage_pools_count);

    // 4. 同步存储卷
    println!("📋 步骤 4/4: 同步存储卷...");
    let storage_volumes = client.storage().list_all_volumes().await?;
    let storage_volumes_count = cache.sync_storage_volumes(&storage_volumes).await?;
    println!("   ✅ 同步 {} 个存储卷\n", storage_volumes_count);

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                      同步完成                                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("📊 同步统计:");
    println!("   主机:     {} 个", hosts_count);
    println!("   虚拟机:   {} 个", domains_count);
    println!("   存储池:   {} 个", storage_pools_count);
    println!("   存储卷:   {} 个", storage_volumes_count);
    println!();
    println!("💡 提示: 数据已缓存到本地，后续查询将使用本地数据");

    Ok(())
}

/// 查询虚拟机磁盘存储位置
///
/// 支持查询本地存储和 Gluster 分布式存储的实际位置。
/// 核心业务逻辑委托给 `StorageOpsService`，CLI 仅负责参数解析和输出格式化。
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
    let client = Arc::new(create_vdi_client(vdi_config).await?);
    println!("   ✅ VDI 登录成功\n");

    // 2. 查找虚拟机并查询磁盘位置
    println!("📋 步骤 2/3: 查找虚拟机 {}...", vm_id_or_name);

    if enable_ssh {
        // 使用 StorageOpsService 查询（支持 Gluster 位置）
        // 创建 SSH 连接管理器（带存储支持，自动从数据库获取密码）
        let storage = match StorageManager::new("~/.config/atp/data.db").await {
            Ok(manager) => Some(Arc::new(Storage::from_manager(&manager))),
            Err(_) => None,
        };

        let mut ssh_manager = SshConnectionManager::new(ssh_user);
        if let Some(s) = storage {
            ssh_manager = ssh_manager.with_storage(s);
        }

        // 创建存储操作服务
        let mut service = StorageOpsService::new(Arc::clone(&client), ssh_manager);

        // 构建 SSH 参数
        let ssh_params = SshParams {
            user: ssh_user.to_string(),
            password: ssh_password.map(|s| s.to_string()),
            key_path: ssh_key.map(PathBuf::from),
        };

        // 执行查询（核心逻辑在 executor 层）
        let result = service
            .query_disk_location(vm_id_or_name, &ssh_params)
            .await?;
        println!(
            "   ✅ 找到虚拟机: {} ({})\n",
            result.domain_name, result.domain_id
        );

        if result.disks.is_empty() {
            println!("⚠️  该虚拟机没有磁盘");
            return Ok(());
        }

        println!("📋 步骤 3/3: 获取磁盘信息...");
        println!("   ✅ 找到 {} 个磁盘\n", result.disks.len());

        // 输出结果（CLI 层职责）
        match format {
            "json" => output_disk_location_result_json(&result)?,
            _ => output_disk_location_result_table(&result)?,
        }
    } else {
        // 不使用 SSH，仅显示基本磁盘信息
        let domains = client.domain().list_all().await?;
        let domain = domains
            .iter()
            .find(|d| {
                d["id"].as_str() == Some(vm_id_or_name) || d["name"].as_str() == Some(vm_id_or_name)
            })
            .context(format!("未找到虚拟机: {}", vm_id_or_name))?;

        let domain_id = domain["id"].as_str().unwrap_or("");
        let domain_name = domain["name"].as_str().unwrap_or("");
        println!("   ✅ 找到虚拟机: {} ({})\n", domain_name, domain_id);

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

        let has_gluster = disks.iter().any(|d| d.is_gluster());
        if has_gluster {
            println!("💡 提示: 使用 --ssh 参数可查询 Gluster 实际 brick 位置\n");
        }

        // 简化输出（无 Gluster 位置）
        let result = DiskLocationResult {
            domain_name: domain_name.to_string(),
            domain_id: domain_id.to_string(),
            disks: disks
                .into_iter()
                .map(|disk| DiskLocationInfo {
                    disk,
                    gluster_location: None,
                    error: None,
                })
                .collect(),
        };

        match format {
            "json" => output_disk_location_result_json(&result)?,
            _ => output_disk_location_result_table(&result)?,
        }
    }

    Ok(())
}

/// 表格格式输出磁盘位置（使用 DiskLocationResult）
fn output_disk_location_result_table(result: &DiskLocationResult) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                      磁盘存储位置详情                          ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("虚拟机: {}\n", result.domain_name);

    for (i, disk_info) in result.disks.iter().enumerate() {
        let disk = &disk_info.disk;
        let boot_label = if disk.is_boot_disk() {
            " [启动盘]"
        } else {
            ""
        };
        println!("📀 磁盘 {} - {}{}\n", i + 1, disk.name, boot_label);

        println!("   文件名:     {}", disk.filename);
        println!("   逻辑路径:   {}", disk.vol_full_path);
        println!("   存储池:     {} ({})", disk.pool_name, disk.pool_type);
        println!("   存储类型:   {}", disk.storage_type_display());
        println!("   大小:       {} GB", disk.size);
        println!("   总线类型:   {}", disk.bus_type);

        // 显示 Gluster 位置信息（如果有）
        if disk.is_gluster() {
            if let Some(ref location) = disk_info.gluster_location {
                println!("\n   🔍 Gluster 实际存储位置:");
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
            } else if let Some(ref err) = disk_info.error {
                println!("\n   ⚠️  查询失败: {}", err);
            } else {
                println!("\n   💡 使用 --ssh 查询 Gluster brick 位置");
            }
        }

        println!();
    }

    Ok(())
}

/// JSON 格式输出磁盘位置（使用 DiskLocationResult）
fn output_disk_location_result_json(result: &DiskLocationResult) -> Result<()> {
    let mut disk_results = Vec::new();

    for disk_info in &result.disks {
        let disk = &disk_info.disk;
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

        // 添加 Gluster 位置信息
        if let Some(ref location) = disk_info.gluster_location {
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

        // 添加错误信息（如果有）
        if let Some(ref err) = disk_info.error {
            disk_json["gluster_location_error"] = json!(err);
        }

        disk_results.push(disk_json);
    }

    let output = json!({
        "domain_name": result.domain_name,
        "domain_id": result.domain_id,
        "disk_count": result.disks.len(),
        "disks": disk_results,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

// ============================================================================
// 批量操作命令实现
// ============================================================================

/// 批量启动虚拟机
///
/// CLI 层仅负责参数解析和输出格式化，核心逻辑委托给 VdiBatchOps
async fn batch_start_vms(
    config_path: &str,
    pattern: &str,
    dry_run: bool,
    verify: bool,
    format: &str,
) -> Result<()> {
    use atp_executor::{BatchOperations, VdiBatchOps, VmInfo};
    use atp_transport::{TransportConfig, TransportManager};
    use std::sync::Arc;

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    批量启动虚拟机                              ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // 加载配置并创建 VDI 客户端
    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;
    let client = Arc::new(create_vdi_client(vdi_config).await?);
    println!("✅ VDI 登录成功\n");

    // 创建核心批量操作器
    let transport_manager = Arc::new(TransportManager::new(TransportConfig::default()));
    let batch_ops = VdiBatchOps::new(transport_manager, client);

    // 使用核心模块获取匹配的虚拟机
    println!("🔍 匹配模式: {}\n", pattern);
    let all_vms = batch_ops.get_matching_vms(pattern).await?;

    // 过滤关机状态的虚拟机 (VDI 平台: status=0 为 Shutoff)
    let vms_to_start: Vec<_> = all_vms
        .iter()
        .filter(|vm| vm.status_code == 0)
        .cloned()
        .collect();

    if vms_to_start.is_empty() {
        println!("⚠️  没有找到需要启动的关机虚拟机");
        return Ok(());
    }

    println!("📋 找到 {} 个关机虚拟机:\n", vms_to_start.len());

    // 输出格式化 (CLI 职责)
    match format {
        "json" => {
            let json_data: Vec<_> = vms_to_start
                .iter()
                .map(|vm| {
                    json!({
                        "id": vm.id,
                        "name": vm.name,
                        "status": vm.status,
                        "host": vm.host_name,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_data)?);
        }
        _ => {
            println!("{:<30} {:<20} {:<15}", "虚拟机名称", "主机", "状态");
            println!("{}", "-".repeat(70));
            for vm in &vms_to_start {
                println!("{:<30} {:<20} {:<15}", vm.name, vm.host_name, vm.status);
            }
        }
    }

    if dry_run {
        println!("\n📝 预览模式 - 不执行实际操作");
        return Ok(());
    }

    // 使用核心模块执行批量启动
    println!("\n🚀 正在启动虚拟机...");
    let result = batch_ops.batch_start(&vms_to_start, verify).await?;

    println!("\n✅ 批量启动命令已发送");
    println!("   成功: {}", result.success_count);

    if !result.failed_vms.is_empty() {
        println!("⚠️  部分虚拟机启动失败:");
        for err in &result.failed_vms {
            println!("   - {}: {}", err.vm_id, err.error);
        }
    }

    // QGA 验证结果输出 (CLI 职责)
    if let Some(ref verify_results) = result.verification_results {
        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║                    QGA 验证结果                                ║");
        println!("╚════════════════════════════════════════════════════════════════╝\n");

        let success_count = verify_results.iter().filter(|r| r.success).count();
        let failed_results: Vec<_> = verify_results.iter().filter(|r| !r.success).collect();

        println!("📊 验证统计:");
        println!("   总数: {}", verify_results.len());
        println!("   成功: {} ✅", success_count);
        println!("   失败: {} ❌", failed_results.len());

        if !failed_results.is_empty() {
            println!("\n❌ 未成功启动的虚拟机列表:");
            println!("{:<30} {:<20} {:<30}", "虚拟机名称", "主机", "错误原因");
            println!("{}", "-".repeat(80));
            for r in &failed_results {
                let error = r.error_msg.as_deref().unwrap_or("未知错误");
                println!("{:<30} {:<20} {:<30}", r.vm_name, r.host_name, error);
            }

            // 如果有失败的虚拟机，以非零状态退出
            std::process::exit(1);
        }
    }

    Ok(())
}

/// 批量分配虚拟机给用户
async fn batch_assign_vms(
    config_path: &str,
    pattern: &str,
    users_str: Option<&str>,
    group_name: Option<&str>,
    force: bool,
    dry_run: bool,
    format: &str,
) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    批量分配虚拟机                              ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // 加载配置
    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;

    // 登录 VDI
    let client = Arc::new(create_vdi_client(vdi_config).await?);
    println!("✅ VDI 登录成功\n");

    use atp_executor::{BatchOperations, VdiBatchOps};
    let transport_manager = Arc::new(TransportManager::default());
    let batch_ops = VdiBatchOps::new(transport_manager, client.clone());

    // 获取匹配的虚拟机
    println!("🔍 匹配模式: {}\n", pattern);
    let all_vms = batch_ops.get_matching_vms(pattern).await?;

    // 分离已分配和未分配的虚拟机
    let (assigned_vms, unassigned_vms): (Vec<_>, Vec<_>) =
        all_vms.iter().partition(|vm| vm.bound_user.is_some());

    // 收集已有虚拟机的用户 ID
    let users_with_vms: std::collections::HashSet<_> = assigned_vms
        .iter()
        .filter_map(|vm| vm.bound_user.as_ref())
        .collect();

    // 确定要处理的虚拟机列表和是否跳过已有虚拟机的用户
    let (vms_to_assign, skip_users_with_vms): (Vec<_>, bool) = if assigned_vms.is_empty() {
        // 没有已分配的虚拟机，直接使用未分配的
        (unassigned_vms.iter().cloned().collect(), false)
    } else if force {
        // 强制模式：使用所有匹配的虚拟机，不跳过用户
        println!(
            "⚠️  强制模式: 将覆盖 {} 个已绑定虚拟机的用户\n",
            assigned_vms.len()
        );
        (all_vms.iter().collect(), false)
    } else if dry_run {
        // 预览模式且有已分配虚拟机：显示全部信息但只处理未分配的
        println!(
            "⚠️  发现 {} 个虚拟机已有绑定用户 (预览模式下跳过):\n",
            assigned_vms.len()
        );
        for vm in &assigned_vms {
            println!("  - {} -> {}", vm.name, vm.bound_user.as_ref().unwrap());
        }
        println!();
        (unassigned_vms.iter().cloned().collect(), true)
    } else {
        // 交互模式：提示用户选择
        println!("\n⚠️  发现 {} 个虚拟机已有绑定用户:", assigned_vms.len());
        for vm in &assigned_vms {
            println!("  - {} -> {}", vm.name, vm.bound_user.as_ref().unwrap());
        }
        println!("\n选择操作:");
        println!("  [S] 跳过已绑定虚拟机，仅分配未绑定的");
        println!("  [R] 重新分配所有虚拟机（覆盖已绑定用户）");
        println!("  [C] 取消操作");
        print!("\n请选择 (S/R/C): ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_uppercase();

        match choice.as_str() {
            "R" => {
                println!("\n📌 将重新分配所有虚拟机\n");
                (all_vms.iter().collect(), false)
            }
            "S" => {
                println!("\n📌 将跳过已绑定虚拟机\n");
                (unassigned_vms.iter().cloned().collect(), true)
            }
            _ => {
                println!("\n❌ 已取消操作");
                return Ok(());
            }
        }
    };

    if vms_to_assign.is_empty() {
        println!("⚠️  没有找到需要分配的虚拟机");
        return Ok(());
    }

    // 获取目标用户
    let all_target_users: Vec<atp_vdiplatform::User> = if let Some(users) = users_str {
        // 从用户名列表获取
        let usernames: Vec<String> = users.split(',').map(|s| s.trim().to_string()).collect();
        println!("📋 指定用户: {:?}\n", usernames);
        client.user().find_by_usernames(&usernames).await?
    } else if let Some(group) = group_name {
        // 从组织单位获取
        println!("📋 组织单位: {}\n", group);
        let group_info = client
            .group()
            .find_by_name(group)
            .await?
            .context(format!("未找到组织单位: {}", group))?;
        client
            .user()
            .list_by_group(&group_info.distinguished_name)
            .await?
    } else {
        bail!("必须指定 --users 或 --group 参数");
    };

    if all_target_users.is_empty() {
        println!("⚠️  没有找到目标用户");
        return Ok(());
    }

    // 如果跳过模式，过滤掉已有虚拟机的用户
    let target_users: Vec<_> = if skip_users_with_vms && !users_with_vms.is_empty() {
        let filtered: Vec<_> = all_target_users
            .into_iter()
            .filter(|u| !users_with_vms.contains(&u.username))
            .collect();
        let skipped_count = users_with_vms.len();
        if skipped_count > 0 {
            println!("📌 跳过 {} 个已有虚拟机的用户\n", skipped_count);
        }
        filtered
    } else {
        all_target_users
    };

    if target_users.is_empty() {
        println!("⚠️  没有需要分配的用户（所有用户都已有虚拟机）");
        return Ok(());
    }

    // 统计
    let reassign_count = vms_to_assign
        .iter()
        .filter(|vm| vm.bound_user.is_some())
        .count();
    let new_assign_count = vms_to_assign.len() - reassign_count;

    println!("👥 找到 {} 个目标用户", target_users.len());
    if reassign_count > 0 {
        println!(
            "💻 找到 {} 个虚拟机 ({} 新分配, {} 重新分配)\n",
            vms_to_assign.len(),
            new_assign_count,
            reassign_count
        );
    } else {
        println!("💻 找到 {} 个未分配虚拟机\n", vms_to_assign.len());
    }

    // 生成分配计划（1:1 对应）
    let plan_count = std::cmp::min(vms_to_assign.len(), target_users.len());
    let mut assignment_plans: Vec<AssignmentPlan> = Vec::new();

    for i in 0..plan_count {
        assignment_plans.push(AssignmentPlan {
            vm_id: vms_to_assign[i].id.clone(),
            vm_name: vms_to_assign[i].name.clone(),
            user_id: target_users[i].id.clone(),
            username: target_users[i].username.clone(),
            is_reassignment: vms_to_assign[i].bound_user.is_some(),
        });
    }

    // 显示分配计划
    match format {
        "json" => {
            let json_data: Vec<_> = assignment_plans
                .iter()
                .map(|p| {
                    json!({
                        "vm_id": p.vm_id,
                        "vm_name": p.vm_name,
                        "user_id": p.user_id,
                        "user_name": p.username,
                        "is_reassignment": p.is_reassignment,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_data)?);
        }
        _ => {
            println!("{:<30} {:<20} {:<12}", "虚拟机", "分配给用户", "状态");
            println!("{}", "-".repeat(65));
            for plan in &assignment_plans {
                let status = if plan.is_reassignment {
                    "重新分配"
                } else {
                    "新分配"
                };
                println!("{:<30} {:<20} {:<12}", plan.vm_name, plan.username, status);
            }
        }
    }

    if vms_to_assign.len() > target_users.len() {
        println!(
            "\n⚠️  有 {} 个虚拟机没有匹配的用户",
            vms_to_assign.len() - target_users.len()
        );
    } else if target_users.len() > vms_to_assign.len() {
        println!(
            "\n⚠️  有 {} 个用户没有匹配的虚拟机",
            target_users.len() - vms_to_assign.len()
        );
    }

    if dry_run {
        println!("\n📝 预览模式 - 不执行实际操作");
        return Ok(());
    }

    // 执行分配 - 使用核心模块
    println!("\n🔗 正在分配虚拟机...");
    let result = batch_ops.batch_assign(&assignment_plans).await?;

    println!(
        "\n📊 分配结果: 成功 {}, 失败 {}",
        result.success_count, result.error_count
    );

    if !result.errors.is_empty() {
        for err in &result.errors {
            error!("❌ {} -> {}", err.vm_name, err.error);
        }
    }

    Ok(())
}

/// 批量重命名虚拟机为绑定用户名
async fn batch_rename_vms(
    config_path: &str,
    pattern: &str,
    dry_run: bool,
    format: &str,
) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    批量重命名虚拟机                            ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // 加载配置
    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;

    // 登录 VDI
    let client = Arc::new(create_vdi_client(vdi_config).await?);
    println!("✅ VDI 登录成功\n");

    use atp_executor::{BatchOperations, VdiBatchOps};
    let transport_manager = Arc::new(TransportManager::default());
    let batch_ops = VdiBatchOps::new(transport_manager, client.clone());

    // 获取匹配的虚拟机
    println!("🔍 匹配模式: {}\n", pattern);
    let all_vms = batch_ops.get_matching_vms(pattern).await?;

    // 过滤：已绑定用户且名称不同
    let rename_plans: Vec<RenamePlan> = all_vms
        .iter()
        .filter_map(|vm| {
            if let (Some(ref bound_user), Some(ref bound_user_id)) =
                (&vm.bound_user, &vm.bound_user_id)
            {
                if vm.name != *bound_user {
                    return Some(RenamePlan {
                        vm_id: vm.id.clone(),
                        old_name: vm.name.clone(),
                        new_name: bound_user.clone(),
                        user_id: bound_user_id.clone(),
                    });
                }
            }
            None
        })
        .collect();

    if rename_plans.is_empty() {
        println!("⚠️  没有找到需要重命名的虚拟机");
        return Ok(());
    }

    println!("📋 找到 {} 个需要重命名的虚拟机:\n", rename_plans.len());

    match format {
        "json" => {
            let json_data: Vec<_> = rename_plans
                .iter()
                .map(|p| {
                    json!({
                        "vm_id": p.vm_id,
                        "old_name": p.old_name,
                        "new_name": p.new_name,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_data)?);
        }
        _ => {
            println!("{:<30} {:<30}", "当前名称", "新名称");
            println!("{}", "-".repeat(65));
            for plan in &rename_plans {
                println!("{:<30} {:<30}", plan.old_name, plan.new_name);
            }
        }
    }

    if dry_run {
        println!("\n📝 预览模式 - 不执行实际操作");
        return Ok(());
    }

    // 执行重命名 - 使用核心模块
    println!("\n📝 正在重命名虚拟机...");
    let result = batch_ops.batch_rename(&rename_plans).await?;

    println!(
        "\n📊 重命名结果: 成功 {}, 失败 {}",
        result.success_count, result.error_count
    );

    if !result.errors.is_empty() {
        for err in &result.errors {
            error!("❌ {} -> {}", err.vm_name, err.error);
        }
    }

    Ok(())
}

/// 批量设置自动加域 (autoJoinDomain)
async fn batch_set_auto_ad(
    config_path: &str,
    pattern: &str,
    enable: bool,
    disable: bool,
    dry_run: bool,
    format: &str,
) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                 批量设置自动加域 (autoJoinDomain)              ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    if !enable && !disable {
        bail!("必须指定 --enable 或 --disable 参数");
    }

    let action_name = if enable { "启用" } else { "禁用" };

    // 加载配置
    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;

    // 登录 VDI
    let client = Arc::new(create_vdi_client(vdi_config).await?);
    println!("✅ VDI 登录成功\n");

    let transport_manager = Arc::new(TransportManager::default());
    let batch_ops = VdiBatchOps::new(transport_manager, client.clone());

    // 获取匹配的虚拟机
    println!("🔍 匹配模式: {}", pattern);
    println!("🎯 操作: {} 自动加域\n", action_name);

    let all_vms = batch_ops.get_matching_vms(pattern).await?;

    if all_vms.is_empty() {
        println!("⚠️  没有找到匹配的虚拟机");
        return Ok(());
    }

    println!("📋 找到 {} 个匹配的虚拟机:\n", all_vms.len());

    match format {
        "json" => {
            let json_data: Vec<_> = all_vms
                .iter()
                .map(|vm| {
                    json!({
                        "id": vm.id,
                        "name": vm.name,
                        "status": vm.status,
                        "host": vm.host_name,
                        "action": action_name,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_data)?);
        }
        _ => {
            println!("{:<30} {:<20} {:<15}", "虚拟机名称", "主机", "操作");
            println!("{}", "-".repeat(70));
            for vm in &all_vms {
                println!("{:<30} {:<20} {}", vm.name, vm.host_name, action_name);
            }
        }
    }

    if dry_run {
        println!("\n📝 预览模式 - 不执行实际操作");
        return Ok(());
    }

    // 执行设置 - 使用核心模块
    println!("\n⚙️  正在设置 autoJoinDomain...");
    let result = batch_ops.batch_set_auto_ad(&all_vms, enable).await?;

    println!(
        "\n📊 设置结果: 成功 {}, 失败 {}",
        result.success_count, result.error_count
    );

    if !result.errors.is_empty() {
        for err in &result.errors {
            error!("❌ {} - {} 失败: {}", err.vm_name, action_name, err.error);
        }
    }

    Ok(())
}

// ============================================
// Gluster 脑裂修复
// ============================================

/// Gluster 存储脑裂修复
///
/// 核心业务逻辑委托给 `StorageOpsService`，CLI 仅负责参数解析、存储池选择和输出格式化。
#[allow(clippy::too_many_arguments)]
async fn heal_splitbrain(
    config_path: &str,
    pool_id: Option<&str>,
    enable_ssh: bool,
    ssh_user: &str,
    ssh_password: Option<&str>,
    ssh_key: Option<&str>,
    dry_run: bool,
    auto_mode: bool,
    format: &str,
) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║              Gluster 存储脑裂修复                               ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    if !enable_ssh {
        bail!("脑裂修复需要 SSH 连接，请使用 --ssh 参数");
    }

    // 加载配置
    let config = TestConfig::load_from_path(config_path)
        .context(format!("无法加载配置文件: {}", config_path))?;
    let vdi_config = config
        .vdi
        .as_ref()
        .context("配置文件中未找到 VDI 平台配置")?;

    // 1. 登录 VDI 平台
    println!("📋 步骤 1/2: 连接 VDI 平台和存储池...");
    let client = Arc::new(create_vdi_client(vdi_config).await?);
    println!("   ✅ VDI 登录成功");

    // 确定存储池 ID（交互式选择或使用指定的）
    let selected_pool_id: String = match pool_id {
        Some(id) => id.to_string(),
        None => {
            // 获取所有存储池并筛选 Gluster 类型
            println!("\n📋 获取存储池列表...");
            let all_pools = client.storage().list_all_pools().await?;

            // 筛选 Gluster 类型的存储池
            let gluster_pools: Vec<_> = all_pools
                .iter()
                .filter(|p| {
                    let pool_type = p["type"]
                        .as_str()
                        .or_else(|| p["poolType"].as_str())
                        .unwrap_or("");
                    pool_type == "gluster"
                })
                .collect();

            if gluster_pools.is_empty() {
                println!("\n   ⚠️  未找到 Gluster 存储池，当前所有存储池：");
                for pool in &all_pools {
                    let name = pool["name"].as_str().unwrap_or("未知");
                    let t = pool["type"]
                        .as_str()
                        .or_else(|| pool["poolType"].as_str())
                        .unwrap_or("未知");
                    println!("      - {} (类型: {})", name, t);
                }
                bail!("未找到 Gluster 类型的存储池");
            }

            println!("\n   发现 {} 个 Gluster 存储池：\n", gluster_pools.len());
            println!("   {:<4} {:<40} {:<30}", "序号", "存储池 ID", "名称");
            println!("   {}", "-".repeat(75));

            for (i, pool) in gluster_pools.iter().enumerate() {
                let pool_name = pool["name"].as_str().unwrap_or("未知");
                let id = pool["id"].as_str().unwrap_or("未知");
                println!("   {:<4} {:<40} {:<30}", i + 1, id, pool_name);
            }

            println!();
            print!(
                "   请选择要修复的存储池 (输入序号 1-{}): ",
                gluster_pools.len()
            );
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let choice: usize = input.trim().parse().context("请输入有效的数字")?;

            if choice == 0 || choice > gluster_pools.len() {
                bail!("无效的选择: {}", choice);
            }

            let selected = &gluster_pools[choice - 1];
            selected["id"]
                .as_str()
                .context("无法获取存储池 ID")?
                .to_string()
        }
    };

    // 验证存储池类型
    let pool_detail = client.storage().get_pool(&selected_pool_id).await?;
    let data = &pool_detail["data"];
    let pool_type = data["type"]
        .as_str()
        .or_else(|| data["poolType"].as_str())
        .unwrap_or("");
    if pool_type != "gluster" {
        bail!(
            "存储池 {} 不是 Gluster 类型 (类型: {})",
            selected_pool_id,
            pool_type
        );
    }

    let volume_name = data["sourceName"]
        .as_str()
        .or_else(|| data["volumeName"].as_str())
        .or_else(|| data["volName"].as_str())
        .context("无法获取 Gluster 卷名")?;

    println!(
        "   ✅ 存储池: {} (Gluster 卷: {})\n",
        selected_pool_id, volume_name
    );

    // 2. 创建存储操作服务
    let storage = match StorageManager::new("~/.config/atp/data.db").await {
        Ok(manager) => Some(Arc::new(Storage::from_manager(&manager))),
        Err(_) => None,
    };

    let mut ssh_manager = SshConnectionManager::new(ssh_user);
    if let Some(s) = storage {
        ssh_manager = ssh_manager.with_storage(s);
    }

    let mut service = StorageOpsService::new(Arc::clone(&client), ssh_manager);

    // 构建 SSH 参数
    let ssh_params = SshParams {
        user: ssh_user.to_string(),
        password: ssh_password.map(|s| s.to_string()),
        key_path: ssh_key.map(PathBuf::from),
    };

    // 3. 确定修复策略
    let strategy = if dry_run {
        HealStrategy::DryRun
    } else if auto_mode {
        HealStrategy::Auto
    } else {
        HealStrategy::Interactive
    };

    // 4. 执行修复
    println!("📋 步骤 2/2: 检测并修复脑裂文件...\n");

    let report = if auto_mode || dry_run {
        // 自动模式或预览模式
        service
            .heal_splitbrain(
                &selected_pool_id,
                &ssh_params,
                strategy,
                &AutoReplicaSelector,
            )
            .await?
    } else {
        // 交互模式：使用回调函数询问用户
        let selector = InteractiveReplicaSelector::new(
            |entry: &SplitBrainEntry,
             stats: &[ReplicaStat],
             affected_vm: Option<&AffectedVm>|
             -> Option<usize> {
                // 输出文件信息
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("📄 文件: {} ({})", entry.path, entry.entry_type);
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

                // 显示 VM 信息
                if let Some(vm) = affected_vm {
                    println!("📋 受影响的虚拟机:");
                    println!("   ├── 名称: {} (ID: {})", vm.name, vm.id);
                    println!("   ├── 磁盘: {}", vm.disk_name);
                    println!("   ├── 主机: {}", vm.host_name);
                    println!("   └── 状态: {}", vm.status);
                } else {
                    println!("   ⚠️  未找到对应的虚拟机，可能是孤立磁盘");
                }

                // 显示副本信息
                println!("\n📋 副本信息:");
                for stat in stats {
                    println!("   副本 {}: {}:{}", stat.index, stat.host, stat.full_path);
                    if let (Some(size), Some(mtime)) = (stat.size, &stat.mtime) {
                        let size_human = format_size(size);
                        println!("      大小: {}, 修改时间: {}", size_human, mtime);
                    }
                }

                // 询问用户选择
                print!("\n   请选择要舍弃的副本 [1/2] (0 跳过): ");
                if io::stdout().flush().is_err() {
                    return None;
                }

                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() {
                    return None;
                }

                match input.trim().parse::<usize>() {
                    Ok(0) => None,
                    Ok(choice) if choice >= 1 && choice <= stats.len() => Some(choice),
                    _ => {
                        println!("   ⚠️  无效选择，跳过此文件");
                        None
                    }
                }
            },
        );

        service
            .heal_splitbrain(&selected_pool_id, &ssh_params, strategy, &selector)
            .await?
    };

    // 5. 输出报告
    output_heal_report(&report, format, dry_run)?;

    Ok(())
}

/// 格式化文件大小
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// 输出脑裂修复报告
fn output_heal_report(report: &HealReport, format: &str, dry_run: bool) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        _ => {
            println!("\n╔════════════════════════════════════════════════════════════════╗");
            println!("║                        修复完成                                 ║");
            println!(
                "║  成功: {} 个文件   跳过: {} 个   失败: {} 个                   ║",
                report.success_count, report.skip_count, report.fail_count
            );
            if !dry_run && report.success_count > 0 {
                println!("║  ⚠️  受影响的 VM 保持关机状态，请手动启动                       ║");
            }
            println!("╚════════════════════════════════════════════════════════════════╝\n");

            // 输出详细结果
            if !report.results.is_empty() {
                println!("详细结果:");
                for result in &report.results {
                    match result {
                        atp_executor::HealEntryResult::Success {
                            path,
                            discarded_replica,
                        } => {
                            println!("   ✅ {} - 舍弃副本: {}", path, discarded_replica);
                        }
                        atp_executor::HealEntryResult::Skipped { path, reason } => {
                            println!("   ⏭️  {} - {}", path, reason);
                        }
                        atp_executor::HealEntryResult::Failed { path, error } => {
                            println!("   ❌ {} - {}", path, error);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

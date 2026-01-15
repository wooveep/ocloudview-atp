//! PowerShell 远程执行命令
//!
//! 通过 QGA (QEMU Guest Agent) 协议向 Windows 虚拟机发送 Base64 编码的 PowerShell 命令
//! WebSocket 验证是可选的，用于确认虚拟机是否收到并执行了命令

use crate::commands::common::create_vdi_client;
use crate::PowerShellAction;
use anyhow::{Context, Result};
use atp_executor::{TestConfig, VdiBatchOps};
use atp_protocol::qga::{GuestExecCommand, GuestExecStatus, QgaProtocol};
use atp_protocol::Protocol; // 需要导入 trait 来使用 connect/disconnect 方法
use atp_transport::{HostConnection, HostInfo, TransportManager};
use atp_vdiplatform::DomainStatus;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use colored::Colorize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

/// VM 目标信息
#[derive(Debug, Clone)]
struct VmTarget {
    name: String,
    ip: Option<String>,
    host_name: String,
    host_ip: Option<String>,
    status: i64,
}

/// PowerShell 执行结果
#[derive(Debug, Clone)]
struct PsExecResult {
    vm_name: String,
    success: bool,
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
    error: Option<String>,
}

pub async fn handle(action: PowerShellAction) -> Result<()> {
    match action {
        PowerShellAction::Exec {
            config,
            vm,
            vms,
            all,
            host,
            command,
            script_file,
            timeout: timeout_secs,
            json_output,
        } => {
            exec_powershell(
                &config,
                vm,
                vms,
                all,
                host,
                command,
                script_file,
                timeout_secs,
                json_output,
            )
            .await
        }
        PowerShellAction::ListVms { config, host } => list_vms(&config, host).await,
    }
}

/// 执行 PowerShell 命令
#[allow(clippy::too_many_arguments)]
async fn exec_powershell(
    config_path: &str,
    vm: Option<String>,
    vms: Option<String>,
    all: bool,
    host_filter: Option<String>,
    command: Option<String>,
    script_file: Option<String>,
    timeout_secs: u64,
    json_output: bool,
) -> Result<()> {
    // 获取要执行的 PowerShell 命令
    let ps_command = if let Some(cmd) = command {
        cmd
    } else if let Some(file) = script_file {
        tokio::fs::read_to_string(&file)
            .await
            .context(format!("无法读取脚本文件: {}", file))?
    } else {
        anyhow::bail!("必须指定 --command 或 --script-file");
    };

    if !json_output {
        println!(
            "{}",
            "╔════════════════════════════════════════════════════════════════╗".cyan()
        );
        println!(
            "{}",
            "║        PowerShell 远程命令执行 (via QGA)                       ║".cyan()
        );
        println!(
            "{}",
            "╚════════════════════════════════════════════════════════════════╝".cyan()
        );
        println!();
    }

    // 确定目标虚拟机
    let targets = resolve_targets(config_path, vm, vms, all, host_filter).await?;

    if targets.is_empty() {
        if json_output {
            println!("{}", json!({"error": "没有找到目标虚拟机"}));
        } else {
            println!("{} 没有找到目标虚拟机", "❌".red());
        }
        return Ok(());
    }

    if !json_output {
        println!(
            "{} 找到 {} 个目标虚拟机",
            "📋".cyan(),
            targets.len().to_string().yellow()
        );
        for target in &targets {
            let status_icon = if target.status == 1 { "✅" } else { "⚪" };
            println!(
                "   {} {} (主机: {}, IP: {})",
                status_icon,
                target.name.green(),
                target.host_name,
                target.ip.as_deref().unwrap_or("N/A")
            );
        }
        println!();
    }

    // 将 PowerShell 命令转为 UTF-16LE Base64 编码（Windows PowerShell -EncodedCommand 要求）
    let utf16_bytes: Vec<u8> = ps_command
        .encode_utf16()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let encoded_command = BASE64.encode(&utf16_bytes);

    if !json_output {
        println!("{} PowerShell 命令 (原始):", "📝".cyan());
        // 截断显示
        let display_cmd = if ps_command.len() > 200 {
            format!("{}...", &ps_command[..200])
        } else {
            ps_command.clone()
        };
        println!("   {}", display_cmd.bright_black());
        println!();
        println!(
            "{} 命令长度: {} 字节, UTF-16LE Base64: {} 字节",
            "📊".cyan(),
            ps_command.len(),
            encoded_command.len()
        );
        println!();
    }

    // 按主机分组执行
    let mut host_groups: HashMap<String, Vec<&VmTarget>> = HashMap::new();
    for target in &targets {
        if let Some(host_ip) = &target.host_ip {
            host_groups.entry(host_ip.clone()).or_default().push(target);
        }
    }

    let mut results: Vec<PsExecResult> = Vec::new();
    let mut success_count = 0;
    let mut fail_count = 0;

    // 遍历每个主机
    for (host_ip, host_vms) in &host_groups {
        if !json_output {
            println!(
                "{} 连接主机: {} ({} 个虚拟机)",
                "🔗".cyan(),
                host_ip.yellow(),
                host_vms.len()
            );
        }

        // 连接到 libvirt
        let uri = format!("qemu+tcp://{}/system", host_ip);
        let host_info = HostInfo {
            id: host_ip.clone(),
            host: host_ip.clone(),
            uri: uri.clone(),
            tags: vec![],
            metadata: HashMap::new(),
        };

        let conn = HostConnection::new(host_info);
        if let Err(e) = conn.connect().await {
            if !json_output {
                println!("   {} 连接主机失败: {}", "❌".red(), e);
            }
            // 标记该主机上所有 VM 为失败
            for vm_target in host_vms {
                results.push(PsExecResult {
                    vm_name: vm_target.name.clone(),
                    success: false,
                    exit_code: None,
                    stdout: None,
                    stderr: None,
                    error: Some(format!("连接主机失败: {}", e)),
                });
                fail_count += 1;
            }
            continue;
        }

        if !json_output {
            println!("   {} 主机连接成功", "✅".green());
        }

        // 对该主机上的每个 VM 执行命令
        for vm_target in host_vms {
            if !json_output {
                println!(
                    "\n   {} 执行命令: {} ...",
                    "🚀".cyan(),
                    vm_target.name.green()
                );
            }

            // 检查 VM 状态
            if vm_target.status != 1 {
                if !json_output {
                    println!("      {} 虚拟机未运行，跳过", "⚠️".yellow());
                }
                results.push(PsExecResult {
                    vm_name: vm_target.name.clone(),
                    success: false,
                    exit_code: None,
                    stdout: None,
                    stderr: None,
                    error: Some("虚拟机未运行".to_string()),
                });
                fail_count += 1;
                continue;
            }

            // 通过 QGA 执行 PowerShell 命令
            match execute_ps_via_qga(&conn, &vm_target.name, &encoded_command, timeout_secs).await {
                Ok(result) => {
                    let exit_code = result.exit_code.unwrap_or(-1);
                    let is_success = exit_code == 0;

                    if !json_output {
                        if is_success {
                            println!("      {} 执行成功 (退出码: 0)", "✅".green());
                        } else {
                            println!("      {} 执行完成 (退出码: {})", "⚠️".yellow(), exit_code);
                        }

                        // 显示 stdout
                        if let Some(stdout) = result.decode_stdout() {
                            if !stdout.trim().is_empty() {
                                println!("      {}", "输出:".bright_black());
                                for line in stdout.lines().take(20) {
                                    println!("        {}", line);
                                }
                                if stdout.lines().count() > 20 {
                                    println!("        ... (截断)");
                                }
                            }
                        }

                        // 显示 stderr
                        if let Some(stderr) = result.decode_stderr() {
                            if !stderr.trim().is_empty() {
                                println!("      {}", "错误:".red());
                                for line in stderr.lines().take(10) {
                                    println!("        {}", line.red());
                                }
                            }
                        }
                    }

                    results.push(PsExecResult {
                        vm_name: vm_target.name.clone(),
                        success: is_success,
                        exit_code: result.exit_code,
                        stdout: result.decode_stdout(),
                        stderr: result.decode_stderr(),
                        error: None,
                    });

                    if is_success {
                        success_count += 1;
                    } else {
                        fail_count += 1;
                    }
                }
                Err(e) => {
                    if !json_output {
                        println!("      {} 执行失败: {}", "❌".red(), e);
                    }
                    results.push(PsExecResult {
                        vm_name: vm_target.name.clone(),
                        success: false,
                        exit_code: None,
                        stdout: None,
                        stderr: None,
                        error: Some(e.to_string()),
                    });
                    fail_count += 1;
                }
            }
        }

        // 断开主机连接
        let _ = conn.disconnect().await;
    }

    // 输出汇总
    if json_output {
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                json!({
                    "vm": r.vm_name,
                    "success": r.success,
                    "exit_code": r.exit_code,
                    "stdout": r.stdout,
                    "stderr": r.stderr,
                    "error": r.error
                })
            })
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "total": targets.len(),
                "success": success_count,
                "failed": fail_count,
                "results": json_results
            }))?
        );
    } else {
        println!();
        println!(
            "{}",
            "═══════════════════════════════════════════════════════════════".cyan()
        );
        println!(
            "{} 执行完成: {} 成功, {} 失败, 共 {} 个目标",
            "📊".cyan(),
            success_count.to_string().green(),
            fail_count.to_string().red(),
            targets.len()
        );
    }

    Ok(())
}

/// 通过 QGA 执行 PowerShell 命令
async fn execute_ps_via_qga(
    conn: &HostConnection,
    vm_name: &str,
    encoded_command: &str,
    timeout_secs: u64,
) -> Result<GuestExecStatus> {
    info!("通过 QGA 执行 PowerShell 命令: vm={}", vm_name);

    // 获取 Domain
    let domain = conn
        .get_domain(vm_name)
        .await
        .context(format!("查找虚拟机失败: {}", vm_name))?;

    // 创建 QGA 协议实例
    let mut qga = QgaProtocol::new().with_timeout(timeout_secs as i32);

    // 连接 QGA
    qga.connect(&domain)
        .await
        .context("连接 QGA 失败，请确保 QEMU Guest Agent 已在虚拟机内安装并运行")?;

    debug!("QGA 连接成功");

    // 构建 PowerShell 命令
    // 使用 -EncodedCommand 参数，命令已经是 UTF-16LE Base64 编码
    let cmd = GuestExecCommand {
        path: "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe".to_string(),
        arg: Some(vec![
            "-NoProfile".to_string(),
            "-NonInteractive".to_string(),
            "-ExecutionPolicy".to_string(),
            "Bypass".to_string(),
            "-EncodedCommand".to_string(),
            encoded_command.to_string(),
        ]),
        env: None,
        input_data: None,
        capture_output: Some(true),
    };

    // 执行命令并等待完成
    let result = qga
        .exec_and_wait(cmd)
        .await
        .context("执行 PowerShell 命令失败")?;

    // 断开 QGA
    let _ = qga.disconnect().await;

    Ok(result)
}

/// 解析目标虚拟机列表
async fn resolve_targets(
    config_path: &str,
    vm: Option<String>,
    vms: Option<String>,
    all: bool,
    host_filter: Option<String>,
) -> Result<Vec<VmTarget>> {
    let config = TestConfig::load_from_path(config_path)
        .context(format!("无法加载配置文件: {}", config_path))?;

    let vdi_config = config
        .vdi
        .as_ref()
        .context("配置文件中未找到 VDI 平台配置")?;

    let client = create_vdi_client(vdi_config).await?;

    // 获取所有主机，建立 ID -> 信息映射
    let hosts = client.host().list_all().await?;
    let mut host_id_to_info: HashMap<String, (String, String)> = HashMap::new(); // id -> (name, ip)
    for host in &hosts {
        let host_id = host["id"].as_str().unwrap_or("").to_string();
        let host_name = host["name"].as_str().unwrap_or("").to_string();
        let host_ip = host["ip"].as_str().unwrap_or("").to_string();
        if !host_id.is_empty() && !host_name.is_empty() {
            host_id_to_info.insert(host_id, (host_name, host_ip));
        }
    }

    // 获取所有虚拟机
    let domains = client.domain().list_all().await?;

    let mut targets: Vec<VmTarget> = Vec::new();

    for domain in &domains {
        let name = domain["name"].as_str().unwrap_or("").to_string();
        let status = domain["status"].as_i64().unwrap_or(-1);
        let host_id = domain["hostId"].as_str().unwrap_or("");
        let (host_name, host_ip) = host_id_to_info.get(host_id).cloned().unwrap_or_default();
        let ip = domain["ip"].as_str().map(|s| s.to_string());

        // 根据参数过滤
        let should_include = if let Some(ref target_vm) = vm {
            // 单个 VM 匹配
            name == *target_vm
        } else if let Some(ref vm_list) = vms {
            // VM 列表匹配
            let vm_names: Vec<&str> = vm_list.split(',').map(|s| s.trim()).collect();
            vm_names.contains(&name.as_str())
        } else if all {
            // 所有 VM（可选主机过滤）
            if let Some(ref filter) = host_filter {
                host_name == *filter
            } else {
                true
            }
        } else {
            false
        };

        if should_include && !name.is_empty() {
            targets.push(VmTarget {
                name,
                ip,
                host_name,
                host_ip: if host_ip.is_empty() {
                    None
                } else {
                    Some(host_ip)
                },
                status,
            });
        }
    }

    Ok(targets)
}

/// 列出可用的虚拟机
async fn list_vms(config_path: &str, host_filter: Option<String>) -> Result<()> {
    println!(
        "{}",
        "╔════════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║              可用的 Windows 虚拟机列表                         ║".cyan()
    );
    println!(
        "{}",
        "╚════════════════════════════════════════════════════════════════╝".cyan()
    );
    println!();

    let config = TestConfig::load_from_path(config_path)?;
    let vdi_config = config.vdi.as_ref().context("未配置 VDI 平台")?;

    let client = Arc::new(create_vdi_client(vdi_config).await?);

    let domains = client.domain().list_all().await?;

    // 建立主机ID到名称的映射
    let transport_manager = Arc::new(TransportManager::default());
    let batch_ops = VdiBatchOps::new(Arc::clone(&transport_manager), Arc::clone(&client));
    let host_id_to_name = batch_ops.build_host_id_to_name_map().await?;
    let host_id_to_ip = batch_ops.build_host_id_to_ip_map().await?;

    println!(
        "{:<30} {:<20} {:<15} {:<15}",
        "虚拟机名称", "主机", "状态", "IP"
    );
    println!("{}", "-".repeat(80));

    let mut count = 0;
    for domain in &domains {
        let name = domain["name"].as_str().unwrap_or("");
        let host_id = domain["hostId"].as_str().unwrap_or("");
        let host_name = host_id_to_name.get(host_id).cloned().unwrap_or_default();

        // 主机过滤
        if let Some(ref filter) = host_filter {
            if host_name != *filter {
                continue;
            }
        }

        let status =
            DomainStatus::from_code(domain["status"].as_i64().unwrap_or(-1)).display_with_emoji();

        let ip = domain["ip"].as_str().unwrap_or("N/A");

        println!("{:<30} {:<20} {:<15} {:<15}", name, host_name, status, ip);
        count += 1;
    }

    println!("\n总计: {} 个虚拟机", count);
    println!();
    println!(
        "{} 提示: 只有状态为 '运行中' 且安装了 QEMU Guest Agent 的虚拟机才能执行 PowerShell 命令",
        "ℹ️".cyan()
    );

    Ok(())
}

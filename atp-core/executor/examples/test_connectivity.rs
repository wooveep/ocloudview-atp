/// VDI 平台和 Libvirt 连通性测试
///
/// 功能:
/// 1. 测试 VDI 平台连接
/// 2. 测试本地 Libvirt 连接
///
/// 使用方法:
/// ```bash
/// cd /home/cloudyi/ocloudview-atp/atp-core/executor
/// cargo run --example test_connectivity
/// ```

use atp_executor::TestConfig;
use atp_transport::HostInfo;
use atp_transport::HostConnection;
use reqwest;
use tracing::{info, error, warn};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║         ATP 连通性测试 - VDI 平台 & Libvirt                   ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // 1. 加载配置
    println!("📋 步骤 1/3: 加载测试配置...");
    let config = match TestConfig::load() {
        Ok(c) => {
            println!("   ✅ 配置加载成功");
            c
        }
        Err(e) => {
            eprintln!("   ❌ 配置加载失败: {}", e);
            return Err(e.into());
        }
    };

    println!();

    // 2. 测试 VDI 平台连接
    let vdi_test_result = if let Some(vdi_config) = &config.vdi {
        println!("📡 步骤 2/3: 测试 VDI 平台连接...");
        println!("   📌 VDI 平台地址: {}", vdi_config.base_url);
        println!("   📌 用户名: {}", vdi_config.username);

        match test_vdi_platform(vdi_config).await {
            Ok(_) => {
                println!("   ✅ VDI 平台连接测试通过");
                println!();
                true
            }
            Err(e) => {
                eprintln!("   ❌ VDI 平台连接失败: {}", e);
                eprintln!("   提示: 请检查:");
                eprintln!("      - VDI 平台地址是否正确");
                eprintln!("      - 网络连接是否正常");
                eprintln!("      - 用户名和密码是否正确");
                println!();
                false
            }
        }
    } else {
        warn!("⚠️  未配置 VDI 平台，跳过 VDI 测试");
        println!();
        false
    };

    // 3. 测试 Libvirt 连接
    println!("🔌 步骤 3/3: 测试 Libvirt 连接...");
    println!("   📌 Libvirt URI: {}", config.libvirt.uri);

    let libvirt_test_result = match test_libvirt_connection(&config.libvirt.uri).await {
        Ok(_) => {
            println!("   ✅ Libvirt 连接测试通过");
            true
        }
        Err(e) => {
            eprintln!("   ❌ Libvirt 连接失败: {}", e);
            eprintln!("   提示: 请检查:");
            eprintln!("      - libvirtd 服务是否运行");
            eprintln!("      - URI 配置是否正确");
            eprintln!("      - 如果是远程连接，检查 SSH 配置");
            false
        }
    };

    println!();

    // 测试总结
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                        测试总结                                ║");
    println!("╠════════════════════════════════════════════════════════════════╣");

    if config.vdi.is_some() {
        if vdi_test_result {
            println!("║  VDI 平台连接:     ✅ 成功                                     ║");
        } else {
            println!("║  VDI 平台连接:     ❌ 失败                                     ║");
        }
    } else {
        println!("║  VDI 平台连接:     ⚠️  未配置                                  ║");
    }

    if libvirt_test_result {
        println!("║  Libvirt 连接:     ✅ 成功                                     ║");
    } else {
        println!("║  Libvirt 连接:     ❌ 失败                                     ║");
    }

    println!("╚════════════════════════════════════════════════════════════════╝");

    if vdi_test_result && libvirt_test_result {
        println!("\n✅ 所有测试通过！");
        Ok(())
    } else {
        println!("\n⚠️  部分测试失败，请检查上述提示");
        Err(anyhow::anyhow!("部分测试失败"))
    }
}

/// 测试 VDI 平台连接
async fn test_vdi_platform(vdi_config: &atp_executor::test_config::VdiConfig) -> anyhow::Result<()> {
    // 创建 HTTP 客户端
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(vdi_config.connect_timeout))
        .danger_accept_invalid_certs(!vdi_config.verify_ssl)
        .build()?;

    // 测试 1: 检查服务可用性
    info!("   🔍 检查 VDI 平台服务...");
    let base_url = vdi_config.base_url.trim_end_matches('/');

    match client.get(base_url).send().await {
        Ok(response) => {
            let status = response.status();
            info!("   📡 HTTP 状态: {}", status);

            if status.is_success() || status.is_redirection() {
                println!("   ✅ VDI 平台服务可访问");
            } else {
                warn!("   ⚠️  VDI 平台返回状态: {}", status);
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("无法连接到 VDI 平台: {}", e));
        }
    }

    // 测试 2: 尝试登录认证
    info!("   🔐 测试登录认证...");

    // 构建登录请求
    let login_url = format!("{}/api/login", base_url);
    let mut login_data = HashMap::new();
    login_data.insert("username", vdi_config.username.as_str());
    login_data.insert("password", vdi_config.password.as_str());

    match client
        .post(&login_url)
        .json(&login_data)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            info!("   📡 登录响应状态: {}", status);

            if status.is_success() {
                println!("   ✅ 登录认证成功");

                // 尝试获取响应内容
                if let Ok(text) = response.text().await {
                    info!("   📄 响应内容: {}", &text[..text.len().min(200)]);
                }
            } else {
                // 登录失败可能是API路径不对，但至少说明服务可访问
                warn!("   ⚠️  登录响应: {} (API路径可能需要调整)", status);
                if let Ok(text) = response.text().await {
                    info!("   📄 错误信息: {}", &text[..text.len().min(200)]);
                }
            }
        }
        Err(e) => {
            warn!("   ⚠️  登录请求失败: {} (可能需要实现具体的VDI API)", e);
        }
    }

    // 测试 3: 尝试获取主机列表 (常见 API 路径)
    info!("   🖥️  测试主机列表API...");

    let possible_paths = vec![
        "/api/hosts",
        "/api/v1/hosts",
        "/api/host/list",
    ];

    let mut found_api = false;
    for path in possible_paths {
        let url = format!("{}{}", base_url, path);
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                println!("   ✅ 找到主机列表API: {}", path);
                found_api = true;
                break;
            }
            Ok(response) => {
                info!("   📡 {} 返回: {}", path, response.status());
            }
            Err(_) => {
                // 忽略错误，继续尝试
            }
        }
    }

    if !found_api {
        warn!("   ⚠️  未找到标准的主机列表API (需要根据实际VDI平台实现)");
    }

    Ok(())
}

/// 测试 libvirt 连接
async fn test_libvirt_connection(uri: &str) -> anyhow::Result<()> {
    let host_info = HostInfo {
        id: "test-host".to_string(),
        host: "local".to_string(),
        uri: uri.to_string(),
        tags: vec![],
        metadata: HashMap::new(),
    };

    info!("   🔗 创建连接...");
    let conn = HostConnection::new(host_info);

    info!("   🔌 正在连接到 libvirt...");
    conn.connect().await?;

    println!("   ✅ 连接建立成功");

    // 验证连接状态
    info!("   ✔️  验证连接状态...");
    if conn.is_alive().await {
        println!("   ✅ 连接状态正常");
    } else {
        return Err(anyhow::anyhow!("连接已断开"));
    }

    // 获取虚拟机信息
    if let Ok(conn_mutex) = conn.get_connection().await {
        let conn_guard = conn_mutex.lock().await;
        if let Some(conn_ref) = conn_guard.as_ref() {
            match conn_ref.num_of_domains() {
                Ok(count) => {
                    println!("   📊 虚拟机总数: {}", count);

                    // 列出部分虚拟机
                    if count > 0 {
                        match conn_ref.list_all_domains(0) {
                            Ok(domains) => {
                                println!("   📋 虚拟机列表 (前5个):");
                                for (i, domain) in domains.iter().enumerate().take(5) {
                                    if let Ok(name) = domain.get_name() {
                                        // 获取虚拟机状态
                                        let state = domain.get_state()
                                            .map(|(s, _)| format!("{:?}", s))
                                            .unwrap_or_else(|_| "Unknown".to_string());

                                        println!("      {}. {} (状态: {})", i + 1, name, state);
                                    }
                                }

                                if domains.len() > 5 {
                                    println!("      ... 还有 {} 个虚拟机", domains.len() - 5);
                                }
                            }
                            Err(e) => {
                                warn!("   ⚠️  无法列出虚拟机: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("   ⚠️  无法获取虚拟机数量: {}", e);
                }
            }

            // 获取主机信息
            if let Ok(hostname) = conn_ref.get_hostname() {
                println!("   🖥️  主机名: {}", hostname);
            }

            if let Ok(version) = conn_ref.get_lib_version() {
                println!("   📦 libvirt 版本: {}.{}.{}",
                    version / 1000000,
                    (version % 1000000) / 1000,
                    version % 1000
                );
            }
        }
    }

    Ok(())
}

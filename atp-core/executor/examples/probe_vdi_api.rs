/// VDI API 探测工具
///
/// 用于探测 VDI 平台的实际 API 接口
///
/// 使用方法:
/// ```bash
/// cd /home/cloudyi/ocloudview-atp/atp-core/executor
/// cargo run --example probe_vdi_api
/// ```

use atp_executor::TestConfig;
use reqwest;
use serde_json::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║              VDI API 探测工具                                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // 加载配置
    let config = TestConfig::load()?;
    let vdi_config = config.vdi.as_ref()
        .ok_or_else(|| anyhow::anyhow!("未配置 VDI 平台"))?;

    let base_url = vdi_config.base_url.trim_end_matches('/');
    println!("📌 VDI 平台地址: {}", base_url);
    println!("📌 用户名: {}", vdi_config.username);
    println!();

    // 创建 HTTP 客户端
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(vdi_config.connect_timeout))
        .danger_accept_invalid_certs(!vdi_config.verify_ssl)
        .build()?;

    // 1. 探测 Swagger 文档
    println!("📋 1. 探测 Swagger 文档...");
    let swagger_paths = vec![
        "/doc.html",
        "/swagger-ui.html",
        "/swagger-ui/index.html",
        "/v2/api-docs",
        "/v3/api-docs",
        "/api/swagger.json",
        "/api/swagger.yaml",
    ];

    for path in swagger_paths {
        let url = format!("{}{}", base_url, path);
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                println!("   ✅ 找到 Swagger 文档: {}", path);

                // 尝试获取内容
                if let Ok(text) = resp.text().await {
                    if text.len() < 1000 {
                        println!("   📄 内容预览: {}", &text[..text.len().min(200)]);
                    } else {
                        println!("   📄 文档大小: {} bytes", text.len());
                    }
                }
                break;
            }
            Ok(resp) => {
                println!("   ⚠️  {} - {}", path, resp.status());
            }
            Err(_) => {}
        }
    }
    println!();

    // 2. 探测登录接口
    println!("📋 2. 探测登录接口...");
    let login_paths = vec![
        ("/api/login", "POST"),
        ("/api/auth/login", "POST"),
        ("/api/v1/login", "POST"),
        ("/api/user/login", "POST"),
        ("/login", "POST"),
        ("/auth/login", "POST"),
    ];

    let mut login_data = HashMap::new();
    login_data.insert("username", vdi_config.username.as_str());
    login_data.insert("password", vdi_config.password.as_str());

    for (path, method) in login_paths {
        let url = format!("{}{}", base_url, path);

        let resp = if method == "POST" {
            client.post(&url).json(&login_data).send().await
        } else {
            client.get(&url).send().await
        };

        match resp {
            Ok(response) => {
                let status = response.status();
                println!("   {} {} - 状态: {}", method, path, status);

                if status.is_success() {
                    if let Ok(text) = response.text().await {
                        println!("   ✅ 响应: {}", &text[..text.len().min(300)]);

                        // 尝试解析 JSON
                        if let Ok(json) = serde_json::from_str::<Value>(&text) {
                            println!("   📊 响应结构: {:#}", json);
                        }
                    }
                    break;
                }
            }
            Err(_) => {}
        }
    }
    println!();

    // 3. 探测主机列表接口
    println!("📋 3. 探测主机列表接口...");
    let host_paths = vec![
        "/api/hosts",
        "/api/host/list",
        "/api/v1/hosts",
        "/api/hypervisor/list",
        "/api/node/list",
        "/api/compute/hosts",
    ];

    for path in host_paths {
        let url = format!("{}{}", base_url, path);
        match client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                println!("   GET {} - 状态: {}", path, status);

                if status.is_success() {
                    if let Ok(text) = response.text().await {
                        println!("   ✅ 响应: {}", &text[..text.len().min(300)]);

                        // 尝试解析 JSON
                        if let Ok(json) = serde_json::from_str::<Value>(&text) {
                            println!("   📊 响应结构:");
                            println!("{:#}", json);
                        }
                    }
                    break;
                }
            }
            Err(_) => {}
        }
    }
    println!();

    // 4. 探测虚拟机列表接口
    println!("📋 4. 探测虚拟机列表接口...");
    let vm_paths = vec![
        "/api/domains",
        "/api/domain/list",
        "/api/v1/domains",
        "/api/vm/list",
        "/api/vms",
        "/api/instances",
    ];

    for path in vm_paths {
        let url = format!("{}{}", base_url, path);
        match client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                println!("   GET {} - 状态: {}", path, status);

                if status.is_success() {
                    if let Ok(text) = response.text().await {
                        println!("   ✅ 响应: {}", &text[..text.len().min(300)]);
                    }
                    break;
                }
            }
            Err(_) => {}
        }
    }
    println!();

    // 5. 探测桌面池接口
    println!("📋 5. 探测桌面池接口...");
    let pool_paths = vec![
        "/api/pools",
        "/api/pool/list",
        "/api/v1/pools",
        "/api/deskpool/list",
        "/api/desktop/pools",
    ];

    for path in pool_paths {
        let url = format!("{}{}", base_url, path);
        match client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                println!("   GET {} - 状态: {}", path, status);

                if status.is_success() {
                    if let Ok(text) = response.text().await {
                        println!("   ✅ 响应: {}", &text[..text.len().min(300)]);
                    }
                    break;
                }
            }
            Err(_) => {}
        }
    }
    println!();

    // 6. 通用 API 路径探测
    println!("📋 6. 探测常见 API 路径...");
    let common_paths = vec![
        "/api",
        "/api/v1",
        "/api/health",
        "/api/status",
        "/api/version",
    ];

    for path in common_paths {
        let url = format!("{}{}", base_url, path);
        match client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                println!("   GET {} - {}", path, status);

                if status.is_success() {
                    if let Ok(text) = response.text().await {
                        if text.len() < 500 {
                            println!("      响应: {}", text);
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }
    println!();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    探测完成                                    ║");
    println!("╠════════════════════════════════════════════════════════════════╣");
    println!("║  建议:                                                         ║");
    println!("║  1. 查看上述输出，找到实际的 API 路径                         ║");
    println!("║  2. 访问 Swagger 文档获取完整的 API 说明                      ║");
    println!("║  3. 根据实际 API 更新 VDI 客户端代码                          ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    Ok(())
}

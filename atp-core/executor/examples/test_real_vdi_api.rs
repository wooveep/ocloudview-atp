/// VDI 平台实际 API 测试
///
/// 基于探测到的实际 API 接口进行测试
///
/// 使用方法:
/// ```bash
/// cd /home/cloudyi/ocloudview-atp
/// cargo run --example test_real_vdi_api --manifest-path atp-core/executor/Cargo.toml
/// ```

use atp_executor::TestConfig;
use reqwest;
use serde_json::{json, Value};
use md5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║         VDI 平台 API 实际测试                                  ║");
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

    // 测试 1: 登录认证
    println!("📋 步骤 1/4: 测试登录认证...");
    let login_url = format!("{}/ocloud/v1/login", base_url);

    // MD5 加密密码
    let password_md5 = format!("{:x}", md5::compute(vdi_config.password.as_bytes()));
    println!("   🔐 密码已进行 MD5 加密");

    // 使用管理员登录 API
    let login_data = json!({
        "username": vdi_config.username,
        "password": password_md5,
        "client": ""
    });

    let token = match client
        .post(&login_url)
        .json(&login_data)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            println!("   🔗 POST /ocloud/v1/login");
            println!("   📡 状态: {}", status);

            if status.is_success() {
                let text = response.text().await?;
                println!("   ✅ 登录成功");

                // 解析响应获取 Token
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    println!("   📄 响应数据:");
                    println!("{}", serde_json::to_string_pretty(&json)?);

                    // 检查状态码
                    let response_status = json["status"].as_i64().unwrap_or(-1);
                    if response_status == 0 {
                        // 提取 Token
                        let token = json["data"]["token"]
                            .as_str()
                            .map(|s| s.to_string());

                        if let Some(ref t) = token {
                            println!("   🔑 Token: {}...{}", &t[..t.len().min(20)], if t.len() > 20 { "..." } else { "" });
                            token
                        } else {
                            println!("   ⚠️  未找到 Token");
                            None
                        }
                    } else {
                        println!("   ⚠️  登录失败: {}", json["msg"].as_str().unwrap_or("未知错误"));
                        None
                    }
                } else {
                    println!("   ⚠️  响应不是 JSON 格式");
                    None
                }
            } else {
                let text = response.text().await?;
                println!("   ❌ 登录失败: {}", status);
                println!("   📄 错误信息: {}", text);
                return Err(anyhow::anyhow!("登录失败"));
            }
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
            return Err(e.into());
        }
    };
    println!();

    // 测试 2: 获取主机列表
    println!("📋 步骤 2/4: 获取主机列表...");
    let host_url = format!("{}/ocloud/v1/host?pageNum=1&pageSize=10", base_url);

    let mut request = client.get(&host_url);
    if let Some(ref t) = token {
        request = request.header("Token", t);
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            println!("   🔗 GET /ocloud/v1/host");
            println!("   📡 状态: {}", status);

            if status.is_success() {
                let text = response.text().await?;
                println!("   ✅ 获取成功");

                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    println!("   📄 主机列表响应:");
                    println!("{}", serde_json::to_string_pretty(&json)?);

                    // 提取主机信息
                    if let Some(data) = json["data"]["list"].as_array() {
                        println!("\n   📊 找到 {} 个主机:", data.len());
                        for (i, host) in data.iter().enumerate().take(5) {
                            let name = host["name"].as_str().unwrap_or("未知");
                            let ip = host["ip"].as_str().unwrap_or("未知");
                            let status = host["status"].as_str().unwrap_or("未知");
                            println!("      {}. {} - IP: {} - 状态: {}", i + 1, name, ip, status);
                        }
                    }
                } else {
                    println!("   响应: {}", &text[..text.len().min(500)]);
                }
            } else {
                let text = response.text().await?;
                println!("   ⚠️  获取失败: {}", status);
                println!("   📄 错误: {}", &text[..text.len().min(300)]);
            }
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
        }
    }
    println!();

    // 测试 3: 获取虚拟机列表
    println!("📋 步骤 3/4: 获取虚拟机列表...");
    let domain_url = format!("{}/ocloud/v1/domain?pageNum=1&pageSize=10", base_url);

    let mut request = client.get(&domain_url);
    if let Some(ref t) = token {
        request = request.header("Token", t);
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            println!("   🔗 GET /ocloud/v1/domain");
            println!("   📡 状态: {}", status);

            if status.is_success() {
                let text = response.text().await?;
                println!("   ✅ 获取成功");

                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    println!("   📄 虚拟机列表响应:");
                    println!("{}", serde_json::to_string_pretty(&json)?);

                    // 提取虚拟机信息
                    if let Some(data) = json["data"]["list"].as_array() {
                        println!("\n   📊 找到 {} 个虚拟机:", data.len());
                        for (i, vm) in data.iter().enumerate().take(5) {
                            let name = vm["name"].as_str().unwrap_or("未知");
                            let status = vm["status"].as_str().unwrap_or("未知");
                            println!("      {}. {} - 状态: {}", i + 1, name, status);
                        }
                    }
                } else {
                    println!("   响应: {}", &text[..text.len().min(500)]);
                }
            } else {
                let text = response.text().await?;
                println!("   ⚠️  获取失败: {}", status);
                println!("   📄 错误: {}", &text[..text.len().min(300)]);
            }
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
        }
    }
    println!();

    // 测试 4: 获取桌面池列表
    println!("📋 步骤 4/4: 获取桌面池列表...");
    let pool_url = format!("{}/ocloud/v1/desk-pool?pageNum=1&pageSize=10", base_url);

    let mut request = client.get(&pool_url);
    if let Some(ref t) = token {
        request = request.header("Token", t);
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            println!("   🔗 GET /ocloud/v1/desk-pool");
            println!("   📡 状态: {}", status);

            if status.is_success() {
                let text = response.text().await?;
                println!("   ✅ 获取成功");

                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    println!("   📄 桌面池列表响应:");
                    println!("{}", serde_json::to_string_pretty(&json)?);

                    // 提取桌面池信息
                    if let Some(data) = json["data"]["list"].as_array() {
                        println!("\n   📊 找到 {} 个桌面池:", data.len());
                        for (i, pool) in data.iter().enumerate().take(5) {
                            let name = pool["name"].as_str().unwrap_or("未知");
                            let type_name = pool["type"].as_str().unwrap_or("未知");
                            println!("      {}. {} - 类型: {}", i + 1, name, type_name);
                        }
                    }
                } else {
                    println!("   响应: {}", &text[..text.len().min(500)]);
                }
            } else {
                let text = response.text().await?;
                println!("   ⚠️  获取失败: {}", status);
                println!("   📄 错误: {}", &text[..text.len().min(300)]);
            }
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
        }
    }
    println!();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    测试完成                                    ║");
    println!("╠════════════════════════════════════════════════════════════════╣");
    println!("║  VDI 平台 API 测试已完成                                       ║");
    println!("║  请查看上面的输出了解详细情况                                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    Ok(())
}

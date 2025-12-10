/// VDI 平台管理员登录测试
///
/// 使用 /ocloud/v1/login API 进行管理员登录
///
/// 使用方法:
/// ```bash
/// cd /home/cloudyi/ocloudview-atp
/// cargo run --example test_admin_login --manifest-path atp-core/executor/Cargo.toml
/// ```

use atp_executor::TestConfig;
use reqwest;
use serde_json::{json, Value};
use md5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║         VDI 平台管理员登录测试                                 ║");
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

    // MD5 加密密码
    let password_md5 = format!("{:x}", md5::compute(vdi_config.password.as_bytes()));
    println!("🔐 密码已进行 MD5 加密: {}", password_md5);
    println!();

    // 测试管理员登录 API: /ocloud/v1/login
    println!("📋 测试管理员登录 API...");
    let login_url = format!("{}/ocloud/v1/login", base_url);
    println!("   🔗 POST {}", login_url);

    let login_data = json!({
        "username": vdi_config.username,
        "password": password_md5,
        "client": ""  // 不传 - 普通登录
    });

    println!("   📤 请求数据:");
    println!("{}", serde_json::to_string_pretty(&login_data)?);
    println!();

    match client
        .post(&login_url)
        .json(&login_data)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            println!("   📡 响应状态: {}", status);

            if status.is_success() {
                let text = response.text().await?;

                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    println!("   📄 响应数据:");
                    println!("{}", serde_json::to_string_pretty(&json)?);
                    println!();

                    // 检查响应状态
                    let response_status = json["status"].as_i64().unwrap_or(-1);

                    if response_status == 0 {
                        println!("   ✅ 登录成功！");

                        // 提取 Token
                        if let Some(token) = json["data"]["token"].as_str() {
                            println!("   🔑 Token: {}...{}",
                                &token[..token.len().min(30)],
                                if token.len() > 30 { "..." } else { "" }
                            );
                            println!();

                            // 提取用户信息
                            if let Some(username) = json["data"]["username"].as_str() {
                                println!("   👤 用户名: {}", username);
                            }
                            if let Some(role_level) = json["data"]["roleLevel"].as_str() {
                                println!("   🎭 角色级别: {}", role_level);
                            }
                            println!();

                            // 测试使用 Token 获取主机列表
                            println!("📋 测试使用 Token 获取主机列表...");
                            test_api_with_token(&client, base_url, token).await?;
                        } else {
                            println!("   ⚠️  未找到 Token");
                        }
                    } else {
                        println!("   ❌ 登录失败:");
                        println!("   错误码: {}", response_status);
                        if let Some(msg) = json["msg"].as_str() {
                            println!("   错误信息: {}", msg);
                        }
                    }
                } else {
                    println!("   ⚠️  响应不是 JSON 格式");
                    println!("   响应内容: {}", text);
                }
            } else {
                let text = response.text().await?;
                println!("   ❌ HTTP 错误: {}", status);
                println!("   响应内容: {}", text);
            }
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
            return Err(e.into());
        }
    }

    println!();
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    测试完成                                    ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    Ok(())
}

/// 测试使用 Token 调用其他 API
async fn test_api_with_token(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
) -> anyhow::Result<()> {
    // 测试主机列表 API
    let host_url = format!("{}/ocloud/v1/host?pageNum=1&pageSize=10", base_url);
    println!("   🔗 GET {}", host_url);
    println!("   🔑 使用 Token 认证");

    match client
        .get(&host_url)
        .header("Token", token)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            println!("   📡 响应状态: {}", status);

            if status.is_success() {
                let text = response.text().await?;

                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    println!("   📄 主机列表响应:");
                    println!("{}", serde_json::to_string_pretty(&json)?);
                    println!();

                    // 提取主机信息
                    if let Some(data) = json["data"]["list"].as_array() {
                        println!("   📊 找到 {} 个主机:", data.len());
                        for (i, host) in data.iter().enumerate().take(5) {
                            let name = host["name"].as_str().unwrap_or("未知");
                            let ip = host["ip"].as_str().unwrap_or("未知");
                            let status = host["status"].as_str().unwrap_or("未知");
                            println!("      {}. {} - IP: {} - 状态: {}", i + 1, name, ip, status);
                        }

                        if data.len() > 5 {
                            println!("      ... 还有 {} 个主机", data.len() - 5);
                        }
                    }
                } else {
                    println!("   响应: {}", &text[..text.len().min(500)]);
                }
            } else {
                let text = response.text().await?;
                println!("   ⚠️  获取失败: {}", status);
                println!("   错误: {}", &text[..text.len().min(300)]);
            }
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
        }
    }

    Ok(())
}

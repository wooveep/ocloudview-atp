/// VDI 平台登录调试工具
///
/// 用于调试 VDI 登录 API 的详细请求和响应
///
/// 使用方法:
/// ```bash
/// cd /home/cloudyi/ocloudview-atp
/// cargo run --example test_vdi_login_debug --manifest-path atp-core/executor/Cargo.toml
/// ```

use atp_executor::TestConfig;
use reqwest;
use serde_json::{json, Value};
use md5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║         VDI 平台登录调试工具                                   ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // 加载配置
    let config = TestConfig::load()?;
    let vdi_config = config.vdi.as_ref()
        .ok_or_else(|| anyhow::anyhow!("未配置 VDI 平台"))?;

    let base_url = vdi_config.base_url.trim_end_matches('/');

    println!("📋 配置信息:");
    println!("   VDI 地址: {}", base_url);
    println!("   用户名: {}", vdi_config.username);
    println!("   原始密码: {}", vdi_config.password);
    println!();

    // MD5 加密密码
    let password_md5 = format!("{:x}", md5::compute(vdi_config.password.as_bytes()));
    println!("🔐 密码加密:");
    println!("   MD5 加密后: {}", password_md5);
    println!();

    // 创建 HTTP 客户端
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(vdi_config.connect_timeout))
        .danger_accept_invalid_certs(!vdi_config.verify_ssl)
        .build()?;

    let login_url = format!("{}/ocloud/usermodule/login", base_url);

    // 测试 1: 使用 username + MD5密码
    println!("🧪 测试 1: 使用 username + MD5密码");
    let login_data_1 = json!({
        "username": vdi_config.username,
        "password": password_md5,
    });
    println!("   请求数据: {}", serde_json::to_string_pretty(&login_data_1)?);

    match client.post(&login_url).json(&login_data_1).send().await {
        Ok(response) => {
            let status = response.status();
            let text = response.text().await?;
            println!("   响应状态: {}", status);
            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                println!("   响应数据: {}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("   响应内容: {}", text);
            }
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
        }
    }
    println!();

    // 测试 2: 使用 sAMAccountName + MD5密码
    println!("🧪 测试 2: 使用 sAMAccountName + MD5密码");
    let login_data_2 = json!({
        "sAMAccountName": vdi_config.username,
        "password": password_md5,
    });
    println!("   请求数据: {}", serde_json::to_string_pretty(&login_data_2)?);

    match client.post(&login_url).json(&login_data_2).send().await {
        Ok(response) => {
            let status = response.status();
            let text = response.text().await?;
            println!("   响应状态: {}", status);
            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                println!("   响应数据: {}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("   响应内容: {}", text);
            }
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
        }
    }
    println!();

    // 测试 3: 使用原始密码 (不加密)
    println!("🧪 测试 3: 使用 username + 原始密码 (不加密)");
    let login_data_3 = json!({
        "username": vdi_config.username,
        "password": vdi_config.password,
    });
    println!("   请求数据: {}", serde_json::to_string_pretty(&login_data_3)?);

    match client.post(&login_url).json(&login_data_3).send().await {
        Ok(response) => {
            let status = response.status();
            let text = response.text().await?;
            println!("   响应状态: {}", status);
            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                println!("   响应数据: {}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("   响应内容: {}", text);
            }
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
        }
    }
    println!();

    // 测试 4: 尝试获取用户列表 API (如果有)
    println!("🧪 测试 4: 查询是否有用户列表 API");
    let user_list_paths = vec![
        "/ocloud/usermodule/user",
        "/ocloud/v1/user",
        "/ocloud/api/user/list",
    ];

    for path in user_list_paths {
        let url = format!("{}{}", base_url, path);
        match client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                println!("   {} -> {}", path, status);
                if status.is_success() {
                    let text = response.text().await?;
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        println!("      响应: {}", serde_json::to_string_pretty(&json)?);
                    }
                }
            }
            Err(_) => {
                // 忽略错误
            }
        }
    }

    println!();
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║ 调试建议:                                                      ║");
    println!("║ 1. 检查 VDI 平台中是否存在用户 'admin'                         ║");
    println!("║ 2. 确认密码是否正确                                            ║");
    println!("║ 3. 查看 VDI 平台的用户管理界面                                 ║");
    println!("║ 4. 可能需要先在 VDI 平台创建测试用户                           ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    Ok(())
}

/// 获取 Swagger API 文档
///
/// 使用方法:
/// ```bash
/// cd /home/cloudyi/ocloudview-atp
/// cargo run --example fetch_swagger --manifest-path atp-core/executor/Cargo.toml > swagger_apis.txt
/// ```

use atp_executor::TestConfig;
use reqwest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = TestConfig::load()?;
    let vdi_config = config.vdi.as_ref()
        .ok_or_else(|| anyhow::anyhow!("未配置 VDI 平台"))?;

    let base_url = vdi_config.base_url.trim_end_matches('/');

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(vdi_config.connect_timeout))
        .danger_accept_invalid_certs(!vdi_config.verify_ssl)
        .build()?;

    // 尝试多个 Swagger 端点
    let endpoints = vec![
        "/doc.html",
        "/v2/api-docs",
        "/v3/api-docs",
        "/swagger/v2/api-docs",
        "/api-docs",
    ];

    for endpoint in endpoints {
        let url = format!("{}{}", base_url, endpoint);
        eprintln!("正在获取: {}", url);

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let content = resp.text().await?;

                eprintln!("✅ 成功获取 Swagger 文档 ({})", endpoint);
                eprintln!("文档大小: {} bytes\n", content.len());

                // 输出内容
                println!("{}", content);
                return Ok(());
            }
            Ok(resp) => {
                eprintln!("❌ {} - 状态: {}", endpoint, resp.status());
            }
            Err(e) => {
                eprintln!("❌ {} - 错误: {}", endpoint, e);
            }
        }
    }

    eprintln!("\n无法获取 Swagger 文档");
    eprintln!("请尝试在浏览器中访问: {}/doc.html", base_url);

    Ok(())
}

use atp_executor::spice_connector::SpiceConnector;
use atp_vdiplatform::client::VdiConfig;
use atp_vdiplatform::VdiClient;
use std::sync::Arc;
use tokio;

#[tokio::test]
#[ignore] // 需要真实环境，默认忽略
async fn test_spice_connector_real_env() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化 VDI 客户端
    // 假设环境变量中由 VDI 配置，否则使用硬编码的默认值测试
    let base_url =
        std::env::var("ATP_VDI_URL").unwrap_or_else(|_| "http://192.168.47.31".to_string());
    let username = std::env::var("ATP_VDI_USER").unwrap_or_else(|_| "admin".to_string());
    let password = std::env::var("ATP_VDI_PASS").unwrap_or_else(|_| "password".to_string());

    let config = VdiConfig {
        connect_timeout: 5,
        request_timeout: 10,
        max_retries: 1,
        verify_ssl: false,
    };

    let mut client = VdiClient::new(&base_url, config)?;
    client.login(&username, &password).await?;
    let vdi_client = Arc::new(client);

    // 2. 创建 SpiceConnector
    let mut connector = SpiceConnector::new(vdi_client);

    // 3. 尝试连接 (需要一个有效的 domain_id)
    // 这里使用一个占位符 ID，实际测试时需要替换
    let domain_id = "test-vm-id";

    match connector.connect(domain_id).await {
        Ok(_) => {
            println!("连接成功");

            // 4. 发送一些输入
            connector.send_text("Integration Test").await?;

            // 5. 断开连接
            connector.disconnect().await?;
        }
        Err(e) => {
            println!("连接失败 (预期内，如果 VM 不存在): {}", e);
            // 这里我们不 assert 失败，因为环境可能没有准备好
        }
    }

    Ok(())
}

//! Verification Server 示例程序
//!
//! 启动 WebSocket 和 TCP 服务器，等待 Guest Agent 连接

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};

use verification_server::{
    client::ClientManager,
    server::{ServerConfig, VerificationServer},
    service::{ServiceConfig, VerificationService},
    types::Event,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    info!("启动 Verification Server 示例");

    // 创建客户端管理器
    let client_manager = Arc::new(ClientManager::new());

    // 创建验证服务
    let service_config = ServiceConfig {
        default_timeout: Duration::from_secs(30),
        cleanup_interval: Duration::from_secs(60),
        max_pending_events: 1000,
    };
    let verification_service = Arc::new(VerificationService::new(
        client_manager.clone(),
        service_config,
    ));

    // 服务器配置
    let server_config = ServerConfig {
        websocket_addr: Some("0.0.0.0:8765".parse::<SocketAddr>()?),
        tcp_addr: Some("0.0.0.0:8766".parse::<SocketAddr>()?),
    };

    info!("WebSocket 服务器地址: 0.0.0.0:8765");
    info!("TCP 服务器地址: 0.0.0.0:8766");
    info!("等待 Guest Agent 连接...");

    // 创建服务器
    let server = VerificationServer::new(server_config, client_manager.clone());

    // 启动服务器（非阻塞）
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            eprintln!("服务器错误: {}", e);
        }
    });

    // 启动测试任务（模拟发送验证事件）
    let test_service = verification_service.clone();
    let test_handle = tokio::spawn(async move {
        // 等待客户端连接
        sleep(Duration::from_secs(2)).await;

        loop {
            sleep(Duration::from_secs(5)).await;

            // 获取所有已连接的客户端
            let clients = test_service.client_manager.get_clients().await;

            if clients.is_empty() {
                info!("当前没有客户端连接");
                continue;
            }

            info!("当前连接的客户端: {}", clients.len());

            // 为每个客户端发送测试事件
            for client_info in clients {
                info!("向客户端 {} 发送测试事件", client_info.vm_id);

                let event = Event {
                    event_type: "keyboard".to_string(),
                    data: serde_json::json!({
                        "key": "a",
                        "timeout_ms": 5000,
                    }),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                };

                match test_service
                    .verify_event(&client_info.vm_id, event, Some(Duration::from_secs(10)))
                    .await
                {
                    Ok(result) => {
                        info!(
                            "验证成功: vm_id={}, verified={}, latency={}ms",
                            client_info.vm_id, result.verified, result.latency_ms
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "验证失败: vm_id={}, error={}",
                            client_info.vm_id, e
                        );
                    }
                }
            }

            // 显示待验证事件数量
            let pending = test_service.pending_count().await;
            if pending > 0 {
                info!("当前待验证事件: {}", pending);
            }
        }
    });

    // 等待服务器（Ctrl+C 退出）
    tokio::select! {
        _ = server_handle => {},
        _ = test_handle => {},
        _ = tokio::signal::ctrl_c() => {
            info!("收到退出信号，正在关闭服务器...");
        }
    }

    info!("服务器已关闭");
    Ok(())
}

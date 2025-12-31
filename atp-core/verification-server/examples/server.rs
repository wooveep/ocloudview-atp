//! Verification Server 示例程序
//!
//! 新架构（输入上报模式）：
//! 1. 启动 WebSocket 和 TCP 服务器，等待 Guest Agent 连接
//! 2. Guest Agent 在 VM 内部捕获输入事件并上报 RawInputEvent
//! 3. 外部系统通过 SPICE/QMP 注入输入到 VM
//! 4. 服务端调用 expect_input() 注册期望事件，等待 Agent 上报匹配

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, Level};

use verification_server::{
    client::ClientManager,
    server::{ServerConfig, VerificationServer},
    service::{ServiceConfig, VerificationService},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    info!("启动 Verification Server 示例（输入上报模式）");

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
    info!("");
    info!("使用说明：");
    info!("  1. 在 VM 内启动 verifier-agent 连接到本服务");
    info!("  2. 通过 SPICE/QMP 向 VM 注入按键（如按下 'A' 键）");
    info!("  3. Agent 捕获并上报事件，服务端完成验证");
    info!("");

    // 创建服务器
    let server = VerificationServer::new(server_config, client_manager.clone());

    // 启动服务器（非阻塞）
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            eprintln!("服务器错误: {}", e);
        }
    });

    // 启动测试任务（演示输入验证流程）
    let test_service = verification_service.clone();
    let test_handle = tokio::spawn(async move {
        // 等待客户端连接
        sleep(Duration::from_secs(2)).await;

        loop {
            sleep(Duration::from_secs(5)).await;

            // 获取所有已连接的客户端
            let clients = test_service.client_manager.get_clients().await;

            if clients.is_empty() {
                info!("当前没有客户端连接，等待中...");
                continue;
            }

            info!("当前连接的客户端: {}", clients.len());

            // 为每个客户端演示期望输入验证
            for client_info in clients {
                info!(
                    "为 VM {} 注册期望键盘输入: 'A' 键按下",
                    client_info.vm_id
                );
                info!("请在 5 秒内通过 SPICE/QMP 向 VM 注入 'A' 键...");

                // 注册期望输入：等待 'A' 键被按下 (value=1)
                // 此时应通过外部系统（SPICE/QMP）向 VM 注入按键
                match test_service
                    .expect_input(
                        &client_info.vm_id,
                        "keyboard",      // 事件类型
                        "A",             // 期望按键名称
                        Some(1),         // value=1 表示按下
                        Some(Duration::from_secs(5)),
                    )
                    .await
                {
                    Ok(result) => {
                        info!(
                            "✓ 验证成功: vm_id={}, latency={}ms, details={:?}",
                            client_info.vm_id, result.latency_ms, result.details
                        );
                    }
                    Err(e) => {
                        warn!(
                            "✗ 验证超时或失败: vm_id={}, error={}",
                            client_info.vm_id, e
                        );
                        info!("提示: 确保在超时前向 VM 注入了对应按键");
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

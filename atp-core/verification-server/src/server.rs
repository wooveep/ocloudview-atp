//! WebSocket 和 TCP 服务器实现

use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

use crate::client::ClientManager;
use crate::types::{ClientInfo, Event, VerifyResult};
use crate::Result;

/// 验证服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// WebSocket 服务器地址
    pub websocket_addr: Option<SocketAddr>,

    /// TCP 服务器地址
    pub tcp_addr: Option<SocketAddr>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            websocket_addr: Some("0.0.0.0:8765".parse().unwrap()),
            tcp_addr: Some("0.0.0.0:8766".parse().unwrap()),
        }
    }
}

/// 验证服务器
pub struct VerificationServer {
    config: ServerConfig,
    client_manager: Arc<ClientManager>,
}

impl VerificationServer {
    /// 创建新的验证服务器
    pub fn new(config: ServerConfig, client_manager: Arc<ClientManager>) -> Self {
        Self {
            config,
            client_manager,
        }
    }

    /// 启动服务器
    pub async fn start(&self) -> Result<()> {
        let mut tasks = Vec::new();

        // 启动 WebSocket 服务器
        if let Some(addr) = self.config.websocket_addr {
            let client_manager = self.client_manager.clone();
            tasks.push(tokio::spawn(async move {
                if let Err(e) = run_websocket_server(addr, client_manager).await {
                    error!("WebSocket 服务器错误: {}", e);
                }
            }));
        }

        // 启动 TCP 服务器
        if let Some(addr) = self.config.tcp_addr {
            let client_manager = self.client_manager.clone();
            tasks.push(tokio::spawn(async move {
                if let Err(e) = run_tcp_server(addr, client_manager).await {
                    error!("TCP 服务器错误: {}", e);
                }
            }));
        }

        // 等待所有任务
        for task in tasks {
            let _ = task.await;
        }

        Ok(())
    }
}

/// 运行 WebSocket 服务器
async fn run_websocket_server(addr: SocketAddr, client_manager: Arc<ClientManager>) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("WebSocket 服务器启动: {}", addr);

    while let Ok((stream, peer_addr)) = listener.accept().await {
        let client_manager = client_manager.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_websocket_client(stream, peer_addr, client_manager).await {
                error!("WebSocket 客户端处理错误 ({}): {}", peer_addr, e);
            }
        });
    }

    Ok(())
}

/// 处理 WebSocket 客户端连接
async fn handle_websocket_client(
    stream: TcpStream,
    peer_addr: SocketAddr,
    client_manager: Arc<ClientManager>,
) -> Result<()> {
    debug!("WebSocket 客户端连接: {}", peer_addr);

    // WebSocket 握手
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("WebSocket 握手失败: {}", e))
        })?;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // 等待客户端发送 VM ID (第一条消息)
    let vm_id = match ws_receiver.next().await {
        Some(Ok(Message::Text(text))) => {
            debug!("收到 VM ID: {}", text);
            text
        }
        _ => {
            warn!("客户端未发送 VM ID: {}", peer_addr);
            return Ok(());
        }
    };

    // 注册客户端
    let info = ClientInfo {
        vm_id: vm_id.clone(),
        connected_at: chrono::Utc::now(),
        remote_addr: Some(peer_addr.to_string()),
    };

    let mut event_rx = client_manager.register_client(info).await?;
    let result_tx = client_manager.get_result_sender();

    info!("WebSocket 客户端已注册: {} ({})", vm_id, peer_addr);

    // 双向消息转发
    loop {
        tokio::select! {
            // 从服务端接收事件，发送给客户端
            Some(event) = event_rx.recv() => {
                let json = serde_json::to_string(&event)?;
                if let Err(e) = ws_sender.send(Message::Text(json)).await {
                    error!("发送事件到客户端失败: {}", e);
                    break;
                }
            }

            // 从客户端接收结果
            Some(msg) = ws_receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<VerifyResult>(&text) {
                            Ok(result) => {
                                debug!("收到验证结果: event_id={}", result.event_id);
                                if result_tx.send(result).is_err() {
                                    error!("转发验证结果失败");
                                }
                            }
                            Err(e) => {
                                warn!("解析验证结果失败: {}", e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("客户端关闭连接: {}", vm_id);
                        break;
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                        // 忽略心跳
                    }
                    Err(e) => {
                        error!("接收消息错误: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            else => {
                break;
            }
        }
    }

    client_manager.unregister_client(&vm_id).await;
    info!("WebSocket 客户端断开: {}", vm_id);

    Ok(())
}

/// 运行 TCP 服务器
async fn run_tcp_server(addr: SocketAddr, client_manager: Arc<ClientManager>) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("TCP 服务器启动: {}", addr);

    while let Ok((stream, peer_addr)) = listener.accept().await {
        let client_manager = client_manager.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_tcp_client(stream, peer_addr, client_manager).await {
                error!("TCP 客户端处理错误 ({}): {}", peer_addr, e);
            }
        });
    }

    Ok(())
}

/// 处理 TCP 客户端连接
async fn handle_tcp_client(
    stream: TcpStream,
    peer_addr: SocketAddr,
    client_manager: Arc<ClientManager>,
) -> Result<()> {
    debug!("TCP 客户端连接: {}", peer_addr);

    // 拆分读写（使用 into_split 获得所有权）
    let (mut read_half, mut write_half) = stream.into_split();

    // 读取 VM ID (长度前缀格式: 4 字节长度 + 字符串)
    let vm_id_len = read_half.read_u32().await? as usize;
    if vm_id_len > 256 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "VM ID 过长",
        )
        .into());
    }

    let mut vm_id_bytes = vec![0u8; vm_id_len];
    read_half.read_exact(&mut vm_id_bytes).await?;
    let vm_id = String::from_utf8(vm_id_bytes).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "VM ID 不是有效的 UTF-8")
    })?;

    debug!("收到 VM ID: {}", vm_id);

    // 注册客户端
    let info = ClientInfo {
        vm_id: vm_id.clone(),
        connected_at: chrono::Utc::now(),
        remote_addr: Some(peer_addr.to_string()),
    };

    let mut event_rx = client_manager.register_client(info).await?;
    let result_tx = client_manager.get_result_sender();

    info!("TCP 客户端已注册: {} ({})", vm_id, peer_addr);

    // 创建通道用于发送任务和接收任务通信
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

    // 发送任务
    let send_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let json = match serde_json::to_string(&event) {
                Ok(j) => j,
                Err(e) => {
                    error!("序列化事件失败: {}", e);
                    continue;
                }
            };

            // 发送长度
            let len = json.len() as u32;
            if write_half.write_u32(len).await.is_err() {
                break;
            }

            // 发送数据
            if write_half.write_all(json.as_bytes()).await.is_err() {
                break;
            }

            if write_half.flush().await.is_err() {
                break;
            }
        }
    });

    // 接收任务
    let recv_task = tokio::spawn(async move {
        loop {
            // 读取长度
            let len = match read_half.read_u32().await {
                Ok(l) => l as usize,
                Err(_) => break,
            };

            if len > 10 * 1024 * 1024 {
                error!("消息过大: {} bytes", len);
                break;
            }

            // 读取数据
            let mut buffer = vec![0u8; len];
            if read_half.read_exact(&mut buffer).await.is_err() {
                break;
            }

            let json = match String::from_utf8(buffer) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // 解析结果
            match serde_json::from_str::<VerifyResult>(&json) {
                Ok(result) => {
                    if result_tx.send(result).is_err() {
                        error!("转发验证结果失败");
                    }
                }
                Err(e) => {
                    warn!("解析验证结果失败: {}", e);
                }
            }
        }

        let _ = shutdown_tx.send(()).await;
    });

    // 等待任一任务完成
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
        _ = shutdown_rx.recv() => {},
    }

    client_manager.unregister_client(&vm_id).await;
    info!("TCP 客户端断开: {}", vm_id);

    Ok(())
}

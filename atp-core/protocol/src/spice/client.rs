//! SPICE 客户端
//!
//! 提供 SPICE 多通道管理和高级 API。

use crate::{ProtocolError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::channel::{ChannelConnection, ChannelType};
use super::display::DisplayChannel;
use super::inputs::InputsChannel;
use super::messages::*;
use super::usbredir::UsbRedirChannel;

/// SPICE 客户端配置
#[derive(Debug, Clone)]
pub struct SpiceConfig {
    /// 服务器地址
    pub host: String,
    /// SPICE 端口
    pub port: u16,
    /// TLS 端口
    pub tls_port: Option<u16>,
    /// 密码
    pub password: Option<String>,
    /// 是否使用 TLS
    pub use_tls: bool,
    /// 自动连接显示通道
    pub auto_display: bool,
    /// 自动连接输入通道
    pub auto_inputs: bool,
    /// 请求客户端鼠标模式
    pub request_client_mouse: bool,
}

impl SpiceConfig {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            tls_port: None,
            password: None,
            use_tls: false,
            auto_display: true,
            auto_inputs: true,
            request_client_mouse: true,
        }
    }

    pub fn with_tls_port(mut self, tls_port: Option<u16>) -> Self {
        self.tls_port = tls_port;
        if tls_port.is_some() {
            self.use_tls = true;
        }
        self
    }

    pub fn with_password(mut self, password: &str) -> Self {
        self.password = Some(password.to_string());
        self
    }

    pub fn with_password_opt(mut self, password: Option<String>) -> Self {
        self.password = password;
        self
    }

    pub fn with_use_tls(mut self, use_tls: bool) -> Self {
        self.use_tls = use_tls;
        self
    }

    pub fn with_auto_display(mut self, auto_display: bool) -> Self {
        self.auto_display = auto_display;
        self
    }

    pub fn with_auto_inputs(mut self, auto_inputs: bool) -> Self {
        self.auto_inputs = auto_inputs;
        self
    }

    pub fn with_client_mouse(mut self, client_mouse: bool) -> Self {
        self.request_client_mouse = client_mouse;
        self
    }
}

/// SPICE 客户端状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// 未连接
    Disconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 断开中
    Disconnecting,
}

/// SPICE 客户端
///
/// 管理多个 SPICE 通道的连接
pub struct SpiceClient {
    /// 配置
    config: SpiceConfig,
    /// 客户端状态
    state: ClientState,
    /// 会话 ID (由服务器分配)
    session_id: u32,
    /// 当前鼠标模式
    mouse_mode: u32,
    /// 主通道
    main_channel: Option<ChannelConnection>,
    /// 输入通道
    inputs_channel: Option<InputsChannel>,
    /// 显示通道
    display_channels: HashMap<u8, DisplayChannel>,
    /// USB 重定向通道
    usbredir_channels: HashMap<u8, UsbRedirChannel>,
    /// 可用通道列表
    available_channels: Vec<(u8, u8)>, // (type, id)
}

impl SpiceClient {
    /// 创建新的 SPICE 客户端
    pub fn new(config: SpiceConfig) -> Self {
        Self {
            config,
            state: ClientState::Disconnected,
            session_id: 0,
            mouse_mode: 0,
            main_channel: None,
            inputs_channel: None,
            display_channels: HashMap::new(),
            usbredir_channels: HashMap::new(),
            available_channels: Vec::new(),
        }
    }

    /// 连接到 SPICE 服务器
    pub async fn connect(&mut self) -> Result<()> {
        if self.state != ClientState::Disconnected {
            return Err(ProtocolError::ConnectionFailed(
                "客户端已连接或正在连接".to_string(),
            ));
        }

        self.state = ClientState::Connecting;
        info!(
            "连接到 SPICE 服务器: {}:{}",
            self.config.host, self.config.port
        );

        // TODO: 添加 TLS 支持
        //
        // 如果 config.use_tls 为 true，需要使用 TLS 加密连接
        //
        // 实现步骤：
        // 1. 选择端口（TLS 或普通）
        //    ```rust
        //    let port = if self.config.use_tls {
        //        self.config.tls_port.unwrap_or(self.config.port)
        //    } else {
        //        self.config.port
        //    };
        //    ```
        //
        // 2. 建立 TCP 连接
        //    ```rust
        //    let addr = format!("{}:{}", self.config.host, port);
        //    let tcp_stream = TcpStream::connect(&addr).await
        //        .map_err(|e| ProtocolError::ConnectionFailed(format!("TCP 连接失败: {}", e)))?;
        //    ```
        //
        // 3. 如果使用 TLS，进行 TLS 握手
        //    使用 tokio-native-tls 或 tokio-rustls crate:
        //    ```rust
        //    if self.config.use_tls {
        //        use tokio_native_tls::{TlsConnector, native_tls};
        //
        //        // 创建 TLS 配置（可选择是否验证证书）
        //        let mut tls_builder = native_tls::TlsConnector::builder();
        //        // 对于自签名证书，可能需要禁用验证（仅用于测试）
        //        // tls_builder.danger_accept_invalid_certs(true);
        //        // tls_builder.danger_accept_invalid_hostnames(true);
        //
        //        let tls_connector = TlsConnector::from(tls_builder.build()
        //            .map_err(|e| ProtocolError::ConnectionFailed(format!("TLS 初始化失败: {}", e)))?);
        //
        //        // 执行 TLS 握手
        //        let tls_stream = tls_connector.connect(&self.config.host, tcp_stream).await
        //            .map_err(|e| ProtocolError::ConnectionFailed(format!("TLS 握手失败: {}", e)))?;
        //
        //        debug!("TLS 连接建立成功");
        //        // 之后将 tls_stream 传递给 ChannelConnection
        //        // 注意：需要修改 ChannelConnection 以支持泛型 AsyncRead + AsyncWrite trait
        //    }
        //    ```
        //
        // 4. 或者使用 rustls（更现代的选择）
        //    ```rust
        //    use tokio_rustls::{TlsConnector, rustls};
        //    use std::sync::Arc;
        //
        //    let mut root_store = rustls::RootCertStore::empty();
        //    // 加载系统证书
        //    root_store.add_server_trust_anchors(
        //        webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        //            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
        //                ta.subject, ta.spki, ta.name_constraints,
        //            )
        //        })
        //    );
        //
        //    let config = rustls::ClientConfig::builder()
        //        .with_safe_defaults()
        //        .with_root_certificates(root_store)
        //        .with_no_client_auth();
        //
        //    let connector = TlsConnector::from(Arc::new(config));
        //    let domain = rustls::ServerName::try_from(self.config.host.as_str())
        //        .map_err(|_| ProtocolError::ConnectionFailed("无效的域名".to_string()))?;
        //
        //    let tls_stream = connector.connect(domain, tcp_stream).await
        //        .map_err(|e| ProtocolError::ConnectionFailed(format!("TLS 握手失败: {}", e)))?;
        //    ```
        //
        // 5. 修改 ChannelConnection 以接受 TLS stream
        //    需要将 ChannelConnection::connect() 改为接受 Box<dyn AsyncRead + AsyncWrite + Unpin + Send>
        //    或者使用枚举包装 TcpStream 和 TlsStream
        //
        // 需要添加的依赖（选择一种）：
        //   方案 A (native-tls):
        //     tokio-native-tls = "0.3"
        //     native-tls = "0.2"
        //
        //   方案 B (rustls，推荐):
        //     tokio-rustls = "0.24"
        //     rustls = "0.21"
        //     webpki-roots = "0.25"
        //
        // 注意事项：
        //   - SPICE 服务器通常使用自签名证书
        //   - 生产环境应验证证书，测试环境可考虑禁用验证
        //   - TLS 连接建立后，SPICE 协议握手流程与非 TLS 相同
        //

        // 1. 连接主通道
        let mut main_channel = ChannelConnection::new(ChannelType::Main, 0);
        main_channel
            .connect(
                &self.config.host,
                self.config.port,
                0, // 初始连接 ID 为 0
                self.config.password.as_deref(),
            )
            .await?;

        // 2. 处理主通道初始化
        self.handle_main_init(&mut main_channel).await?;

        self.main_channel = Some(main_channel);
        self.state = ClientState::Connected;

        // 3. 自动连接其他通道
        if self.config.auto_inputs {
            if let Err(e) = self.connect_inputs().await {
                warn!("自动连接输入通道失败: {}", e);
            }
        }

        if self.config.auto_display {
            if let Err(e) = self.connect_display(0).await {
                warn!("自动连接显示通道失败: {}", e);
            }
        }

        info!("SPICE 客户端连接成功, session_id={}", self.session_id);
        Ok(())
    }

    /// 处理主通道初始化
    async fn handle_main_init(&mut self, channel: &mut ChannelConnection) -> Result<()> {
        loop {
            let (msg_type, data) = channel.receive_message().await?;

            match msg_type {
                // SPICE_MSG_MAIN_INIT
                103 => {
                    if let Some(init) = MsgMainInit::from_bytes(&data) {
                        self.session_id = init.session_id;
                        self.mouse_mode = init.current_mouse_mode;
                        debug!(
                            "收到 Main Init: session_id={}, mouse_mode={}",
                            init.session_id, init.current_mouse_mode
                        );

                        // 请求客户端鼠标模式
                        if self.config.request_client_mouse && (init.supported_mouse_modes & 2) != 0
                        {
                            let req = MsgcMainMouseModeRequest::new(2);
                            channel.send_message(105, &req.to_bytes()).await?;
                        }
                    }
                }
                // SPICE_MSG_MAIN_CHANNELS_LIST
                104 => {
                    if let Some(list) = MsgMainChannelsList::from_bytes(&data) {
                        debug!("收到通道列表: {} 个通道", list.channels.len());
                        for ch in &list.channels {
                            self.available_channels
                                .push((ch.channel_type, ch.channel_id));
                        }
                    }
                    // 通道列表之后初始化完成
                    break;
                }
                // SPICE_MSG_MAIN_MOUSE_MODE
                105 => {
                    if let Some(mode) = MsgMainMouseMode::from_bytes(&data) {
                        self.mouse_mode = mode.current_mode;
                        debug!("鼠标模式更新: {}", mode.current_mode);
                    }
                }
                // SPICE_MSG_PING
                4 => {
                    // 回复 PONG
                    channel.send_message(3, &data).await?;
                }
                _ => {
                    debug!("忽略主通道消息: type={}", msg_type);
                }
            }
        }

        Ok(())
    }

    /// 连接输入通道
    pub async fn connect_inputs(&mut self) -> Result<()> {
        if self.inputs_channel.is_some() {
            return Ok(());
        }

        let mut inputs = InputsChannel::new(0);
        inputs
            .connect(
                &self.config.host,
                self.config.port,
                self.session_id,
                self.config.password.as_deref(),
            )
            .await?;

        self.inputs_channel = Some(inputs);
        info!("输入通道已连接");
        Ok(())
    }

    /// 连接显示通道
    pub async fn connect_display(&mut self, id: u8) -> Result<()> {
        if self.display_channels.contains_key(&id) {
            return Ok(());
        }

        let mut display = DisplayChannel::new(id);
        display
            .connect(
                &self.config.host,
                self.config.port,
                self.session_id,
                self.config.password.as_deref(),
            )
            .await?;

        self.display_channels.insert(id, display);
        info!("显示通道 {} 已连接", id);
        Ok(())
    }

    /// 连接 USB 重定向通道
    pub async fn connect_usbredir(&mut self, id: u8) -> Result<()> {
        if self.usbredir_channels.contains_key(&id) {
            return Ok(());
        }

        let mut usbredir = UsbRedirChannel::new(id);
        usbredir
            .connect(
                &self.config.host,
                self.config.port,
                self.session_id,
                self.config.password.as_deref(),
            )
            .await?;

        self.usbredir_channels.insert(id, usbredir);
        info!("USB 重定向通道 {} 已连接", id);
        Ok(())
    }

    /// 断开所有连接
    pub async fn disconnect(&mut self) -> Result<()> {
        if self.state == ClientState::Disconnected {
            return Ok(());
        }

        self.state = ClientState::Disconnecting;
        info!("断开 SPICE 连接");

        // 断开各通道
        if let Some(ref mut inputs) = self.inputs_channel {
            let _ = inputs.disconnect().await;
        }
        self.inputs_channel = None;

        for (_, display) in self.display_channels.iter_mut() {
            let _ = display.disconnect().await;
        }
        self.display_channels.clear();

        for (_, usbredir) in self.usbredir_channels.iter_mut() {
            let _ = usbredir.disconnect().await;
        }
        self.usbredir_channels.clear();

        if let Some(ref mut main) = self.main_channel {
            let _ = main.disconnect().await;
        }
        self.main_channel = None;

        self.state = ClientState::Disconnected;
        self.session_id = 0;
        self.available_channels.clear();

        info!("SPICE 连接已断开");
        Ok(())
    }

    /// 获取输入通道引用
    ///
    /// # Returns
    /// - `Some(&InputsChannel)` 如果输入通道已连接
    /// - `None` 如果输入通道未连接
    pub fn inputs(&self) -> Option<&InputsChannel> {
        self.inputs_channel.as_ref()
    }

    /// 获取输入通道可变引用
    ///
    /// # Returns
    /// - `Some(&mut InputsChannel)` 如果输入通道已连接
    /// - `None` 如果输入通道未连接
    pub fn inputs_mut(&mut self) -> Option<&mut InputsChannel> {
        self.inputs_channel.as_mut()
    }

    /// 获取显示通道引用
    pub fn display(&self, id: u8) -> Option<&DisplayChannel> {
        self.display_channels.get(&id)
    }

    /// 获取显示通道可变引用
    pub fn display_mut(&mut self, id: u8) -> Option<&mut DisplayChannel> {
        self.display_channels.get_mut(&id)
    }

    /// 获取 USB 重定向通道引用
    pub fn usbredir(&self, id: u8) -> Option<&UsbRedirChannel> {
        self.usbredir_channels.get(&id)
    }

    /// 获取客户端状态
    pub fn state(&self) -> ClientState {
        self.state
    }

    /// 是否已连接
    pub fn is_connected(&self) -> bool {
        self.state == ClientState::Connected
    }

    /// 获取会话 ID
    pub fn session_id(&self) -> u32 {
        self.session_id
    }

    /// 获取当前鼠标模式
    pub fn mouse_mode(&self) -> u32 {
        self.mouse_mode
    }

    /// 获取可用通道列表
    pub fn available_channels(&self) -> &[(u8, u8)] {
        &self.available_channels
    }

    /// 请求切换鼠标模式
    pub async fn request_mouse_mode(&mut self, mode: u32) -> Result<()> {
        let main = self
            .main_channel
            .as_mut()
            .ok_or_else(|| ProtocolError::ConnectionFailed("主通道未连接".to_string()))?;

        let req = MsgcMainMouseModeRequest::new(mode);
        main.send_message(105, &req.to_bytes()).await?;

        debug!("请求鼠标模式: {}", mode);
        Ok(())
    }

    /// 处理主通道事件（应在后台任务中调用）
    pub async fn process_main_events(&mut self) -> Result<()> {
        let main = self
            .main_channel
            .as_mut()
            .ok_or_else(|| ProtocolError::ConnectionFailed("主通道未连接".to_string()))?;

        let (msg_type, data) = main.receive_message().await?;

        match msg_type {
            // SPICE_MSG_MAIN_MOUSE_MODE
            105 => {
                if let Some(mode) = MsgMainMouseMode::from_bytes(&data) {
                    self.mouse_mode = mode.current_mode;
                    debug!("鼠标模式更新: {}", mode.current_mode);
                }
            }
            // SPICE_MSG_PING
            4 => {
                main.send_message(3, &data).await?;
            }
            _ => {
                debug!("主通道消息: type={}, size={}", msg_type, data.len());
            }
        }

        Ok(())
    }
}

/// 线程安全的 SPICE 客户端包装器
pub struct SharedSpiceClient {
    inner: Arc<RwLock<SpiceClient>>,
}

impl SharedSpiceClient {
    pub fn new(config: SpiceConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(SpiceClient::new(config))),
        }
    }

    pub async fn connect(&self) -> Result<()> {
        let mut client = self.inner.write().await;
        client.connect().await
    }

    pub async fn disconnect(&self) -> Result<()> {
        let mut client = self.inner.write().await;
        client.disconnect().await
    }

    pub async fn is_connected(&self) -> bool {
        let client = self.inner.read().await;
        client.is_connected()
    }

    /// 获取内部引用
    pub fn inner(&self) -> Arc<RwLock<SpiceClient>> {
        self.inner.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = SpiceConfig::new("192.168.1.100", 5900)
            .with_password("secret")
            .with_tls_port(Some(5901))
            .with_client_mouse(true);

        assert_eq!(config.host, "192.168.1.100");
        assert_eq!(config.port, 5900);
        assert_eq!(config.password, Some("secret".to_string()));
        assert_eq!(config.tls_port, Some(5901));
        assert!(config.request_client_mouse);
    }

    #[test]
    fn test_client_initial_state() {
        let config = SpiceConfig::new("localhost", 5900);
        let client = SpiceClient::new(config);

        assert_eq!(client.state(), ClientState::Disconnected);
        assert!(!client.is_connected());
        assert_eq!(client.session_id(), 0);
    }
}

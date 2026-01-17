use anyhow::{anyhow, Context, Result};
use std::sync::Arc;
use tracing::{debug, info};

use atp_protocol::spice::client::{SpiceClient, SpiceConfig};
use atp_protocol::spice::inputs::MouseButton;
use atp_vdiplatform::client::VdiClient;

/// SPICE 连接器
///
/// 提供简化的接口用于连接虚拟机 SPICE 并发送输入事件
pub struct SpiceConnector {
    vdi_client: Arc<VdiClient>,
    spice_client: Option<SpiceClient>,
    domain_id: Option<String>,
}

impl SpiceConnector {
    /// 创建新的 SPICE 连接器
    pub fn new(vdi_client: Arc<VdiClient>) -> Self {
        Self {
            vdi_client,
            spice_client: None,
            domain_id: None,
        }
    }

    /// 连接到指定虚拟机的 SPICE 服务
    pub async fn connect(&mut self, domain_id: &str) -> Result<()> {
        info!("正在获取虚拟机 {} 的 SPICE 连接信息...", domain_id);

        let domain_api = self.vdi_client.domain();
        let info = domain_api
            .get_spice_connection_info(domain_id)
            .await
            .context("获取 SPICE 连接信息失败")?;

        info!(
            "SPICE 连接信息: {}:{} (TLS: {:?})",
            info.host, info.port, info.tls_port
        );

        let config = SpiceConfig::new(&info.host, info.port)
            .with_password_opt(info.password)
            .with_tls_port(info.tls_port)
            .with_auto_inputs(true)
            .with_auto_display(false); // 暂时不需要 display

        let mut client = SpiceClient::new(config);
        client
            .connect()
            .await
            .map_err(|e| anyhow!("连接 SPICE 服务器失败: {}", e))?;

        // 等待 Input 通道就绪 (可选，简单等待)
        if let Some(inputs) = client.inputs() {
            if !inputs.is_connected().await {
                debug!("等待 Inputs 通道连接...");
                // 可以在这里添加重试或等待逻辑，SpiceClient 内部已尝试自动连接
            }
        }

        self.spice_client = Some(client);
        self.domain_id = Some(domain_id.to_string());

        info!("SPICE 连接成功");
        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(_client) = self.spice_client.take() {
            // 目前 SpiceClient 没有显式的 disconnect 方法，Drop 会处理
            // 或者如果将来实现了 disconnect，可以在这里调用
            info!("SPICE 已断开连接");
        }
        self.domain_id = None;
        Ok(())
    }

    /// 获取 SPICE 客户端引用
    pub fn client(&self) -> Option<&SpiceClient> {
        self.spice_client.as_ref()
    }

    // ========================================================================
    // 输入操作封装
    // ========================================================================

    /// 确保连接有效
    fn ensure_connected(&self) -> Result<&SpiceClient> {
        self.spice_client
            .as_ref()
            .ok_or_else(|| anyhow!("SPICE 未连接"))
    }

    /// 发送键盘按键 (按下+释放)
    pub async fn press_key(&self, scancode: u32) -> Result<()> {
        let client = self.ensure_connected()?;
        if let Some(inputs) = client.inputs() {
            inputs
                .send_key_press(scancode)
                .await
                .map_err(|e| anyhow!("发送按键失败: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("Inputs 通道不可用"))
        }
    }

    /// 发送文本 (自动转换扫描码)
    pub async fn send_text(&self, text: &str) -> Result<()> {
        let client = self.ensure_connected()?;
        if let Some(inputs) = client.inputs() {
            inputs
                .send_text(text)
                .await
                .map_err(|e| anyhow!("发送文本失败: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("Inputs 通道不可用"))
        }
    }

    /// 发送鼠标点击
    pub async fn click(&self, x: u32, y: u32, button: MouseButton) -> Result<()> {
        self.move_mouse(x, y).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        self.mouse_click(button).await?;
        Ok(())
    }

    /// 仅移动鼠标
    pub async fn move_mouse(&self, x: u32, y: u32) -> Result<()> {
        let client = self.ensure_connected()?;
        if let Some(inputs) = client.inputs() {
            // 使用 absolute 坐标，假设使用第一个显示器 (id=0)
            inputs
                .send_mouse_position(x, y, 0)
                .await
                .map_err(|e| anyhow!("鼠标移动失败: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("Inputs 通道不可用"))
        }
    }

    /// 仅点击鼠标 (在当前位置)
    pub async fn mouse_click(&self, button: MouseButton) -> Result<()> {
        let client = self.ensure_connected()?;
        if let Some(inputs) = client.inputs() {
            inputs
                .send_mouse_click(button)
                .await
                .map_err(|e| anyhow!("鼠标点击失败: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("Inputs 通道不可用"))
        }
    }

    /// 鼠标双击 (在当前位置)
    pub async fn mouse_double_click(&self, button: MouseButton) -> Result<()> {
        let client = self.ensure_connected()?;
        if let Some(inputs) = client.inputs() {
            inputs
                .send_mouse_double_click(button)
                .await
                .map_err(|e| anyhow!("鼠标双击失败: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("Inputs 通道不可用"))
        }
    }
}

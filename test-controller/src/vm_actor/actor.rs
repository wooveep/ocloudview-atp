use anyhow::{Context, Result};
use async_channel::{Receiver, Sender};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::qmp::QmpClient;
use crate::keymapping::KeyMapper;

use super::message::{VmCommand, VmEvent, VmMessage};

/// VM Actor - 每个虚拟机对应一个 Actor
pub struct VmActor {
    /// 虚拟机名称
    vm_name: String,

    /// QMP 客户端
    qmp_client: QmpClient,

    /// 键值映射器
    key_mapper: KeyMapper,

    /// 接收命令的通道
    command_rx: Receiver<VmCommand>,

    /// 发送事件的通道
    event_tx: Sender<VmEvent>,

    /// Guest Agent WebSocket 连接状态
    agent_connected: bool,
}

impl VmActor {
    /// 创建新的 VM Actor
    pub fn new(
        vm_name: String,
        qmp_client: QmpClient,
        command_rx: Receiver<VmCommand>,
        event_tx: Sender<VmEvent>,
    ) -> Self {
        Self {
            vm_name,
            qmp_client,
            key_mapper: KeyMapper::default(),
            command_rx,
            event_tx,
            agent_connected: false,
        }
    }

    /// 启动 Actor 主循环
    pub async fn run(mut self) -> Result<()> {
        info!("VM Actor 启动: {}", self.vm_name);

        // 发送启动事件
        self.send_event(VmEvent::Started {
            vm_name: self.vm_name.clone(),
        })
        .await?;

        // 主循环：处理命令
        loop {
            match self.command_rx.recv().await {
                Ok(command) => {
                    if let Err(e) = self.handle_command(command).await {
                        error!("处理命令失败: {}", e);
                        self.send_event(VmEvent::Error {
                            message: e.to_string(),
                        })
                        .await?;
                    }
                }
                Err(_) => {
                    info!("命令通道已关闭，退出 Actor: {}", self.vm_name);
                    break;
                }
            }
        }

        // 发送停止事件
        self.send_event(VmEvent::Stopped).await?;

        Ok(())
    }

    /// 处理命令
    async fn handle_command(&mut self, command: VmCommand) -> Result<()> {
        debug!("收到命令: {:?}", command);

        match command {
            VmCommand::SendKeys { keys } => {
                self.send_keys(&keys).await?;
            }

            VmCommand::SendText { text } => {
                self.send_text(&text).await?;
            }

            VmCommand::QueryStatus => {
                let status = self.qmp_client.query_status().await?;
                debug!("虚拟机状态: {:?}", status);
            }

            VmCommand::WaitForAgent { timeout_secs } => {
                self.wait_for_agent(timeout_secs).await?;
            }

            VmCommand::RunTestCase { test_id } => {
                self.run_test_case(&test_id).await?;
            }

            VmCommand::Shutdown => {
                info!("收到关闭命令，退出 Actor: {}", self.vm_name);
                return Err(anyhow::anyhow!("Actor shutdown requested"));
            }
        }

        Ok(())
    }

    /// 发送按键序列
    async fn send_keys(&mut self, keys: &[String]) -> Result<()> {
        info!("发送按键序列: {:?}", keys);

        for key in keys {
            self.qmp_client
                .send_key(key)
                .await
                .context("发送按键失败")?;

            // 按键之间的间隔
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(())
    }

    /// 发送文本
    async fn send_text(&mut self, text: &str) -> Result<()> {
        info!("发送文本: {}", text);

        // 将文本映射为按键序列
        let keys = self
            .key_mapper
            .map_string(text)
            .context("文本映射失败")?;

        self.send_keys(&keys).await?;

        Ok(())
    }

    /// 等待 Guest Agent 连接
    async fn wait_for_agent(&mut self, timeout_secs: u64) -> Result<()> {
        info!("等待 Guest Agent 连接 (超时: {}秒)", timeout_secs);

        // TODO: 实现 WebSocket 服务器，接收 Guest Agent 连接
        // 这里仅作为占位符

        match timeout(Duration::from_secs(timeout_secs), self.wait_agent_impl()).await {
            Ok(_) => {
                self.agent_connected = true;
                self.send_event(VmEvent::AgentConnected).await?;
                Ok(())
            }
            Err(_) => {
                Err(anyhow::anyhow!("等待 Guest Agent 连接超时"))
            }
        }
    }

    /// 等待 Agent 实现（占位符）
    async fn wait_agent_impl(&self) {
        // TODO: 实际实现
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    /// 运行测试用例
    async fn run_test_case(&mut self, test_id: &str) -> Result<()> {
        info!("运行测试用例: {}", test_id);

        // TODO: 实现测试用例逻辑
        // 1. 发送输入
        // 2. 等待 Guest Agent 回传
        // 3. 验证结果

        // 临时实现：发送一个简单的测试序列
        self.send_text("Hello World").await?;

        // 模拟测试完成
        self.send_event(VmEvent::TestCaseCompleted {
            test_id: test_id.to_string(),
            passed: true,
        })
        .await?;

        Ok(())
    }

    /// 发送事件
    async fn send_event(&self, event: VmEvent) -> Result<()> {
        self.event_tx
            .send(event)
            .await
            .context("发送事件失败")?;
        Ok(())
    }
}

/// VM Actor 的句柄，用于与 Actor 通信
#[derive(Clone)]
pub struct VmActorHandle {
    pub vm_name: String,
    pub command_tx: Sender<VmCommand>,
    pub event_rx: Receiver<VmEvent>,
}

impl VmActorHandle {
    /// 创建新的 Actor 句柄
    pub fn new(vm_name: String) -> (Self, Receiver<VmCommand>, Sender<VmEvent>) {
        let (command_tx, command_rx) = async_channel::unbounded();
        let (event_tx, event_rx) = async_channel::unbounded();

        let handle = Self {
            vm_name,
            command_tx,
            event_rx,
        };

        (handle, command_rx, event_tx)
    }

    /// 发送命令到 Actor
    pub async fn send_command(&self, command: VmCommand) -> Result<()> {
        self.command_tx
            .send(command)
            .await
            .context("发送命令失败")?;
        Ok(())
    }

    /// 接收事件（非阻塞）
    pub async fn recv_event(&self) -> Result<VmEvent> {
        self.event_rx.recv().await.context("接收事件失败")
    }
}

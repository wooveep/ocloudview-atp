use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::libvirt::{DomainInfo, LibvirtManager};
use crate::qmp::QmpClient;
use crate::vm_actor::{VmActor, VmActorHandle, VmCommand, VmEvent};

/// 测试编排器 - 管理多个 VM Actor
pub struct TestOrchestrator {
    libvirt: LibvirtManager,
    actors: HashMap<String, VmActorHandle>,
    tasks: Vec<JoinHandle<()>>,
}

impl TestOrchestrator {
    /// 创建新的测试编排器
    pub fn new() -> Result<Self> {
        let libvirt = LibvirtManager::connect()?;

        Ok(Self {
            libvirt,
            actors: HashMap::new(),
            tasks: Vec::new(),
        })
    }

    /// 发现并初始化所有活动的虚拟机
    pub async fn discover_vms(&mut self) -> Result<Vec<DomainInfo>> {
        info!("发现活动的虚拟机...");

        let domains = self.libvirt.list_active_domains()?;
        let mut vm_infos = Vec::new();

        for domain in domains {
            let info = self.libvirt.get_domain_info(&domain)?;
            info!("发现虚拟机: {} (UUID: {})", info.name, info.uuid);
            vm_infos.push(info);
        }

        Ok(vm_infos)
    }

    /// 为指定的虚拟机启动 Actor
    pub async fn spawn_actor(&mut self, vm_name: &str) -> Result<()> {
        info!("为虚拟机 {} 启动 Actor", vm_name);

        // 查找虚拟机
        let domain = self.libvirt.lookup_domain_by_name(vm_name)?;

        // 获取 QMP Socket 路径
        let qmp_socket = self.libvirt.get_qmp_socket_path(&domain)?;
        info!("QMP Socket: {:?}", qmp_socket);

        // 连接 QMP
        let qmp_client = QmpClient::connect(qmp_socket.to_str().unwrap()).await?;

        // 创建 Actor 句柄和通道
        let (handle, command_rx, event_tx) = VmActorHandle::new(vm_name.to_string());

        // 创建并启动 Actor
        let actor = VmActor::new(vm_name.to_string(), qmp_client, command_rx, event_tx);

        let task = tokio::spawn(async move {
            if let Err(e) = actor.run().await {
                warn!("VM Actor 运行错误: {}", e);
            }
        });

        // 保存句柄和任务
        self.actors.insert(vm_name.to_string(), handle);
        self.tasks.push(task);

        Ok(())
    }

    /// 向指定虚拟机发送命令
    pub async fn send_command(&self, vm_name: &str, command: VmCommand) -> Result<()> {
        let handle = self
            .actors
            .get(vm_name)
            .context(format!("虚拟机 {} 的 Actor 未找到", vm_name))?;

        handle.send_command(command).await?;
        Ok(())
    }

    /// 向所有虚拟机发送命令
    pub async fn broadcast_command(&self, command: VmCommand) -> Result<()> {
        for (vm_name, handle) in &self.actors {
            info!("向 {} 发送命令: {:?}", vm_name, command);
            handle.send_command(command.clone()).await?;
        }

        Ok(())
    }

    /// 等待所有 Actor 完成
    pub async fn wait_all(&mut self) -> Result<()> {
        info!("等待所有 VM Actor 完成...");

        while let Some(task) = self.tasks.pop() {
            task.await?;
        }

        info!("所有 VM Actor 已完成");
        Ok(())
    }

    /// 关闭所有 Actor
    pub async fn shutdown_all(&self) -> Result<()> {
        info!("关闭所有 VM Actor...");

        self.broadcast_command(VmCommand::Shutdown).await?;

        Ok(())
    }

    /// 运行批量测试
    pub async fn run_batch_test(&self, test_id: &str) -> Result<HashMap<String, bool>> {
        info!("运行批量测试: {}", test_id);

        let mut results = HashMap::new();

        // 向所有虚拟机发送测试命令
        for (vm_name, handle) in &self.actors {
            info!("在 {} 上运行测试 {}", vm_name, test_id);

            handle
                .send_command(VmCommand::RunTestCase {
                    test_id: test_id.to_string(),
                })
                .await?;

            // 等待测试完成事件
            match handle.recv_event().await? {
                VmEvent::TestCaseCompleted { test_id: _, passed } => {
                    results.insert(vm_name.clone(), passed);
                }
                VmEvent::Error { message } => {
                    warn!("测试失败: {}", message);
                    results.insert(vm_name.clone(), false);
                }
                _ => {}
            }
        }

        Ok(results)
    }

    /// 获取活动的 Actor 数量
    pub fn actor_count(&self) -> usize {
        self.actors.len()
    }
}

impl Drop for TestOrchestrator {
    fn drop(&mut self) {
        // 在 Drop 时尝试关闭所有 Actor
        // 注意：这是同步的 Drop，无法使用 async
        // 实际应用中应该显式调用 shutdown_all
    }
}

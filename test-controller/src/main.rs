mod qmp;
mod libvirt;
mod keymapping;
mod vm_actor;
mod orchestrator;

use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("OCloudView ATP Test Controller 启动中...");

    // TODO: 实现主控制逻辑
    // 1. 连接到 Libvirt
    // 2. 发现虚拟机
    // 3. 启动 VM Actors
    // 4. 运行测试编排器

    info!("Test Controller 已启动");

    Ok(())
}

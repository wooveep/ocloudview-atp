//! 主机管理命令

use anyhow::Result;

pub async fn handle(action: crate::HostAction) -> Result<()> {
    // TODO: 实现主机管理逻辑
    println!("主机管理命令: {:?}", action);
    Ok(())
}

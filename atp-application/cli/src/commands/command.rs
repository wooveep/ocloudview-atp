//! Command 命令处理

use anyhow::Result;

pub async fn handle(action: crate::CommandAction) -> Result<()> {
    // TODO: 实现 command 逻辑
    println!("Command 命令: {:?}", action);
    Ok(())
}

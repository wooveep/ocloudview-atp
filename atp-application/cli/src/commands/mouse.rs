//! Mouse 命令处理

use anyhow::Result;

pub async fn handle(action: crate::MouseAction) -> Result<()> {
    // TODO: 实现 mouse 逻辑
    println!("Mouse 命令: {:?}", action);
    Ok(())
}

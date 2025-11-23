//! Keyboard 命令处理

use anyhow::Result;

pub async fn handle(action: crate::KeyboardAction) -> Result<()> {
    // TODO: 实现 keyboard 逻辑
    println!("Keyboard 命令: {:?}", action);
    Ok(())
}

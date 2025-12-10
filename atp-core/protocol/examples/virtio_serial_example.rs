//! VirtioSerial 协议使用示例
//!
//! 演示如何使用 VirtioSerial 协议与虚拟机内自定义 agent 通信

use std::path::PathBuf;
use atp_protocol::{
    VirtioSerialBuilder,
    VirtioChannel,
    Protocol,
    RawProtocolHandler,
    JsonProtocolHandler,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("VirtioSerial 协议使用示例\n");
    println!("==========================================\n");

    // 示例 1: 使用原始协议处理器
    println!("示例 1: 原始协议处理器");
    println!("------------------------------------------");
    example_raw_protocol().await?;
    println!();

    // 示例 2: 使用 JSON 协议处理器
    println!("示例 2: JSON 协议处理器");
    println!("------------------------------------------");
    example_json_protocol().await?;
    println!();

    // 示例 3: 直接使用通道发送数据
    println!("示例 3: 直接通道操作");
    println!("------------------------------------------");
    example_direct_channel().await?;

    Ok(())
}

/// 示例 1: 使用原始协议处理器
async fn example_raw_protocol() -> anyhow::Result<()> {
    println!("创建 VirtioSerial 协议（原始模式）...");

    // 方式1: 使用构建器
    let builder = VirtioSerialBuilder::new("com.vmagent.sock")
        .with_raw_handler()
        .with_socket_path(PathBuf::from("/var/lib/libvirt/qemu/channel/target/test-uuid"));

    let mut protocol = builder.build();

    println!("协议类型: {:?}", protocol.protocol_type());
    println!();

    // 注意：实际使用时需要连接到真实的虚拟机
    // protocol.connect(&domain).await?;

    println!("使用原始协议发送数据:");
    println!("  - 直接发送原始字节");
    println!("  - 不做任何编码或封装");
    println!("  - 适合自定义二进制协议");

    // 模拟发送数据
    println!("\n发送示例:");
    println!("  protocol.send(b\"custom command\\n\").await?;");

    Ok(())
}

/// 示例 2: 使用 JSON 协议处理器
async fn example_json_protocol() -> anyhow::Result<()> {
    println!("创建 VirtioSerial 协议（JSON 模式）...");

    // 方式2: 使用 JSON 处理器
    let builder = VirtioSerialBuilder::new("com.vmagent.sock")
        .with_json_handler()
        .with_socket_path(PathBuf::from("/var/lib/libvirt/qemu/channel/target/test-uuid"));

    let mut protocol = builder.build();

    println!("协议类型: {:?}", protocol.protocol_type());
    println!();

    println!("使用 JSON 协议发送数据:");
    println!("  - 自动包装为 JSON 格式");
    println!("  - 默认字段: {{\"data\": \"your message\"}}");
    println!("  - 自动添加换行符作为分隔符");

    // 模拟 JSON 编码
    let handler = JsonProtocolHandler::default();
    let encoded = handler.encode_request(b"test message").await?;
    println!("\n编码结果:");
    println!("  {}", String::from_utf8_lossy(&encoded));

    println!("\n自定义 JSON 字段:");
    let builder = VirtioSerialBuilder::new("com.vmagent.sock")
        .with_custom_json_handler("command", "response");

    println!("  请求字段: command");
    println!("  响应字段: response");

    Ok(())
}

/// 示例 3: 直接使用通道
async fn example_direct_channel() -> anyhow::Result<()> {
    println!("直接使用 VirtioChannel...");

    // 创建通道
    let mut channel = VirtioChannel::new(
        "com.vmagent.sock",
        PathBuf::from("/var/lib/libvirt/qemu/channel/target/test-uuid"),
    );

    println!("通道信息: {:?}", channel.info());
    println!();

    // 注意：实际使用时需要连接
    // channel.connect().await?;

    println!("通道操作:");
    println!("  - send_raw(data)      : 发送原始字节");
    println!("  - receive_raw(buffer) : 接收原始字节");
    println!("  - send_string(text)   : 发送字符串");
    println!("  - receive_line()      : 接收一行文本");

    println!("\n使用场景:");
    println!("  1. 发送自定义命令到 Guest Agent");
    println!("  2. 接收 Agent 状态报告");
    println!("  3. 实现自定义通信协议");

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_example() {
        // 示例测试
    }
}

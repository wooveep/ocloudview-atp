//! SPICE 协议使用示例
//!
//! 演示如何使用 SPICE 协议连接到虚拟机并模拟用户操作
//!
//! ## 使用方法
//!
//! ```bash
//! cargo run --example spice_demo
//! ```

use atp_protocol::spice::{MouseButton, MouseMode, SpiceClient, SpiceConfig, SpiceDiscovery};
use tokio;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("SPICE 协议演示程序");

    // 示例 1: 手动配置连接
    demo_manual_connection().await?;

    // 示例 2: 通过 libvirt 发现虚拟机
    // demo_libvirt_discovery().await?;

    // 示例 3: 模拟用户操作
    // demo_user_interaction().await?;

    Ok(())
}

/// 示例 1: 手动配置 SPICE 连接
async fn demo_manual_connection() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== 示例 1: 手动配置 SPICE 连接 ===");

    // 创建 SPICE 配置
    let config = SpiceConfig::new("192.168.1.100", 5900)
        .with_password("your-spice-password") // 如果有密码
        .with_client_mouse(true); // 使用客户端鼠标模式

    // 创建客户端
    let mut client = SpiceClient::new(config);

    // 连接到服务器
    match client.connect().await {
        Ok(_) => {
            info!("✓ 连接到 SPICE 服务器成功");
            info!("  会话 ID: {}", client.session_id());
            info!("  鼠标模式: {}", client.mouse_mode());

            // 断开连接
            client.disconnect().await?;
            info!("✓ 已断开连接");
        }
        Err(e) => {
            error!("✗ 连接失败: {}", e);
        }
    }

    Ok(())
}

/// 示例 2: 通过 libvirt 发现虚拟机
#[allow(dead_code)]
async fn demo_libvirt_discovery() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== 示例 2: 通过 libvirt 发现虚拟机 ===");

    // 连接到 libvirt
    let conn = virt::connect::Connect::open(Some("qemu:///system"))?;

    // 创建 SPICE 发现器
    let discovery = SpiceDiscovery::new().with_default_host("192.168.1.100");

    // 发现所有带 SPICE 的虚拟机
    let vms = discovery.discover_all(&conn).await?;

    info!("发现 {} 个带 SPICE 的虚拟机:", vms.len());
    for vm in &vms {
        info!("  - {} ({}:{})", vm.name, vm.host, vm.port);
        if let Some(tls_port) = vm.tls_port {
            info!("    TLS 端口: {}", tls_port);
        }
    }

    // 连接到第一个虚拟机
    if let Some(vm) = vms.first() {
        info!("连接到虚拟机: {}", vm.name);

        let config = SpiceConfig::new(&vm.host, vm.port).with_password_opt(vm.password.clone());

        let mut client = SpiceClient::new(config);
        client.connect().await?;

        info!("✓ 连接成功");

        // 执行一些操作...

        client.disconnect().await?;
    }

    Ok(())
}

/// 示例 3: 模拟用户操作
#[allow(dead_code)]
async fn demo_user_interaction() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== 示例 3: 模拟用户操作 ===");

    // 配置和连接
    let config = SpiceConfig::new("192.168.1.100", 5900)
        .with_password("password")
        .with_client_mouse(true)
        .with_auto_inputs(true); // 自动连接输入通道

    let mut client = SpiceClient::new(config);
    client.connect().await?;

    info!("✓ 已连接到虚拟机");

    // 获取输入通道
    let inputs = client.inputs().ok_or("输入通道未连接")?;

    // 1. 模拟键盘输入
    info!("模拟键盘输入: 输入文本 'Hello World'");
    inputs.send_text("Hello World").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // 2. 模拟按键组合 (Ctrl+C)
    info!("模拟按键组合: Ctrl+C");
    use atp_protocol::spice::inputs::scancode;
    inputs.send_key_down(scancode::LEFT_CTRL).await?;
    inputs.send_key_press(0x2E).await?; // C 键
    inputs.send_key_up(scancode::LEFT_CTRL).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 3. 模拟鼠标移动和点击
    info!("模拟鼠标操作: 移动到 (500, 300) 并点击");
    inputs.send_mouse_position(500, 300, 0).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    inputs.send_mouse_click(MouseButton::Left).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 4. 模拟双击
    info!("模拟双击");
    inputs.send_mouse_double_click(MouseButton::Left).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 5. 模拟鼠标滚轮
    info!("模拟鼠标滚轮: 向上滚动 3 次");
    inputs.send_mouse_scroll(true, 3).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 6. 模拟右键菜单
    info!("模拟右键点击");
    inputs.send_mouse_click(MouseButton::Right).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // 7. 模拟按下 ESC 键关闭菜单
    info!("按 ESC 键");
    inputs.send_key_press(scancode::ESCAPE).await?;

    info!("✓ 演示完成");

    // 断开连接
    client.disconnect().await?;

    Ok(())
}

/// 示例 4: 并发操作多个虚拟机
#[allow(dead_code)]
async fn demo_concurrent_vms() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== 示例 4: 并发操作多个虚拟机 ===");

    let vms = vec![
        ("192.168.1.101", 5900, "vm1"),
        ("192.168.1.102", 5900, "vm2"),
        ("192.168.1.103", 5900, "vm3"),
    ];

    let mut tasks = Vec::new();

    for (host, port, name) in vms {
        let task = tokio::spawn(async move {
            let config = SpiceConfig::new(host, port).with_password("password");

            let mut client = SpiceClient::new(config);

            match client.connect().await {
                Ok(_) => {
                    info!("[{}] 连接成功", name);

                    // 执行一些操作
                    if let Some(inputs) = client.inputs() {
                        inputs.send_text("Test from ").await?;
                        inputs.send_text(name).await?;
                    }

                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    client.disconnect().await?;
                    info!("[{}] 断开连接", name);

                    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                }
                Err(e) => {
                    error!("[{}] 连接失败: {}", name, e);
                    Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            }
        });

        tasks.push(task);
    }

    // 等待所有任务完成
    for task in tasks {
        let _ = task.await;
    }

    info!("✓ 所有虚拟机操作完成");

    Ok(())
}

/// 示例 5: 负载测试 - 持续模拟用户操作
#[allow(dead_code)]
async fn demo_load_testing() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== 示例 5: 负载测试 ===");

    let config = SpiceConfig::new("192.168.1.100", 5900)
        .with_password("password")
        .with_client_mouse(true);

    let mut client = SpiceClient::new(config);
    client.connect().await?;

    info!("✓ 已连接，开始负载测试 (持续 60 秒)");

    let start_time = std::time::Instant::now();
    let duration = std::time::Duration::from_secs(60);
    let mut operation_count = 0u64;

    while start_time.elapsed() < duration {
        let inputs = client.inputs().ok_or("输入通道未连接")?;

        // 模拟鼠标移动
        let x = rand::random::<u32>() % 1920;
        let y = rand::random::<u32>() % 1080;
        inputs.send_mouse_position(x, y, 0).await?;

        // 随机点击
        if rand::random::<u32>() % 10 == 0 {
            inputs.send_mouse_click(MouseButton::Left).await?;
        }

        // 随机按键
        if rand::random::<u32>() % 20 == 0 {
            inputs.send_text("test ").await?;
        }

        operation_count += 1;

        // 控制操作频率 (100ms 间隔)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    let elapsed = start_time.elapsed();
    let ops_per_sec = operation_count as f64 / elapsed.as_secs_f64();

    info!("✓ 负载测试完成:");
    info!("  总操作数: {}", operation_count);
    info!("  测试时长: {:.1} 秒", elapsed.as_secs_f64());
    info!("  平均速率: {:.1} ops/s", ops_per_sec);

    client.disconnect().await?;

    Ok(())
}

/// QGA 使用示例
///
/// 本模块展示如何使用 QEMU Guest Agent 客户端执行各种操作

use anyhow::Result;
use virt::domain::Domain;

use super::QgaClient;
use crate::qga::protocol::GuestExecCommand;

/// 示例 1: 测试 QGA 连通性
pub fn example_ping(domain: &Domain) -> Result<()> {
    let qga = QgaClient::new(domain);

    // 测试连通性
    qga.ping()?;

    println!("QGA 连通性测试成功");
    Ok(())
}

/// 示例 2: 获取 Guest 操作系统信息
pub fn example_get_osinfo(domain: &Domain) -> Result<()> {
    let qga = QgaClient::new(domain);

    // 获取操作系统信息
    let os_info = qga.get_osinfo()?;

    println!("操作系统信息:");
    println!("  名称: {:?}", os_info.pretty_name);
    println!("  版本: {:?}", os_info.version);
    println!("  内核: {:?}", os_info.kernel_release);
    println!("  架构: {:?}", os_info.machine);

    Ok(())
}

/// 示例 3: 执行简单的 Shell 命令
pub fn example_exec_shell(domain: &Domain) -> Result<()> {
    let qga = QgaClient::new(domain);

    // 执行 Shell 命令
    let result = qga.exec_shell("echo 'Hello from QGA'")?;

    println!("命令执行结果:");
    println!("  退出码: {:?}", result.exit_code);

    if let Some(stdout) = result.decode_stdout() {
        println!("  标准输出:\n{}", stdout);
    }

    if let Some(stderr) = result.decode_stderr() {
        println!("  标准错误:\n{}", stderr);
    }

    Ok(())
}

/// 示例 4: 执行复杂命令（带参数）
pub fn example_exec_with_args(domain: &Domain) -> Result<()> {
    let qga = QgaClient::new(domain);

    // Linux 示例：列出当前目录
    let cmd = GuestExecCommand::simple(
        "/bin/ls",
        vec!["-la".to_string(), "/tmp".to_string()],
    );

    let pid = qga.exec(cmd)?;
    println!("进程已启动，PID: {}", pid);

    // 等待完成
    std::thread::sleep(std::time::Duration::from_secs(1));

    let status = qga.exec_status(pid)?;
    if status.exited {
        println!("命令已完成");
        if let Some(output) = status.decode_stdout() {
            println!("输出:\n{}", output);
        }
    }

    Ok(())
}

/// 示例 5: Windows 命令执行
pub fn example_exec_windows(domain: &Domain) -> Result<()> {
    let qga = QgaClient::new(domain);

    // Windows 示例：查看系统信息
    let result = qga.exec_shell("systeminfo | findstr /B /C:\"OS Name\" /C:\"OS Version\"")?;

    println!("Windows 系统信息:");
    if let Some(output) = result.decode_stdout() {
        println!("{}", output);
    }

    Ok(())
}

/// 示例 6: 文件读取
pub fn example_read_file(domain: &Domain, file_path: &str) -> Result<String> {
    let qga = QgaClient::new(domain);

    // 读取文件内容
    let content = qga.read_file(file_path)?;

    println!("文件内容 ({}):", file_path);
    println!("{}", content);

    Ok(content)
}

/// 示例 7: 文件写入
pub fn example_write_file(domain: &Domain, file_path: &str, content: &str) -> Result<()> {
    let qga = QgaClient::new(domain);

    // 写入文件
    qga.write_file(file_path, content)?;

    println!("文件已写入: {}", file_path);

    Ok(())
}

/// 示例 8: 批量执行命令
pub fn example_batch_commands(domain: &Domain) -> Result<()> {
    let qga = QgaClient::new(domain);

    let commands = vec![
        "whoami",
        "pwd",
        "date",
        "uname -a",
    ];

    for cmd in commands {
        println!("\n执行命令: {}", cmd);
        match qga.exec_shell(cmd) {
            Ok(result) => {
                if let Some(output) = result.decode_stdout() {
                    println!("输出: {}", output.trim());
                }
            }
            Err(e) => {
                eprintln!("命令执行失败: {}", e);
            }
        }
    }

    Ok(())
}

/// 示例 9: 收集系统诊断信息
pub fn example_collect_diagnostics(domain: &Domain) -> Result<String> {
    let qga = QgaClient::new(domain);

    let mut diagnostics = String::new();

    // 操作系统信息
    if let Ok(os_info) = qga.get_osinfo() {
        diagnostics.push_str(&format!("OS: {:?}\n", os_info.pretty_name));
        diagnostics.push_str(&format!("Kernel: {:?}\n", os_info.kernel_release));
    }

    // 系统负载（Linux）
    if let Ok(result) = qga.exec_shell("uptime") {
        if let Some(output) = result.decode_stdout() {
            diagnostics.push_str(&format!("Uptime: {}\n", output.trim()));
        }
    }

    // 内存使用（Linux）
    if let Ok(result) = qga.exec_shell("free -h") {
        if let Some(output) = result.decode_stdout() {
            diagnostics.push_str(&format!("\nMemory:\n{}\n", output));
        }
    }

    // 磁盘使用（Linux）
    if let Ok(result) = qga.exec_shell("df -h") {
        if let Some(output) = result.decode_stdout() {
            diagnostics.push_str(&format!("\nDisk Usage:\n{}\n", output));
        }
    }

    println!("{}", diagnostics);
    Ok(diagnostics)
}

/// 示例 10: 综合测试流程
pub fn example_full_workflow(domain: &Domain) -> Result<()> {
    let qga = QgaClient::new(domain);

    println!("=== QGA 综合测试 ===\n");

    // 1. 测试连通性
    println!("1. 测试 QGA 连通性...");
    qga.ping()?;
    println!("   ✓ QGA 可用\n");

    // 2. 获取系统信息
    println!("2. 获取系统信息...");
    let os_info = qga.get_osinfo()?;
    println!("   操作系统: {:?}", os_info.pretty_name);
    println!("   架构: {:?}\n", os_info.machine);

    // 3. 执行测试命令
    println!("3. 执行测试命令...");
    let test_cmd = if os_info.id.as_deref() == Some("windows") {
        "echo %COMPUTERNAME%"
    } else {
        "hostname"
    };

    let result = qga.exec_shell(test_cmd)?;
    if let Some(output) = result.decode_stdout() {
        println!("   主机名: {}\n", output.trim());
    }

    // 4. 文件操作测试
    println!("4. 测试文件操作...");
    let test_file = if os_info.id.as_deref() == Some("windows") {
        "C:\\temp\\qga_test.txt"
    } else {
        "/tmp/qga_test.txt"
    };

    let test_content = "QGA 测试文件\n测试时间：2024-01-01\n";

    qga.write_file(test_file, test_content)?;
    println!("   ✓ 文件已写入: {}", test_file);

    let read_content = qga.read_file(test_file)?;
    println!("   ✓ 文件已读取，大小: {} 字节\n", read_content.len());

    println!("=== 测试完成 ===");

    Ok(())
}

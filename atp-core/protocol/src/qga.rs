//! QGA (QEMU Guest Agent) 协议实现
//!
//! QGA 是 QEMU 的客户机代理，用于在虚拟机内部执行命令和操作。
//! 通过 libvirt 的 qemu_agent_command API 进行通信。

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};
use virt::domain::Domain;

use crate::{Protocol, ProtocolBuilder, ProtocolError, ProtocolType, Result};

// ============================================================================
// QGA 协议数据结构
// ============================================================================

/// QGA 命令的通用结构
#[derive(Debug, Serialize)]
pub struct QgaCommand<T> {
    pub execute: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<T>,
}

/// QGA 响应的通用结构
#[derive(Debug, Deserialize)]
pub struct QgaResponse<T> {
    #[serde(rename = "return")]
    pub ret: Option<T>,
    pub error: Option<QgaErrorInfo>,
}

/// QGA 错误信息
#[derive(Debug, Deserialize)]
pub struct QgaErrorInfo {
    #[serde(rename = "class")]
    pub error_class: String,
    pub desc: String,
}

/// guest-exec 命令参数
#[derive(Debug, Serialize)]
pub struct GuestExecCommand {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arg: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "input-data")]
    pub input_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "capture-output")]
    pub capture_output: Option<bool>,
}

/// guest-exec 返回结果
#[derive(Debug, Deserialize)]
pub struct GuestExecResult {
    pub pid: i64,
}

/// guest-exec-status 请求参数
#[derive(Debug, Serialize)]
pub struct GuestExecStatusRequest {
    pub pid: i64,
}

/// guest-exec-status 返回结果
#[derive(Debug, Deserialize)]
pub struct GuestExecStatus {
    pub exited: bool,
    #[serde(rename = "exitcode")]
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    #[serde(rename = "out-data")]
    pub out_data: Option<String>,
    #[serde(rename = "err-data")]
    pub err_data: Option<String>,
    #[serde(rename = "out-truncated")]
    pub out_truncated: Option<bool>,
    #[serde(rename = "err-truncated")]
    pub err_truncated: Option<bool>,
}

impl GuestExecCommand {
    pub fn simple(path: &str, args: Vec<String>) -> Self {
        Self {
            path: path.to_string(),
            arg: Some(args),
            env: None,
            input_data: None,
            capture_output: Some(true),
        }
    }
}

impl GuestExecStatus {
    pub fn decode_stdout(&self) -> Option<String> {
        use base64::{engine::general_purpose, Engine as _};
        self.out_data.as_ref().and_then(|data| {
            general_purpose::STANDARD
                .decode(data)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        })
    }

    pub fn decode_stderr(&self) -> Option<String> {
        use base64::{engine::general_purpose, Engine as _};
        self.err_data.as_ref().and_then(|data| {
            general_purpose::STANDARD
                .decode(data)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        })
    }
}

/// Guest 操作系统信息 (guest-get-osinfo 返回)
#[derive(Debug, Deserialize)]
pub struct GuestOsInfo {
    /// 内核版本
    #[serde(default, rename = "kernel-release")]
    pub kernel_release: Option<String>,
    /// 内核版本
    #[serde(default, rename = "kernel-version")]
    pub kernel_version: Option<String>,
    /// 机器架构
    #[serde(default)]
    pub machine: Option<String>,
    /// 操作系统 ID (如 "linux", "windows")
    #[serde(default)]
    pub id: Option<String>,
    /// 操作系统名称
    #[serde(default)]
    pub name: Option<String>,
    /// 操作系统版本 ID
    #[serde(default, rename = "version-id")]
    pub version_id: Option<String>,
    /// 操作系统版本
    #[serde(default)]
    pub version: Option<String>,
    /// 操作系统 pretty 名称
    #[serde(default, rename = "pretty-name")]
    pub pretty_name: Option<String>,
}

impl GuestOsInfo {
    /// 判断是否为 Linux 系统
    pub fn is_linux(&self) -> bool {
        self.id
            .as_ref()
            .map(|id| {
                let id_lower = id.to_lowercase();
                id_lower.contains("linux")
                    || id_lower == "ubuntu"
                    || id_lower == "debian"
                    || id_lower == "centos"
                    || id_lower == "fedora"
                    || id_lower == "rhel"
                    || id_lower == "arch"
                    || id_lower == "opensuse"
            })
            .unwrap_or(false)
    }

    /// 判断是否为 Windows 系统
    pub fn is_windows(&self) -> bool {
        self.id
            .as_ref()
            .map(|id| id.to_lowercase().contains("windows") || id.to_lowercase() == "mswindows")
            .unwrap_or(false)
    }

    /// 获取操作系统类型字符串 ("linux" 或 "windows")
    pub fn os_type(&self) -> &str {
        if self.is_windows() {
            "windows"
        } else {
            "linux" // 默认假设为 Linux
        }
    }
}

// ============================================================================
// QGA 协议实现
// ============================================================================

/// QGA 协议实现
pub struct QgaProtocol {
    /// Domain 引用（用Arc<Mutex>包装）
    domain: Option<Arc<Mutex<Domain>>>,
    /// 超时时间（秒）
    timeout: i32,
    /// 连接状态
    connected: bool,
}

impl QgaProtocol {
    pub fn new() -> Self {
        Self {
            domain: None,
            timeout: 30,
            connected: false,
        }
    }

    pub fn with_timeout(mut self, timeout: i32) -> Self {
        self.timeout = timeout;
        self
    }

    /// 执行 QGA 命令的通用方法
    pub async fn execute_command<T, R>(&self, command: &str, args: Option<T>) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        if !self.connected {
            return Err(ProtocolError::ConnectionFailed("QGA 未连接".to_string()));
        }

        let domain_guard = self
            .domain
            .as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("Domain 不可用".to_string()))?;

        let cmd = QgaCommand {
            execute: command.to_string(),
            arguments: args,
        };

        let cmd_json = serde_json::to_string(&cmd)
            .map_err(|e| ProtocolError::SendFailed(format!("序列化命令失败: {}", e)))?;

        debug!("发送 QGA 命令: {}", cmd_json);

        // 通过 Libvirt 发送命令（在阻塞任务中执行）
        let domain_guard = domain_guard.lock().await;
        let domain_clone = domain_guard.clone();
        let timeout = self.timeout;
        drop(domain_guard);

        let response_json = tokio::task::spawn_blocking(move || {
            domain_clone.qemu_agent_command(&cmd_json, timeout, 0)
        })
        .await
        .map_err(|e| ProtocolError::CommandFailed(format!("任务执行失败: {}", e)))?
        .map_err(|e| ProtocolError::CommandFailed(format!("QGA 命令失败: {}", e)))?;

        debug!("收到 QGA 响应: {}", response_json);

        // 解析响应
        let response: QgaResponse<R> = serde_json::from_str(&response_json)
            .map_err(|e| ProtocolError::ParseError(format!("解析响应失败: {}", e)))?;

        // 检查错误
        if let Some(error) = response.error {
            return Err(ProtocolError::CommandFailed(format!(
                "{}: {}",
                error.error_class, error.desc
            )));
        }

        // 返回结果
        response
            .ret
            .ok_or_else(|| ProtocolError::CommandFailed("无响应数据".to_string()))
    }

    /// 测试 QGA 连通性
    pub async fn ping(&self) -> Result<()> {
        #[derive(Serialize)]
        struct Empty {}
        #[derive(Deserialize)]
        struct PingResponse {}

        self.execute_command::<Empty, PingResponse>("guest-ping", None)
            .await?;
        info!("QGA 连通性测试成功");
        Ok(())
    }

    /// 执行命令（异步启动）
    pub async fn exec(&self, cmd: GuestExecCommand) -> Result<i64> {
        info!("执行 Guest 命令: {}", cmd.path);
        let result: GuestExecResult = self.execute_command("guest-exec", Some(cmd)).await?;
        info!("命令已启动，PID: {}", result.pid);
        Ok(result.pid)
    }

    /// 查询命令执行状态
    pub async fn exec_status(&self, pid: i64) -> Result<GuestExecStatus> {
        debug!("查询进程状态: PID {}", pid);
        let request = GuestExecStatusRequest { pid };
        self.execute_command("guest-exec-status", Some(request))
            .await
    }

    /// 执行命令并等待完成
    pub async fn exec_and_wait(&self, cmd: GuestExecCommand) -> Result<GuestExecStatus> {
        let pid = self.exec(cmd).await?;
        info!("等待进程完成: PID {}", pid);

        // 轮询直到进程退出
        loop {
            let status = self.exec_status(pid).await?;
            if status.exited {
                info!("进程已退出: PID {}, 退出码: {:?}", pid, status.exit_code);
                return Ok(status);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    /// 执行 Shell 命令（便捷方法）
    pub async fn exec_shell(&self, shell_cmd: &str) -> Result<GuestExecStatus> {
        info!("执行 Shell 命令: {}", shell_cmd);

        // 默认使用 Linux Shell
        let cmd =
            GuestExecCommand::simple("/bin/sh", vec!["-c".to_string(), shell_cmd.to_string()]);

        self.exec_and_wait(cmd).await
    }

    /// 跨平台执行后台程序
    ///
    /// 根据目标操作系统类型，使用适当的方式启动后台程序
    ///
    /// # 参数
    /// * `os_type` - 操作系统类型 ("linux" 或 "windows")
    /// * `program_path` - 程序路径
    /// * `args` - 程序参数列表
    ///
    /// # 返回
    /// 返回启动的进程 PID（对于后台进程可能不准确）
    pub async fn exec_background(
        &self,
        os_type: &str,
        program_path: &str,
        args: &[String],
    ) -> Result<i64> {
        info!(
            "启动后台程序: {} {} (OS: {})",
            program_path,
            args.join(" "),
            os_type
        );

        let args_str = args.join(" ");

        match os_type.to_lowercase().as_str() {
            "linux" => {
                // Linux: 使用 nohup 和 & 后台运行
                let shell_cmd = format!(
                    "nohup {} {} > /tmp/verifier-agent.log 2>&1 & echo $!",
                    program_path, args_str
                );
                let cmd =
                    GuestExecCommand::simple("/bin/sh", vec!["-c".to_string(), shell_cmd.clone()]);
                let status = self.exec_and_wait(cmd).await?;

                // 尝试从输出获取 PID
                if let Some(stdout) = status.decode_stdout() {
                    if let Ok(pid) = stdout.trim().parse::<i64>() {
                        info!("后台进程已启动，PID: {}", pid);
                        return Ok(pid);
                    }
                }

                // 如果无法获取 PID，返回 0 表示已启动但 PID 未知
                Ok(0)
            }
            "windows" => {
                // Windows: 使用 PowerShell 的 Start-Process
                let args_escaped = args
                    .iter()
                    .map(|a| format!("'{}'", a.replace('\'', "''")))
                    .collect::<Vec<_>>()
                    .join(",");

                let powershell_cmd = format!(
                    "$proc = Start-Process -FilePath '{}' -ArgumentList {} -PassThru -WindowStyle Hidden; $proc.Id",
                    program_path.replace('\'', "''"),
                    args_escaped
                );

                let cmd = GuestExecCommand::simple(
                    "powershell.exe",
                    vec!["-Command".to_string(), powershell_cmd],
                );
                let status = self.exec_and_wait(cmd).await?;

                // 尝试从输出获取 PID
                if let Some(stdout) = status.decode_stdout() {
                    if let Ok(pid) = stdout.trim().parse::<i64>() {
                        info!("后台进程已启动，PID: {}", pid);
                        return Ok(pid);
                    }
                }

                Ok(0)
            }
            _ => Err(ProtocolError::CommandFailed(format!(
                "不支持的操作系统类型: {}",
                os_type
            ))),
        }
    }

    /// 获取 Guest 操作系统信息
    pub async fn get_os_info(&self) -> Result<GuestOsInfo> {
        info!("获取 Guest 操作系统信息");
        self.execute_command::<(), GuestOsInfo>("guest-get-osinfo", None)
            .await
    }
}

impl Default for QgaProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Protocol for QgaProtocol {
    async fn connect(&mut self, domain: &Domain) -> Result<()> {
        info!("连接到 QGA");

        // 克隆 domain（libvirt Domain 支持 clone）
        let domain_clone = domain.clone();

        self.domain = Some(Arc::new(Mutex::new(domain_clone)));
        self.connected = true;

        // 测试连通性
        self.ping().await?;

        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(ProtocolError::ConnectionFailed("QGA 未连接".to_string()));
        }

        let domain_guard = self
            .domain
            .as_ref()
            .ok_or_else(|| ProtocolError::ConnectionFailed("Domain 不可用".to_string()))?;

        let cmd_json = String::from_utf8(data.to_vec())
            .map_err(|e| ProtocolError::SendFailed(format!("数据不是有效的 UTF-8: {}", e)))?;

        let domain_guard = domain_guard.lock().await;
        let domain_clone = domain_guard.clone();
        let timeout = self.timeout;
        drop(domain_guard);

        tokio::task::spawn_blocking(move || domain_clone.qemu_agent_command(&cmd_json, timeout, 0))
            .await
            .map_err(|e| ProtocolError::SendFailed(format!("任务执行失败: {}", e)))?
            .map_err(|e| ProtocolError::SendFailed(format!("发送失败: {}", e)))?;

        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        // QGA 是请求-响应模式，不支持单独的 receive
        // 这里返回空，实际的接收在 execute_command 中处理
        Err(ProtocolError::CommandFailed(
            "QGA 不支持独立的 receive 操作，请使用 execute_command".to_string(),
        ))
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.domain = None;
        self.connected = false;
        info!("QGA 连接已断开");
        Ok(())
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::QGA
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }
}

// ============================================================================
// QGA 协议构建器
// ============================================================================

/// QGA 协议构建器
pub struct QgaProtocolBuilder {
    timeout: i32,
}

impl QgaProtocolBuilder {
    pub fn new() -> Self {
        Self { timeout: 30 }
    }

    pub fn with_timeout(mut self, timeout: i32) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Default for QgaProtocolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolBuilder for QgaProtocolBuilder {
    fn build(&self) -> Box<dyn Protocol> {
        Box::new(QgaProtocol::new().with_timeout(self.timeout))
    }

    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::QGA
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guest_exec_command_creation() {
        let cmd = GuestExecCommand::simple("/bin/ls", vec!["-la".to_string()]);
        assert_eq!(cmd.path, "/bin/ls");
        assert_eq!(cmd.capture_output, Some(true));
    }
}

//! 命令执行验证器实现

use async_trait::async_trait;
use serde_json::json;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::{debug, error, info};
use verifier_core::{Event, Result, Verifier, VerifierError, VerifierType, VerifyResult};

/// 命令执行验证器
pub struct CommandVerifier {}

impl CommandVerifier {
    /// 创建新的命令验证器
    pub fn new() -> Result<Self> {
        info!("初始化命令验证器");
        Ok(Self {})
    }

    /// 执行命令并获取输出
    async fn execute_command(&self, cmd: &str, args: &[String]) -> Result<CommandResult> {
        debug!("执行命令: {} {:?}", cmd, args);

        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let start_time = tokio::time::Instant::now();

        let mut child = command.spawn().map_err(|e| {
            error!("启动命令失败: {}", e);
            VerifierError::IoError(e)
        })?;

        // 获取 stdout 和 stderr
        let mut stdout = child.stdout.take().ok_or_else(|| {
            VerifierError::VerificationFailed("无法获取 stdout".to_string())
        })?;

        let mut stderr = child.stderr.take().ok_or_else(|| {
            VerifierError::VerificationFailed("无法获取 stderr".to_string())
        })?;

        // 读取输出
        let stdout_read = tokio::spawn(async move {
            let mut data = Vec::new();
            stdout.read_to_end(&mut data).await.ok();
            data
        });

        let stderr_read = tokio::spawn(async move {
            let mut data = Vec::new();
            stderr.read_to_end(&mut data).await.ok();
            data
        });

        // 等待命令完成
        let status = child.wait().await.map_err(|e| {
            error!("等待命令完成失败: {}", e);
            VerifierError::IoError(e)
        })?;

        // 获取输出数据
        let stdout_data = stdout_read.await.map_err(|e| {
            error!("读取 stdout 失败: {}", e);
            VerifierError::VerificationFailed(format!("读取 stdout 失败: {}", e))
        })?;

        let stderr_data = stderr_read.await.map_err(|e| {
            error!("读取 stderr 失败: {}", e);
            VerifierError::VerificationFailed(format!("读取 stderr 失败: {}", e))
        })?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        let result = CommandResult {
            exit_code: status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&stdout_data).to_string(),
            stderr: String::from_utf8_lossy(&stderr_data).to_string(),
            execution_time_ms: execution_time,
        };

        debug!("命令执行完成: exit_code={}, execution_time={}ms",
               result.exit_code, result.execution_time_ms);

        Ok(result)
    }

    /// 验证命令执行结果
    fn verify_result(&self, result: &CommandResult, expected: &CommandExpectation) -> bool {
        // 检查退出码
        if let Some(expected_code) = expected.exit_code {
            if result.exit_code != expected_code {
                debug!("退出码不匹配: 期望 {}, 实际 {}", expected_code, result.exit_code);
                return false;
            }
        }

        // 检查 stdout 包含
        if let Some(ref stdout_contains) = expected.stdout_contains {
            if !result.stdout.contains(stdout_contains) {
                debug!("stdout 不包含预期字符串: {}", stdout_contains);
                return false;
            }
        }

        // 检查 stderr 包含
        if let Some(ref stderr_contains) = expected.stderr_contains {
            if !result.stderr.contains(stderr_contains) {
                debug!("stderr 不包含预期字符串: {}", stderr_contains);
                return false;
            }
        }

        true
    }
}

impl Default for CommandVerifier {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// 命令执行结果
#[derive(Debug, Clone)]
struct CommandResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
    execution_time_ms: u64,
}

/// 命令期望结果
#[derive(Debug, Clone)]
struct CommandExpectation {
    exit_code: Option<i32>,
    stdout_contains: Option<String>,
    stderr_contains: Option<String>,
}

#[async_trait]
impl Verifier for CommandVerifier {
    async fn verify(&self, event: Event) -> Result<VerifyResult> {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        debug!("验证命令执行事件: {:?}", event);

        // 从事件数据中提取命令信息
        let command = event
            .data
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                VerifierError::VerificationFailed("事件缺少 command 字段".to_string())
            })?;

        // 提取参数
        let args: Vec<String> = event
            .data
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // 提取期望结果
        let expectation = CommandExpectation {
            exit_code: event
                .data
                .get("expected_exit_code")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
            stdout_contains: event
                .data
                .get("expected_stdout_contains")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            stderr_contains: event
                .data
                .get("expected_stderr_contains")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        // 执行命令
        let result = self.execute_command(command, &args).await?;

        // 验证结果
        let verified = self.verify_result(&result, &expectation);

        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let latency_ms = (end_time - start_time) as u64;

        Ok(VerifyResult {
            message_type: "verify_result".to_string(),
            event_id: event
                .data
                .get("event_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            verified,
            timestamp: end_time,
            latency_ms,
            details: json!({
                "command": command,
                "args": args,
                "exit_code": result.exit_code,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "execution_time_ms": result.execution_time_ms,
                "expectation": {
                    "exit_code": expectation.exit_code,
                    "stdout_contains": expectation.stdout_contains,
                    "stderr_contains": expectation.stderr_contains,
                }
            }),
        })
    }

    fn verifier_type(&self) -> VerifierType {
        VerifierType::Command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_command_verifier_creation() {
        let verifier = CommandVerifier::new();
        assert!(verifier.is_ok());
    }

    #[tokio::test]
    async fn test_simple_command() {
        let verifier = CommandVerifier::new().unwrap();

        let result = verifier
            .execute_command("echo", &["hello".to_string()])
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
    }
}

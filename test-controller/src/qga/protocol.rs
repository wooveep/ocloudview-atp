use serde::{Deserialize, Serialize};

/// QGA 命令的通用结构
#[derive(Debug, Serialize)]
pub struct QgaCommand<'a, T> {
    pub execute: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<T>,
}

/// QGA 响应的通用结构
#[derive(Debug, Deserialize)]
pub struct QgaResponse<T> {
    #[serde(rename = "return")]
    pub ret: Option<T>,
    pub error: Option<QgaError>,
}

/// QGA 错误信息
#[derive(Debug, Deserialize)]
pub struct QgaError {
    #[serde(rename = "class")]
    pub error_class: String,
    pub desc: String,
}

// ============================================================================
// guest-exec: 在 Guest 中执行命令
// ============================================================================

/// guest-exec 命令参数
#[derive(Debug, Serialize)]
pub struct GuestExecCommand {
    /// 要执行的命令路径（绝对路径）
    pub path: String,

    /// 命令参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arg: Option<Vec<String>>,

    /// 环境变量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,

    /// 标准输入数据（Base64 编码）
    #[serde(skip_serializing_if = "Option::is_none", rename = "input-data")]
    pub input_data: Option<String>,

    /// 是否捕获标准输出
    #[serde(skip_serializing_if = "Option::is_none", rename = "capture-output")]
    pub capture_output: Option<bool>,
}

/// guest-exec 返回结果
#[derive(Debug, Deserialize)]
pub struct GuestExecResult {
    /// 进程 ID
    pub pid: i64,
}

// ============================================================================
// guest-exec-status: 查询命令执行状态
// ============================================================================

/// guest-exec-status 请求参数
#[derive(Debug, Serialize)]
pub struct GuestExecStatusRequest {
    /// 进程 ID
    pub pid: i64,
}

/// guest-exec-status 返回结果
#[derive(Debug, Deserialize)]
pub struct GuestExecStatus {
    /// 进程是否已退出
    pub exited: bool,

    /// 退出码（如果已退出）
    #[serde(rename = "exitcode")]
    pub exit_code: Option<i32>,

    /// 信号编号（如果被信号终止）
    pub signal: Option<i32>,

    /// 标准输出（Base64 编码）
    #[serde(rename = "out-data")]
    pub out_data: Option<String>,

    /// 标准错误（Base64 编码）
    #[serde(rename = "err-data")]
    pub err_data: Option<String>,

    /// 输出是否被截断
    #[serde(rename = "out-truncated")]
    pub out_truncated: Option<bool>,

    #[serde(rename = "err-truncated")]
    pub err_truncated: Option<bool>,
}

// ============================================================================
// guest-info: 获取 Guest Agent 信息
// ============================================================================

/// guest-info 返回结果
#[derive(Debug, Deserialize)]
pub struct GuestInfo {
    /// QGA 版本
    pub version: String,

    /// 支持的命令列表
    #[serde(rename = "supported_commands")]
    pub supported_commands: Vec<GuestCommandInfo>,
}

#[derive(Debug, Deserialize)]
pub struct GuestCommandInfo {
    pub name: String,
    pub enabled: bool,
    #[serde(rename = "success-response")]
    pub success_response: bool,
}

/// guest-get-osinfo 返回结果
#[derive(Debug, Deserialize)]
pub struct GuestOsInfo {
    /// 操作系统 ID
    pub id: Option<String>,

    /// 操作系统名称
    pub name: Option<String>,

    /// 操作系统版本
    pub version: Option<String>,

    /// 操作系统版本 ID
    #[serde(rename = "version-id")]
    pub version_id: Option<String>,

    /// 漂亮的名称
    #[serde(rename = "pretty-name")]
    pub pretty_name: Option<String>,

    /// 内核版本
    #[serde(rename = "kernel-version")]
    pub kernel_version: Option<String>,

    /// 内核发行版
    #[serde(rename = "kernel-release")]
    pub kernel_release: Option<String>,

    /// 机器架构
    pub machine: Option<String>,
}

// ============================================================================
// guest-file-*: 文件操作
// ============================================================================

/// guest-file-open 参数
#[derive(Debug, Serialize)]
pub struct GuestFileOpen {
    /// 文件路径
    pub path: String,

    /// 打开模式: "r", "w", "a", "r+", "w+", "a+"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// guest-file-open 返回结果
#[derive(Debug, Deserialize)]
pub struct GuestFileHandle {
    /// 文件句柄
    pub handle: i64,
}

/// guest-file-read 参数
#[derive(Debug, Serialize)]
pub struct GuestFileRead {
    /// 文件句柄
    pub handle: i64,

    /// 读取字节数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
}

/// guest-file-read 返回结果
#[derive(Debug, Deserialize)]
pub struct GuestFileReadResult {
    /// 读取的数据（Base64 编码）
    #[serde(rename = "buf-b64")]
    pub buf_b64: String,

    /// 读取的字节数
    pub count: i64,

    /// 是否到达文件末尾
    pub eof: bool,
}

/// guest-file-write 参数
#[derive(Debug, Serialize)]
pub struct GuestFileWrite {
    /// 文件句柄
    pub handle: i64,

    /// 要写入的数据（Base64 编码）
    #[serde(rename = "buf-b64")]
    pub buf_b64: String,
}

/// guest-file-write 返回结果
#[derive(Debug, Deserialize)]
pub struct GuestFileWriteResult {
    /// 实际写入的字节数
    pub count: i64,

    /// 是否到达文件末尾
    pub eof: bool,
}

/// guest-file-close 参数
#[derive(Debug, Serialize)]
pub struct GuestFileClose {
    /// 文件句柄
    pub handle: i64,
}

// ============================================================================
// 辅助方法
// ============================================================================

impl GuestExecCommand {
    /// 创建简单的命令执行（捕获输出）
    pub fn simple(path: &str, args: Vec<String>) -> Self {
        Self {
            path: path.to_string(),
            arg: Some(args),
            env: None,
            input_data: None,
            capture_output: Some(true),
        }
    }

    /// 创建带输入的命令
    pub fn with_input(path: &str, args: Vec<String>, input: &str) -> Self {
        use base64::{Engine as _, engine::general_purpose};

        let input_b64 = general_purpose::STANDARD.encode(input.as_bytes());

        Self {
            path: path.to_string(),
            arg: Some(args),
            env: None,
            input_data: Some(input_b64),
            capture_output: Some(true),
        }
    }
}

impl GuestExecStatus {
    /// 解码标准输出
    pub fn decode_stdout(&self) -> Option<String> {
        use base64::{Engine as _, engine::general_purpose};

        self.out_data.as_ref().and_then(|data| {
            general_purpose::STANDARD
                .decode(data)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        })
    }

    /// 解码标准错误
    pub fn decode_stderr(&self) -> Option<String> {
        use base64::{Engine as _, engine::general_purpose};

        self.err_data.as_ref().and_then(|data| {
            general_purpose::STANDARD
                .decode(data)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        })
    }
}

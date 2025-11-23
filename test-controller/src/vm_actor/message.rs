use serde::{Deserialize, Serialize};

/// VM Actor 消息类型
#[derive(Debug, Clone)]
pub enum VmMessage {
    Command(VmCommand),
    Event(VmEvent),
}

/// VM 命令（从 Orchestrator 发送到 Actor）
#[derive(Debug, Clone)]
pub enum VmCommand {
    /// 发送按键序列
    SendKeys { keys: Vec<String> },

    /// 发送文本
    SendText { text: String },

    /// 查询虚拟机状态
    QueryStatus,

    /// 等待 Guest Agent 连接
    WaitForAgent { timeout_secs: u64 },

    /// 运行测试用例
    RunTestCase { test_id: String },

    // ========== QGA 命令 ==========
    /// 执行 Shell 命令
    ExecShellCommand { command: String },

    /// 读取 Guest 文件内容
    ReadGuestFile { path: String },

    /// 写入 Guest 文件内容
    WriteGuestFile { path: String, content: String },

    /// 获取 Guest 操作系统信息
    GetGuestOsInfo,

    /// 停止 Actor
    Shutdown,
}

/// VM 事件（从 Actor 发送到 Orchestrator）
#[derive(Debug, Clone)]
pub enum VmEvent {
    /// Actor 已启动
    Started { vm_name: String },

    /// Guest Agent 已连接
    AgentConnected,

    /// Guest Agent 已断开
    AgentDisconnected,

    /// 收到按键事件
    KeyEventReceived { event: GuestKeyEvent },

    /// 测试用例完成
    TestCaseCompleted { test_id: String, passed: bool },

    // ========== QGA 事件 ==========
    /// Shell 命令执行完成
    ShellCommandCompleted {
        command: String,
        exit_code: i32,
        stdout: String,
        stderr: String,
    },

    /// 文件读取完成
    FileReadCompleted { path: String, content: String },

    /// 文件写入完成
    FileWriteCompleted { path: String },

    /// Guest OS 信息
    GuestOsInfoReceived { os_info: String },

    /// 错误发生
    Error { message: String },

    /// Actor 已停止
    Stopped,
}

/// Guest 端回传的按键事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestKeyEvent {
    /// 事件类型 (keydown, keyup, keypress)
    #[serde(rename = "type")]
    pub event_type: String,

    /// 按键值 (如 'a', 'Enter')
    pub key: String,

    /// 按键码 (如 'KeyA', 'Enter')
    pub code: String,

    /// 数字键码
    #[serde(rename = "keyCode")]
    pub key_code: u32,

    /// 修饰键状态
    #[serde(rename = "ctrlKey")]
    pub ctrl_key: bool,

    #[serde(rename = "shiftKey")]
    pub shift_key: bool,

    #[serde(rename = "altKey")]
    pub alt_key: bool,

    #[serde(rename = "metaKey")]
    pub meta_key: bool,

    /// 时间戳
    #[serde(rename = "timeStamp")]
    pub timestamp: f64,

    /// 是否为可信事件
    #[serde(rename = "isTrusted")]
    pub is_trusted: bool,
}

impl GuestKeyEvent {
    /// 验证事件是否匹配预期的按键
    pub fn matches_key(&self, expected_key: &str) -> bool {
        self.key.to_lowercase() == expected_key.to_lowercase()
            || self.code.to_lowercase() == expected_key.to_lowercase()
    }
}

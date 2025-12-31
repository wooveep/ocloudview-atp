//! 传输层模块

pub mod websocket;
pub mod tcp;

pub use websocket::WebSocketTransport;
pub use tcp::TcpTransport;

use async_trait::async_trait;
use crate::{Event, RawInputEvent, VerifiedInputEvent, Result, VerifyResult};

/// 传输层抽象接口
#[async_trait]
pub trait VerifierTransport: Send + Sync {
    /// 连接到服务器并发送 VM ID
    ///
    /// # 参数
    /// - `endpoint`: 服务器地址
    /// - `vm_id`: 虚拟机 ID（可选，用于客户端标识）
    async fn connect(&mut self, endpoint: &str, vm_id: Option<&str>) -> Result<()>;

    /// 发送验证结果
    async fn send_result(&mut self, result: &VerifyResult) -> Result<()>;

    /// 发送已验证的输入事件（用于调试）
    async fn send_input_event(&mut self, event: &VerifiedInputEvent) -> Result<()>;

    /// 发送原始输入事件（输入上报模式）
    async fn send_raw_input_event(&mut self, event: &RawInputEvent) -> Result<()>;

    /// 接收事件
    async fn receive_event(&mut self) -> Result<Event>;

    /// 断开连接
    async fn disconnect(&mut self) -> Result<()>;
}

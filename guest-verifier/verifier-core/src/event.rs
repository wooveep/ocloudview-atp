//! 事件和结果定义

// 从 atp-common 重新导出共享类型
pub use atp_common::{Event, RawInputEvent, VerifiedInputEvent, VerifyResult};

// 为了向后兼容，保留 InputEvent 别名
#[deprecated(since = "0.1.0", note = "请使用 VerifiedInputEvent")]
pub type InputEvent = VerifiedInputEvent;


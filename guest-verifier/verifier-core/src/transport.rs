//! 传输层接口

use async_trait::async_trait;
use crate::{Event, Result, VerifyResult};

#[async_trait]
pub trait VerifierTransport: Send + Sync {
    async fn connect(&mut self, endpoint: &str) -> Result<()>;
    async fn send_result(&mut self, result: &VerifyResult) -> Result<()>;
    async fn receive_event(&mut self) -> Result<Event>;
}

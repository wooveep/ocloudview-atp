//! 验证器接口

use async_trait::async_trait;
use crate::{Event, Result, VerifyResult};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VerifierType {
    Keyboard,
    Mouse,
    Command,
    Custom(String),
}

#[async_trait]
pub trait Verifier: Send + Sync {
    async fn verify(&self, event: Event) -> Result<VerifyResult>;
    fn verifier_type(&self) -> VerifierType;
}

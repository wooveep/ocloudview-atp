//! 协议注册表

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::{Protocol, ProtocolBuilder, ProtocolError, ProtocolType, Result};

/// 协议注册表
///
/// 管理所有已注册的协议
pub struct ProtocolRegistry {
    /// 协议构建器映射
    builders: Arc<RwLock<HashMap<String, Box<dyn ProtocolBuilder>>>>,
}

impl ProtocolRegistry {
    /// 创建新的协议注册表
    pub fn new() -> Self {
        Self {
            builders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册协议
    pub async fn register(&self, name: &str, builder: Box<dyn ProtocolBuilder>) -> Result<()> {
        info!("注册协议: {}", name);

        let mut builders = self.builders.write().await;

        if builders.contains_key(name) {
            return Err(ProtocolError::ProtocolAlreadyRegistered(
                name.to_string(),
            ));
        }

        builders.insert(name.to_string(), builder);

        Ok(())
    }

    /// 注销协议
    pub async fn unregister(&self, name: &str) -> Result<()> {
        info!("注销协议: {}", name);

        let mut builders = self.builders.write().await;

        builders
            .remove(name)
            .ok_or_else(|| ProtocolError::ProtocolNotFound(name.to_string()))?;

        Ok(())
    }

    /// 获取协议实例
    pub async fn get(&self, name: &str) -> Result<Box<dyn Protocol>> {
        debug!("获取协议实例: {}", name);

        let builders = self.builders.read().await;

        let builder = builders
            .get(name)
            .ok_or_else(|| ProtocolError::ProtocolNotFound(name.to_string()))?;

        Ok(builder.build())
    }

    /// 列出所有已注册的协议
    pub async fn list(&self) -> Vec<String> {
        let builders = self.builders.read().await;
        builders.keys().cloned().collect()
    }

    /// 检查协议是否已注册
    pub async fn is_registered(&self, name: &str) -> bool {
        let builders = self.builders.read().await;
        builders.contains_key(name)
    }
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = ProtocolRegistry::new();
        assert_eq!(registry.list().await.len(), 0);
    }
}

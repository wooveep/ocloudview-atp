//! 组织单位管理 API
//!
//! 提供组织单位/部门的查询功能，用于：
//! - 获取组织单位树结构
//! - 按名称查找组织单位

use tracing::info;

use crate::client::VdiClient;
use crate::error::{Result, VdiError};
use crate::models::{UserGroup, find_group_by_name};

/// 组织单位管理 API
pub struct GroupApi<'a> {
    client: &'a VdiClient,
}

impl<'a> GroupApi<'a> {
    /// 创建新的组织单位 API 实例
    pub(crate) fn new(client: &'a VdiClient) -> Self {
        Self { client }
    }

    /// 获取组织单位树
    ///
    /// 调用 GET /ocloud/v1/group/tree
    /// 返回所有组织单位的树形结构
    pub async fn tree(&self) -> Result<Vec<UserGroup>> {
        info!("查询组织单位树");

        let url = "/ocloud/v1/group/tree";
        let token = self.client.get_token().await?;

        let response: serde_json::Value = self
            .client
            .http_client()
            .get(&format!("{}{}", self.client.base_url(), url))
            .header("Token", &token)
            .send()
            .await
            .map_err(|e| VdiError::HttpError(e.to_string()))?
            .json()
            .await
            .map_err(|e| VdiError::ParseError(e.to_string()))?;

        if response["status"].as_i64().unwrap_or(-1) != 0 {
            let msg = response["msg"].as_str().unwrap_or("未知错误");
            return Err(VdiError::ApiError(500, msg.to_string()));
        }

        // 解析组织单位树
        let groups: Vec<UserGroup> = response["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();

        info!("获取到 {} 个顶级组织单位", groups.len());
        Ok(groups)
    }

    /// 按名称查找组织单位
    ///
    /// 在组织单位树中递归搜索指定名称的组织单位
    ///
    /// # Arguments
    /// * `name` - 组织单位名称
    ///
    /// # Returns
    /// 找到的组织单位，如果不存在则返回 None
    pub async fn find_by_name(&self, name: &str) -> Result<Option<UserGroup>> {
        let tree = self.tree().await?;
        Ok(find_group_by_name(&tree, name).cloned())
    }

    /// 获取所有组织单位（扁平化列表）
    ///
    /// 将树形结构展开为一维列表
    pub async fn list_all(&self) -> Result<Vec<UserGroup>> {
        let tree = self.tree().await?;
        let mut result = Vec::new();
        for group in &tree {
            for g in group.flatten() {
                result.push(g.clone());
            }
        }
        Ok(result)
    }
}

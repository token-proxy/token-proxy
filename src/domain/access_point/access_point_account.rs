//! 接入点账户池条目 — domain/access_point/
//!
//! 定义 `AccessPointAccount` 值对象，记录接入点与 API 账号的关联关系，
//! 包含路由所需的权重和优先级字段。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 接入点账户池中的账户条目，记录关联的 API 账号及其权重和优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPointAccount {
    pub account_id: Uuid,
    /// 权重，用于加权随机路由策略
    pub weight: i32,
    /// 优先级，值越小优先级越高
    pub priority: i32,
}

impl AccessPointAccount {
    /// 创建新的账户池条目
    pub fn new(account_id: Uuid, weight: i32, priority: i32) -> Self {
        AccessPointAccount {
            account_id,
            weight,
            priority,
        }
    }
}

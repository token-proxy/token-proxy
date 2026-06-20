use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 接入点-账户关联 DTO
///
/// 用于接入点账户池的请求和响应，描述账号在接入点中的路由权重和优先级。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDto {
    pub account_id: Uuid,
    /// 权重（加权路由策略使用）
    pub weight: Option<i32>,
    /// 优先级（优先级路由策略使用，数值越小优先级越高）
    pub priority: Option<i32>,
}

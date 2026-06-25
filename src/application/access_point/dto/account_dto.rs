use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 接入点-账户关联 DTO
///
/// 同时用于请求（创建/更新时仅需 `account_id`、`weight`、`priority`）
/// 和响应（额外返回 `provider_id` 和 `status`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDto {
    pub account_id: Uuid,
    /// 权重（加权路由策略使用）
    pub weight: Option<i32>,
    /// 优先级（优先级路由策略使用，数值越小优先级越高）
    pub priority: Option<i32>,
    /// 所属服务商 ID（仅响应）
    #[serde(default)]
    pub provider_id: Option<Uuid>,
    /// 账号状态："enabled" | "disabled"（仅响应）
    #[serde(default)]
    pub status: Option<String>,
}

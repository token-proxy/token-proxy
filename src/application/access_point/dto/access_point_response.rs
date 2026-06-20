use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use super::account_dto::AccountDto;
use super::model_routing_grid_dto::ModelRoutingGridDto;

/// 接入点响应体
///
/// 包含接入点的基本信息、账户池和模型路由网格。
#[derive(Debug, Clone, Serialize)]
pub struct AccessPointResponse {
    pub id: Uuid,
    pub name: String,
    /// 接入类型（如 anthropic、openai-compatible）
    pub api_type: String,
    /// 唯一短码，用于构造代理 URL
    pub short_code: String,
    /// 账户池列表
    pub accounts: Vec<AccountDto>,
    /// 路由策略（priority / weighted）
    pub routing_strategy: String,
    /// 模型路由网格定义
    pub model_routing_grid: ModelRoutingGridDto,
    /// 对外代理 URL（如 /ap/<short_code>）
    pub access_url: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

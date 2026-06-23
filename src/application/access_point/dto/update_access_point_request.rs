use serde::Deserialize;

use super::account_dto::AccountDto;
use super::model_routing_grid_dto::ModelRoutingGridDto;

/// 更新接入点请求体
///
/// 所有字段可选，仅提供的字段会被更新。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAccessPointRequest {
    /// 接入点名称
    pub name: Option<String>,
    /// API 类型（anthropic / openai）
    pub api_type: Option<String>,
    /// 账户池（全量替换）
    pub accounts: Option<Vec<AccountDto>>,
    /// 路由策略（priority / weighted）
    pub routing_strategy: Option<String>,
    /// 模型路由网格（全量替换）
    pub model_routing_grid: Option<ModelRoutingGridDto>,
    /// 状态（enabled / disabled）
    pub status: Option<String>,
}

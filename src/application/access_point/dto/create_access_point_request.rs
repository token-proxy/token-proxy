use serde::Deserialize;

use super::account_dto::AccountDto;
use super::model_routing_grid_dto::ModelRoutingGridDto;

/// 创建接入点请求体
///
/// 名称必填，其余字段可选（系统会填充默认值）。
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAccessPointRequest {
    /// 接入点名称（必填）
    pub name: String,
    /// 自定义短码（可选，不提供则自动生成 16 位随机短码）
    pub short_code: Option<String>,
    /// 接入类型（可选，默认 anthropic）
    pub api_type: Option<String>,
    /// 初始账户池（可选）
    pub accounts: Option<Vec<AccountDto>>,
    /// 路由策略（可选，默认 priority）
    pub routing_strategy: Option<String>,
    /// 模型路由网格（可选）
    pub model_routing_grid: Option<ModelRoutingGridDto>,
}

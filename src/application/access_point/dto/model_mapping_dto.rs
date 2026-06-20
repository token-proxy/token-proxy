use serde::{Deserialize, Serialize};

/// 模型映射 DTO（已弃用，保留兼容）
///
/// 旧版模型一对一映射结构，当前使用 ModelRoutingGridDto。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMappingDto {
    pub source_model: String,
    pub target_model: String,
    /// 匹配类型（exact / prefix / unmatched）
    #[serde(default)]
    pub match_type: String,
}

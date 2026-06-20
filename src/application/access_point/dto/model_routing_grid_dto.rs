use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 模型路由网格 DTO
///
/// 定义接入点中 source_model 到各 Provider 目标模型的映射关系。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingGridDto {
    /// 网格关联的 Provider ID 列表
    pub provider_ids: Vec<Uuid>,
    /// 路由规则行
    pub rows: Vec<ModelRoutingRowDto>,
}

/// 模型路由规则行 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingRowDto {
    /// 原始模型名（支持 `*` 前缀通配和 `__unmatched__` 兜底）
    pub source_model: String,
    /// 目标映射，key 为 provider_id，value 为目标模型名（None 表示透传原模型）
    pub targets: HashMap<Uuid, Option<String>>,
}

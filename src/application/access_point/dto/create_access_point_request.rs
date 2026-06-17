use serde::Deserialize;
use uuid::Uuid;

use super::model_mapping_dto::ModelMappingDto;

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAccessPointRequest {
    pub name: String,
    pub provider_id: Uuid,
    pub account_id: Uuid,
    pub short_code: Option<String>,
    pub api_type: Option<String>,
    pub model_mappings: Option<Vec<ModelMappingDto>>,
    pub default_model: Option<String>,
}

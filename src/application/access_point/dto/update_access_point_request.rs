use serde::Deserialize;
use uuid::Uuid;

use super::model_mapping_dto::ModelMappingDto;

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAccessPointRequest {
    pub name: Option<String>,
    pub provider_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub model_mappings: Option<Vec<ModelMappingDto>>,
    pub default_model: Option<String>,
    pub status: Option<String>,
}

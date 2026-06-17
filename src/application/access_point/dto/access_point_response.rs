use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use super::model_mapping_dto::ModelMappingDto;

#[derive(Debug, Clone, Serialize)]
pub struct AccessPointResponse {
    pub id: Uuid,
    pub name: String,
    pub api_type: String,
    pub short_code: String,
    pub provider_id: Uuid,
    pub account_id: Uuid,
    pub model_mappings: Vec<ModelMappingDto>,
    pub default_model: Option<String>,
    pub access_url: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

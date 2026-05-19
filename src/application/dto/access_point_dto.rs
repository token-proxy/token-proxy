use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ─── Model Mapping DTO ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMappingDto {
    pub source_model: String,
    pub target_model: String,
}

// ─── Request DTOs ───

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAccessPointRequest {
    pub name: String,
    pub provider_id: Uuid,
    pub account_id: Uuid,
    pub short_code: Option<String>,
    pub model_mappings: Option<Vec<ModelMappingDto>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAccessPointRequest {
    pub name: Option<String>,
    pub provider_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub model_mappings: Option<Vec<ModelMappingDto>>,
    pub status: Option<String>,
}

// ─── Response DTOs ───

#[derive(Debug, Clone, Serialize)]
pub struct AccessPointResponse {
    pub id: Uuid,
    pub name: String,
    pub api_type: String,
    pub short_code: String,
    pub provider_id: Uuid,
    pub account_id: Uuid,
    pub model_mappings: Vec<ModelMappingDto>,
    pub access_url: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
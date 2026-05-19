use crate::domain::value_objects::access_point_type::AccessPointType;
use crate::domain::value_objects::model_mapping::ModelMapping;
use crate::domain::value_objects::short_code::ShortCode;
use crate::domain::value_objects::status::Status;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPoint {
    pub id: Uuid,
    pub name: String,
    pub api_type: AccessPointType,
    pub short_code: ShortCode,
    pub provider_id: Uuid,
    pub account_id: Uuid,
    pub model_mappings: Vec<ModelMapping>,
    pub status: Status,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AccessPoint {
    /// 创建新的 AccessPoint
    pub fn new(
        name: String,
        api_type: AccessPointType,
        short_code: ShortCode,
        provider_id: Uuid,
        account_id: Uuid,
        created_by: Uuid,
    ) -> Self {
        let now = Utc::now();
        AccessPoint {
            id: Uuid::new_v4(),
            name,
            api_type,
            short_code,
            provider_id,
            account_id,
            model_mappings: Vec::new(),
            status: Status::Enabled,
            created_by,
            created_at: now,
            updated_at: now,
        }
    }
}
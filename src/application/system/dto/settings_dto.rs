use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct SettingsResponse {
    pub log_retention_months: i16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSettingsRequest {
    pub log_retention_months: i16,
}

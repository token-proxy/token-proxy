use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMappingDto {
    pub source_model: String,
    pub target_model: String,
    #[serde(default)]
    pub match_type: String,
}

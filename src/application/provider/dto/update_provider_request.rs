use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub models: Option<Vec<String>>,
    pub status: Option<String>,
}

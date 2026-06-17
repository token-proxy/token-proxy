use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
}

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAccountRequest {
    pub name: Option<String>,
    pub api_key: String,
}

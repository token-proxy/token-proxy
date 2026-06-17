use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAccountRequest {
    pub name: Option<String>,
    pub api_key: Option<String>,
}

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub password: Option<String>,
    pub status: Option<String>,
}

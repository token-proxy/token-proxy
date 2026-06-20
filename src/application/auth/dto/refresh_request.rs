use serde::Deserialize;

/// Token 刷新请求体
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

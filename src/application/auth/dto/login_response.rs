use serde::Serialize;

/// 登录响应体（含 access_token 和 refresh_token）
#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    /// access_token 有效期（秒）
    pub expires_in: u64,
    pub username: String,
    pub display_name: String,
}

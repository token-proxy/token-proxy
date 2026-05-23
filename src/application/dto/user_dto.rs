use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Request DTOs ───

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub password: Option<String>,
    pub status: Option<String>,
}

/// 更新当前用户 profile 请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: String,
}

/// 修改密码请求
#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 创建 API key 请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub description: String,
}

// ─── Response DTOs ───

#[derive(Debug, Clone, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API key 列表响应（脱敏，不返回完整 key）
#[derive(Debug, Clone, Serialize)]
pub struct UserApiKeyResponse {
    pub id: Uuid,
    pub key_prefix: String,
    pub description: String,
    pub status: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 创建 API key 响应（完整 key 仅在创建时返回一次）
#[derive(Debug, Clone, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub full_key: String,
    pub key_prefix: String,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}
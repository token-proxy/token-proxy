use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::application::dto::user_dto::{
    ChangePasswordRequest, CreateApiKeyRequest, CreateApiKeyResponse, UpdateProfileRequest,
    UserApiKeyResponse, UserResponse,
};
use crate::application::AppState;
use crate::presentation::middleware::jwt_auth::CurrentUser;
use crate::shared::error::AppError;

/// 构建当前用户相关路由
///
/// 所有路由均需经过 JWT 认证，使用 `CurrentUser` extractor 获取当前用户 ID。
///
/// - `GET    /api/users/me`                           -> get_my_profile
/// - `PUT    /api/users/me/profile`                   -> update_my_profile
/// - `PUT    /api/users/me/change-password`           -> change_my_password
/// - `GET    /api/users/me/api-keys`                  -> list_my_api_keys
/// - `POST   /api/users/me/api-keys`                  -> create_my_api_key
/// - `POST   /api/users/me/api-keys/{id}/revoke`      -> revoke_my_api_key
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/users/me", get(get_my_profile))
        .route("/api/users/me/profile", put(update_my_profile))
        .route("/api/users/me/change-password", put(change_my_password))
        .route("/api/users/me/api-keys", get(list_my_api_keys))
        .route("/api/users/me/api-keys", post(create_my_api_key))
        .route(
            "/api/users/me/api-keys/{id}/revoke",
            post(revoke_my_api_key),
        )
}

/// GET /api/users/me
///
/// 获取当前登录用户的 profile 信息。
async fn get_my_profile(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
) -> Result<Json<UserResponse>, AppError> {
    let user = state.user_service.get_by_id(user_id).await?;
    Ok(Json(user))
}

/// PUT /api/users/me/profile
///
/// 更新当前用户的 display_name。
///
/// 请求体:
/// ```json
/// { "display_name": "新显示名称" }
/// ```
async fn update_my_profile(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let user = state.user_service.update_profile(user_id, req).await?;
    Ok(Json(user))
}

/// PUT /api/users/me/change-password
///
/// 修改当前用户密码。需验证旧密码，新密码长度不能少于 6 位。
///
/// 请求体:
/// ```json
/// { "old_password": "旧密码", "new_password": "新密码" }
/// ```
async fn change_my_password(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.user_service.change_password(user_id, req).await?;
    Ok(Json(serde_json::json!({"message": "密码已修改"})))
}

/// GET /api/users/me/api-keys
///
/// 获取当前用户的所有 API key（脱敏，不返回完整 key）。
async fn list_my_api_keys(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
) -> Result<Json<Vec<UserApiKeyResponse>>, AppError> {
    let keys = state.user_api_key_service.list_by_user(user_id).await?;
    Ok(Json(keys))
}

/// POST /api/users/me/api-keys
///
/// 创建新的 API key。完整 key 仅在创建响应中返回一次。
///
/// 请求体:
/// ```json
/// { "description": "用途描述" }
/// ```
async fn create_my_api_key(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>, AppError> {
    let key = state
        .user_api_key_service
        .create(user_id, req.description)
        .await?;
    Ok(Json(key))
}

/// POST /api/users/me/api-keys/{id}/revoke
///
/// 撤销指定 API key。校验 key 属于当前用户。
async fn revoke_my_api_key(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.user_api_key_service.revoke(user_id, key_id).await?;
    Ok(Json(serde_json::json!({"message": "API key 已撤销"})))
}

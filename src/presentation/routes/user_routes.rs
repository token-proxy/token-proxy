use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::application::user::dto::{
    CreateUserRequest, UpdateUserRequest, UserApiKeyResponse, UserResponse,
};
use crate::application::AppState;
use crate::presentation::middleware::jwt_auth::CurrentUser;
use crate::shared::error::AppError;

/// 构建用户管理路由
///
/// - `GET    /api/users`                               → list_users
/// - `POST   /api/users`                               → create_user
/// - `GET    /api/users/{id}`                          → get_user
/// - `PUT    /api/users/{id}`                          → update_user
/// - `DELETE /api/users/{id}`                          → delete_user
/// - `GET    /api/users/{user_id}/api-keys`             → list_user_api_keys
/// - `DELETE /api/users/{user_id}/api-keys/{id}`       → revoke_user_api_key
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/users", get(list_users))
        .route("/api/users", post(create_user))
        .route("/api/users/{id}", get(get_user))
        .route("/api/users/{id}", put(update_user))
        .route("/api/users/{id}", delete(delete_user))
        .route("/api/users/{user_id}/api-keys", get(list_user_api_keys))
        .route(
            "/api/users/{user_id}/api-keys/{id}",
            delete(revoke_user_api_key),
        )
}

/// GET /api/users
///
/// 返回所有用户列表
async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<UserResponse>>, AppError> {
    let users = state.user_service.list_all().await?;
    Ok(Json(users))
}

/// POST /api/users
///
/// 创建新用户
async fn create_user(
    State(state): State<AppState>,
    CurrentUser(operator_id): CurrentUser,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let user = state.user_service.create(req, Some(operator_id)).await?;
    Ok(Json(user))
}

/// GET /api/users/{id}
///
/// 获取指定用户详情
async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    let user = state.user_service.get_by_id(id).await?;
    Ok(Json(user))
}

/// PUT /api/users/{id}
///
/// 更新指定用户
async fn update_user(
    State(state): State<AppState>,
    CurrentUser(operator_id): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let user = state
        .user_service
        .update(id, req, Some(operator_id))
        .await?;
    Ok(Json(user))
}

/// DELETE /api/users/{id}
///
/// 删除指定用户
async fn delete_user(
    State(state): State<AppState>,
    CurrentUser(operator_id): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.user_service.delete(id, Some(operator_id)).await?;
    Ok(Json(serde_json::json!({"message": "用户已删除"})))
}

/// GET /api/users/{user_id}/api-keys
///
/// 管理员查看指定用户的 API key 列表（脱敏）
async fn list_user_api_keys(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<UserApiKeyResponse>>, AppError> {
    let keys = state.user_api_key_service.list_by_user(user_id).await?;
    Ok(Json(keys))
}

/// DELETE /api/users/{user_id}/api-keys/{id}
///
/// 管理员吊销指定用户的 API key
async fn revoke_user_api_key(
    State(state): State<AppState>,
    CurrentUser(operator_id): CurrentUser,
    Path((_user_id, key_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .user_api_key_service
        .admin_revoke(key_id, operator_id)
        .await?;
    Ok(Json(serde_json::json!({"message": "API key 已撤销"})))
}

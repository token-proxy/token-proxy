use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::application::dto::user_dto::{CreateUserRequest, UpdateUserRequest, UserResponse};
use crate::application::AppState;
use crate::shared::error::AppError;

/// 构建用户管理路由
///
/// - `GET    /api/users`       → list_users
/// - `POST   /api/users`       → create_user
/// - `GET    /api/users/{id}`  → get_user
/// - `PUT    /api/users/{id}`  → update_user
/// - `DELETE /api/users/{id}`  → delete_user
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/users", get(list_users))
        .route("/api/users", post(create_user))
        .route("/api/users/{id}", get(get_user))
        .route("/api/users/{id}", put(update_user))
        .route("/api/users/{id}", delete(delete_user))
}

/// GET /api/users
async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<UserResponse>>, AppError> {
    let users = state.user_service.list_all().await?;
    Ok(Json(users))
}

/// POST /api/users
async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let user = state.user_service.create(req).await?;
    Ok(Json(user))
}

/// GET /api/users/{id}
async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    let user = state.user_service.get_by_id(id).await?;
    Ok(Json(user))
}

/// PUT /api/users/{id}
async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let user = state.user_service.update(id, req).await?;
    Ok(Json(user))
}

/// DELETE /api/users/{id}
async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.user_service.delete(id).await?;
    Ok(Json(serde_json::json!({"message": "用户已删除"})))
}

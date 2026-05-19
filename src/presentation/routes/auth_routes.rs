use axum::{extract::State, routing::post, Json, Router};
use serde_json::Value;

use crate::application::dto::auth_dto::{LoginRequest, LoginResponse, RefreshRequest};
use crate::application::AppState;
use crate::presentation::middleware::jwt_auth::CurrentUser;
use crate::shared::error::AppError;

/// 构建认证路由
///
/// - `POST /api/auth/login`   — 登录（公开）
/// - `POST /api/auth/refresh` — 刷新 token（公开）
/// - `POST /api/auth/logout`  — 登出（需认证）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
}

/// POST /api/auth/login
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let resp = state.auth_service.login(req).await?;
    Ok(Json(resp))
}

/// POST /api/auth/refresh
async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let resp = state.auth_service.refresh(req).await?;
    Ok(Json(resp))
}

/// POST /api/auth/logout（需要认证）
async fn logout(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
) -> Result<Json<Value>, AppError> {
    state.auth_service.logout(user_id).await?;
    Ok(Json(serde_json::json!({"message": "已登出"})))
}
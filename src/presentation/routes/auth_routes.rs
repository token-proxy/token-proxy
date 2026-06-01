use axum::{extract::State, routing::{delete, post}, Json, Router};
use serde_json::Value;

use crate::application::dto::auth_dto::{LoginRequest, LoginResponse, RefreshRequest};
use crate::application::AppState;
use crate::presentation::middleware::jwt_auth::CurrentUser;
use crate::shared::error::AppError;

/// 构建 Token 资源路由
///
/// - `POST   /api/tokens`          — 创建 Token（登录, 公开）
/// - `POST   /api/tokens:refresh`  — 刷新 Token（公开, 自定义方法）
/// - `DELETE /api/tokens/current`  — 删除当前 Token（登出, 需认证）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/tokens", post(login))
        .route("/api/tokens:refresh", post(refresh))
        .route("/api/tokens/current", delete(logout))
}

/// POST /api/tokens
///
/// 用户登录，创建 JWT 令牌对
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let resp = state.auth_service.login(req).await?;
    Ok(Json(resp))
}

/// POST /api/tokens:refresh
///
/// 刷新访问令牌（自定义方法）
async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let resp = state.auth_service.refresh(req).await?;
    Ok(Json(resp))
}

/// DELETE /api/tokens/current（需要认证）
///
/// 登出，删除当前用户的 refresh token
async fn logout(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
) -> Result<Json<Value>, AppError> {
    state.auth_service.logout(user_id).await?;
    Ok(Json(serde_json::json!({"message": "已登出"})))
}

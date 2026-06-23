//! 系统设置路由（展示层）

use axum::routing::get;
use axum::{extract::State, Json, Router};

use crate::application::system::dto::UpdateSettingsRequest;
use crate::application::AppState;
use crate::presentation::middleware::jwt_auth::CurrentUser;
use crate::shared::error::AppError;

/// 构建系统设置管理路由
///
/// - `GET    /api/settings`   → get_settings
/// - `PUT    /api/settings`   → update_settings（需要认证）
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/settings", get(get_settings).put(update_settings))
}

/// GET /api/settings
///
/// 获取当前系统设置
async fn get_settings(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let settings = state.settings_service.get_settings().await?;
    Ok(Json(serde_json::json!(settings)))
}

/// PUT /api/settings
///
/// 更新系统设置
async fn update_settings(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Json(input): Json<UpdateSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings = state
        .settings_service
        .update_settings(input, user_id)
        .await?;
    Ok(Json(serde_json::json!(settings)))
}

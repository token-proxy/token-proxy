//! 系统设置路由（展示层）

use axum::extract::State;
use axum::routing::{delete, get};
use axum::{Json, Router};

use crate::application::system::dto::UpdateSettingsRequest;
use crate::application::AppState;
use crate::presentation::middleware::jwt_auth::CurrentUser;
use crate::shared::error::AppError;

/// 构建系统设置管理路由
///
/// - `GET    /api/settings`                  → get_settings
/// - `PUT    /api/settings`                  → update_settings（需要认证）
/// - `GET    /api/settings/log-stats`        → get_log_stats（需要认证）
/// - `DELETE /api/settings/logs/{year_month}` → delete_month_logs（需要认证）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/settings", get(get_settings).put(update_settings))
        .route("/api/settings/log-stats", get(get_log_stats))
        .route("/api/settings/logs/{year_month}", delete(delete_month_logs))
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

/// GET /api/settings/log-stats
///
/// 获取日志分区统计信息，包括分区列表、月度汇总和总磁盘占用。
/// 仅管理 `log_metadata` 和 `log_contents` 的分区数据，不包含 `log_token_usage`。
async fn get_log_stats(
    State(state): State<AppState>,
    CurrentUser(_user_id): CurrentUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats = state.settings_service.get_log_stats().await?;
    Ok(Json(serde_json::json!(stats)))
}

/// DELETE /api/settings/logs/{year_month}
///
/// 删除指定月份的日志分区数据（log_metadata + log_contents）。
/// 需要 JWT 认证，当前月份不可删除。
async fn delete_month_logs(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    axum::extract::Path(year_month): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .settings_service
        .delete_month_logs(&year_month, user_id)
        .await?;
    Ok(Json(result))
}

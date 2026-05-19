use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use crate::application::dto::log_dto::{
    LogDetailResponse, LogFilterParams, LogSummaryResponse, SessionSummaryResponse,
};
use crate::application::AppState;
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

/// 构建日志查询路由
///
/// 注意路径注册顺序:
/// 1. `/api/logs/sessions`        (静态路径)
/// 2. `/api/logs/sessions/{id}`   (参数路径)
/// 3. `/api/logs/{id}`            (参数路径, 在 sessions 之后注册)
///
/// - `GET /api/logs`               → query_logs
/// - `GET /api/logs/sessions`      → get_sessions
/// - `GET /api/logs/sessions/{id}` → get_session_detail
/// - `GET /api/logs/{id}`          → get_log_detail
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/logs", get(query_logs))
        .route("/api/logs/sessions", get(get_sessions))
        .route("/api/logs/sessions/{id}", get(get_session_detail))
        .route("/api/logs/{id}", get(get_log_detail))
}

/// GET /api/logs
async fn query_logs(
    State(state): State<AppState>,
    Query(filters): Query<LogFilterParams>,
) -> Result<Json<PaginatedResult<LogSummaryResponse>>, AppError> {
    let logs = state.log_service.query_logs(filters).await?;
    Ok(Json(logs))
}

/// GET /api/logs/{id}
async fn get_log_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<LogDetailResponse>, AppError> {
    let detail = state
        .log_service
        .get_log_detail(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("日志 {} 未找到", id)))?;
    Ok(Json(detail))
}

/// GET /api/logs/sessions
async fn get_sessions(
    State(state): State<AppState>,
    Query(filters): Query<LogFilterParams>,
) -> Result<Json<PaginatedResult<SessionSummaryResponse>>, AppError> {
    let sessions = state.log_service.get_sessions(filters).await?;
    Ok(Json(sessions))
}

/// GET /api/logs/sessions/{id}
async fn get_session_detail(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<LogSummaryResponse>>, AppError> {
    let logs = state.log_service.get_session_detail(&session_id).await?;
    Ok(Json(logs))
}
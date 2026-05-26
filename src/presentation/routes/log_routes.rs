use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use crate::application::dto::log_dto::{
    LogDetailFullResponse, LogDetailResponse, LogFilterParams, LogSummaryResponse,
    SessionContentItemResponse, SessionSummaryResponse, TokenUsageResponse,
};
use crate::application::AppState;
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

/// 构建日志查询路由
///
/// - `GET /api/logs`                          → query_logs
/// - `GET /api/logs/sessions`                 → get_sessions
/// - `GET /api/logs/sessions/{id}/contents`   → get_session_contents
/// - `GET /api/logs/sessions/{id}/token-usage`→ get_session_token_usage
/// - `GET /api/logs/{id}/detail`              → get_log_detail_full
/// - `GET /api/logs/{id}/raw`                 → get_log_detail
/// - `GET /api/logs/{id}`                     → get_log_detail
/// - `GET /api/logs/{id}/token-usage`         → get_log_token_usage
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/logs", get(query_logs))
        .route("/api/logs/sessions", get(get_sessions))
        .route(
            "/api/logs/sessions/{id}/contents",
            get(get_session_contents),
        )
        .route(
            "/api/logs/sessions/{id}/token-usage",
            get(get_session_token_usage),
        )
        .route("/api/logs/{id}/detail", get(get_log_detail_full))
        .route("/api/logs/{id}/raw", get(get_log_detail))
        .route("/api/logs/{id}/token-usage", get(get_log_token_usage))
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

/// GET /api/logs/{id}/detail
async fn get_log_detail_full(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<LogDetailFullResponse>, AppError> {
    let detail = state
        .log_service
        .get_log_detail_full(id)
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

/// GET /api/logs/sessions/{id}/contents
async fn get_session_contents(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<SessionContentItemResponse>>, AppError> {
    let contents = state.log_service.get_session_contents(&session_id).await?;
    Ok(Json(contents))
}

async fn get_log_token_usage(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TokenUsageResponse>, AppError> {
    let usage = state
        .log_service
        .get_log_token_usage(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("日志 {} 的 token 用量未找到", id)))?;
    Ok(Json(usage))
}

async fn get_session_token_usage(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<TokenUsageResponse>>, AppError> {
    let usages = state
        .log_service
        .get_session_token_usage_response(&session_id)
        .await?;
    Ok(Json(usages))
}

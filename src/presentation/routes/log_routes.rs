use axum::{
    extract::{Path, Query, State},
    response::sse::{Event as SseEvent, KeepAlive, Sse},
    routing::get,
    Json, Router,
};
use futures::stream::Stream;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::application::log::dto::{
    LogDetailFullResponse, LogDetailResponse, LogFilterParams, LogSummaryResponse,
    SessionContentItemResponse, SessionSummaryResponse, TokenUsageResponse,
};
use crate::application::AppState;
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

/// 构建日志查询路由（受 JWT 中间件保护）
///
/// - `GET /api/logs`                          → query_logs
/// - `GET /api/logs/sessions`                 → get_sessions
/// - `GET /api/logs/sessions/{id}/contents`   → get_session_contents
/// - `GET /api/logs/sessions/{id}/token-usage`→ get_session_token_usage
/// - `GET /api/logs/{id}`                     → get_log_detail_full
/// - `GET /api/logs/{id}/raw`                 → get_log_detail
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
        .route("/api/logs/{id}", get(get_log_detail_full))
        .route("/api/logs/{id}/raw", get(get_log_detail))
        .route("/api/logs/{id}/token-usage", get(get_log_token_usage))
}

/// 构建 SSE 日志事件路由（自行认证，不走 JWT 中间件）
///
/// 因浏览器 `EventSource` API 不支持自定义 HTTP header，
/// JWT 通过 URL query 参数 `?token=<jwt>` 传递，handler 内部自行验证。
pub fn sse_routes() -> Router<AppState> {
    Router::new().route("/api/logs/events", get(log_events))
}

/// GET /api/logs
///
/// 分页查询代理请求日志
async fn query_logs(
    State(state): State<AppState>,
    Query(filters): Query<LogFilterParams>,
) -> Result<Json<PaginatedResult<LogSummaryResponse>>, AppError> {
    let logs = state.log_service.query_logs(filters).await?;
    Ok(Json(logs))
}

/// GET /api/logs/{id}
///
/// 获取日志完整详情（含客户端信息和 token 用量）
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

/// GET /api/logs/{id}/raw
///
/// 获取指定日志的原始内容
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
///
/// 分页查询会话列表
async fn get_sessions(
    State(state): State<AppState>,
    Query(filters): Query<LogFilterParams>,
) -> Result<Json<PaginatedResult<SessionSummaryResponse>>, AppError> {
    let sessions = state.log_service.get_sessions(filters).await?;
    Ok(Json(sessions))
}

/// GET /api/logs/sessions/{id}/contents
///
/// 获取会话的所有原始内容事件
async fn get_session_contents(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<SessionContentItemResponse>>, AppError> {
    let contents = state.log_service.get_session_contents(&session_id).await?;
    Ok(Json(contents))
}

/// GET /api/logs/{id}/token-usage
///
/// 获取指定日志的 token 用量
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

/// GET /api/logs/sessions/{id}/token-usage
///
/// 获取会话内所有日志的 token 用量汇总
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

/// GET /api/logs/events
///
/// SSE 端点，实时推送新日志写入事件。
/// 认证由 `sse_auth_middleware` 处理（从 URL query `?token=` 提取 JWT，
/// 因浏览器 EventSource API 不支持自定义 header）。
/// 客户端断开或收到关闭信号时自动结束。
async fn log_events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<SseEvent, std::convert::Infallible>>> {
    let mut rx = state.log_event_tx.subscribe();
    let mut shutdown_rx = state.shutdown_rx.clone();

    let stream = async_stream::stream! {
        // 立即发送初始事件，确保浏览器 EventSource 尽快触发 onopen
        // （部分浏览器需要收到第一块数据才认为连接就绪）
        yield Ok(SseEvent::default().event("connected").data("{}"));

        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            match serde_json::to_string(&event) {
                                Ok(json) => {
                                    yield Ok(SseEvent::default().data(json));
                                }
                                Err(_) => continue,
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!(skipped, "SSE 客户端消息滞后，已跳过");
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    tracing::debug!("SSE 连接因服务关闭而结束");
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}

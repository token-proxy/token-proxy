use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};

use crate::application::AppState;
use crate::shared::error::AppError;

use super::stats::dto::{
    OverviewResponse, TopAccessPointItem, TopModelItem, TopQuery, TrendItem, TrendsQuery,
};

/// 构建统计查询路由
///
/// - `GET /api/stats/overview`              → get_overview
/// - `GET /api/stats/trends`                → get_trends
/// - `GET /api/stats/top-access-points`     → get_top_access_points
/// - `GET /api/stats/top-models`            → get_top_models
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/stats/overview", get(get_overview))
        .route("/api/stats/trends", get(get_trends))
        .route("/api/stats/top-access-points", get(get_top_access_points))
        .route("/api/stats/top-models", get(get_top_models))
}

/// GET /api/stats/overview
///
/// 返回全局概览统计，委托给 `LogService::get_overview_stats()`。
async fn get_overview(State(state): State<AppState>) -> Result<Json<OverviewResponse>, AppError> {
    let (total_requests, active_access_points) = state.log_service.get_overview_stats().await?;

    Ok(Json(OverviewResponse {
        total_requests,
        active_access_points,
    }))
}

/// GET /api/stats/trends?days=7
///
/// 返回最近 N 天每日请求量趋势，委托给 `LogService::get_trends()`。
async fn get_trends(
    State(state): State<AppState>,
    Query(query): Query<TrendsQuery>,
) -> Result<Json<Vec<TrendItem>>, AppError> {
    let days = query.days.unwrap_or(7).min(365);

    let trends = state.log_service.get_trends(days).await?;

    let items: Vec<TrendItem> = trends
        .into_iter()
        .map(|(date, count)| TrendItem { date, count })
        .collect();

    Ok(Json(items))
}

/// GET /api/stats/top-access-points?limit=10
///
/// 返回请求量最高的接入点排名，委托给 `LogService::get_top_access_points()`。
async fn get_top_access_points(
    State(state): State<AppState>,
    Query(query): Query<TopQuery>,
) -> Result<Json<Vec<TopAccessPointItem>>, AppError> {
    let limit = query.limit.unwrap_or(10).min(100);

    let top = state.log_service.get_top_access_points(limit).await?;

    let items: Vec<TopAccessPointItem> = top
        .into_iter()
        .map(|(access_point_id, count)| TopAccessPointItem {
            access_point_id,
            count,
        })
        .collect();

    Ok(Json(items))
}

/// GET /api/stats/top-models?limit=10
///
/// 返回请求量最高的模型排名，委托给 `LogService::get_top_models()`。
async fn get_top_models(
    State(state): State<AppState>,
    Query(query): Query<TopQuery>,
) -> Result<Json<Vec<TopModelItem>>, AppError> {
    let limit = query.limit.unwrap_or(10).min(100);

    let top = state.log_service.get_top_models(limit).await?;

    let items: Vec<TopModelItem> = top
        .into_iter()
        .map(|(model, count)| TopModelItem { model, count })
        .collect();

    Ok(Json(items))
}

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::AppState;
use crate::shared::error::AppError;

// ─── Query DTOs ───

#[derive(Debug, Deserialize)]
pub struct TrendsQuery {
    /// 统计最近 N 天的趋势（默认 7 天，最大 365 天）
    pub days: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TopQuery {
    /// 返回 Top N 条记录（默认 10，最大 100）
    pub limit: Option<u64>,
}

// ─── Response DTOs ───

#[derive(Debug, Serialize)]
pub struct OverviewResponse {
    /// 请求总数
    pub total_requests: u64,
    /// 活跃接入点数量（有日志记录的接入点）
    pub active_access_points: u64,
}

#[derive(Debug, Serialize)]
pub struct TrendItem {
    /// 日期
    pub date: NaiveDate,
    /// 请求量
    pub count: u64,
}

#[derive(Debug, Serialize)]
pub struct TopAccessPointItem {
    /// 接入点 ID
    pub access_point_id: Uuid,
    /// 请求次数
    pub count: u64,
}

#[derive(Debug, Serialize)]
pub struct TopModelItem {
    /// 模型名称
    pub model: String,
    /// 请求次数
    pub count: u64,
}

/// 构建统计查询路由
///
/// - `GET /api/stats/overview`             → get_overview
/// - `GET /api/stats/trends`               → get_trends
/// - `GET /api/stats/top-access-points`    → get_top_access_points
/// - `GET /api/stats/top-models`           → get_top_models
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/stats/overview", get(get_overview))
        .route("/api/stats/trends", get(get_trends))
        .route("/api/stats/top-access-points", get(get_top_access_points))
        .route("/api/stats/top-models", get(get_top_models))
}

/// GET /api/stats/overview
///
/// 返回全局概览统计：请求总数、活跃接入点数量等。
async fn get_overview(
    State(state): State<AppState>,
) -> Result<Json<OverviewResponse>, AppError> {
    let total_requests = state.log_repo.count_total().await?;
    let active_access_points = state.log_repo.count_active_access_points().await?;

    Ok(Json(OverviewResponse {
        total_requests,
        active_access_points,
    }))
}

/// GET /api/stats/trends?days=7
///
/// 返回最近 N 天的每日请求量趋势。
async fn get_trends(
    State(state): State<AppState>,
    Query(query): Query<TrendsQuery>,
) -> Result<Json<Vec<TrendItem>>, AppError> {
    let days = query.days.unwrap_or(7).min(365);
    let end = Utc::now();
    let start = end - chrono::Duration::days(days as i64);

    let trends = state.log_repo.count_by_date_range(start, end).await?;

    let items: Vec<TrendItem> = trends
        .into_iter()
        .map(|(date, count)| TrendItem { date, count })
        .collect();

    Ok(Json(items))
}

/// GET /api/stats/top-access-points?limit=10
///
/// 返回请求量最高的 Top N 接入点排名。
async fn get_top_access_points(
    State(state): State<AppState>,
    Query(query): Query<TopQuery>,
) -> Result<Json<Vec<TopAccessPointItem>>, AppError> {
    let limit = query.limit.unwrap_or(10).min(100);

    let top = state.log_repo.top_access_points(limit).await?;

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
/// 返回请求量最高的 Top N 模型排名。
async fn get_top_models(
    State(state): State<AppState>,
    Query(query): Query<TopQuery>,
) -> Result<Json<Vec<TopModelItem>>, AppError> {
    let limit = query.limit.unwrap_or(10).min(100);

    let top = state.log_repo.top_models(limit).await?;

    let items: Vec<TopModelItem> = top
        .into_iter()
        .map(|(model, count)| TopModelItem { model, count })
        .collect();

    Ok(Json(items))
}

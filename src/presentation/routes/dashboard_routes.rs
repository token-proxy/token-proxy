//! Dashboard 路由（个人用量报告）。
//!
//! 所有端点均按 `CurrentUser` 提取 `user_id` 后传给 Service，
//! 数据按用户视角过滤。
//!
//! 端点：
//! - `GET /api/getting-started/heatmap?tz=Asia/Shanghai` 近 1 年用量热力图
//! - `GET /api/getting-started/kpi?start=...&end=...&tz=...` KPI 卡组（请求数 + 词元总量 + 构成 + 缓存命中率 + sparkline）
//! - `GET /api/getting-started/usage-trends?start=...&end=...&tz=...` 用户视角日级用量趋势
//! - `GET /api/getting-started/top-models?start=...&end=...` 用户视角模型排行 Top 8
//! - `GET /api/getting-started/top-access-points?start=...&end=...` 用户视角接入点排行 Top 5
//! - `GET /api/getting-started/quality?start=...&end=...` 调用质量指标
//!
//! 认证：本模块仅声明路由结构与 handler；JWT 认证 layer 由
//! `presentation::routes::mod` 的 `jwt_protected` 分组统一加挂。

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};

use crate::application::dashboard::dto::{
    HeatmapQuery, HeatmapResponse, KpiResponse, QualityResponse, TimeRangeParams,
    TopAccessPointsResponse, TopModelsResponse, UsageTrendsResponse,
};
use crate::application::AppState;
use crate::presentation::middleware::jwt_auth::CurrentUser;
use crate::shared::error::AppError;

/// 构建 Dashboard 路由
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/getting-started/heatmap", get(get_heatmap))
        .route("/api/getting-started/kpi", get(get_kpi))
        .route("/api/getting-started/usage-trends", get(get_usage_trends))
        .route("/api/getting-started/top-models", get(get_top_models))
        .route(
            "/api/getting-started/top-access-points",
            get(get_top_access_points),
        )
        .route("/api/getting-started/quality", get(get_quality))
}

async fn get_heatmap(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Query(q): Query<HeatmapQuery>,
) -> Result<Json<HeatmapResponse>, AppError> {
    let resp = state.dashboard_service.get_heatmap(user_id, &q.tz).await?;
    Ok(Json(resp))
}

async fn get_kpi(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Query(params): Query<TimeRangeParams>,
) -> Result<Json<KpiResponse>, AppError> {
    let resp = state.dashboard_service.get_kpi(user_id, params).await?;
    Ok(Json(resp))
}

async fn get_usage_trends(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Query(params): Query<TimeRangeParams>,
) -> Result<Json<UsageTrendsResponse>, AppError> {
    let resp = state
        .dashboard_service
        .get_usage_trends(user_id, params)
        .await?;
    Ok(Json(resp))
}

async fn get_top_models(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Query(params): Query<TimeRangeParams>,
) -> Result<Json<TopModelsResponse>, AppError> {
    let resp = state
        .dashboard_service
        .get_top_models(user_id, params)
        .await?;
    Ok(Json(resp))
}

async fn get_top_access_points(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Query(params): Query<TimeRangeParams>,
) -> Result<Json<TopAccessPointsResponse>, AppError> {
    let resp = state
        .dashboard_service
        .get_top_access_points(user_id, params)
        .await?;
    Ok(Json(resp))
}

async fn get_quality(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Query(params): Query<TimeRangeParams>,
) -> Result<Json<QualityResponse>, AppError> {
    let resp = state.dashboard_service.get_quality(user_id, params).await?;
    Ok(Json(resp))
}

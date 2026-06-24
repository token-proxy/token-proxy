//! Dashboard 路由：面向技术主管的数据洞察 API
//!
//! 端点：
//! - `GET /api/dashboard/kpi` —— 4 张 KPI 卡 + sparkline 序列
//! - `GET /api/dashboard/top-users` —— 成员请求量排行 Top 10
//! - `GET /api/dashboard/top-accounts` —— 上游账号词元消耗排行 Top 10
//!
//! 所有端点支持统一 query 参数：`?range=today|last7|last30|custom[&start=ISO][&end=ISO]`
//!
//! 认证：本模块仅声明路由结构与 handler；JWT 认证 layer 由
//! `presentation::routes::mod` 的 `jwt_protected` 分组统一加挂。

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};

use crate::application::dashboard::dto::{
    KpiResponse, TimeRangeQuery, TopAccountsResponse, TopUsersResponse,
};
use crate::application::AppState;
use crate::shared::error::AppError;

/// 构建 Dashboard 路由
///
/// - `GET /api/dashboard/kpi`           → [`get_kpi`]
/// - `GET /api/dashboard/top-users`     → [`get_top_users`]
/// - `GET /api/dashboard/top-accounts`  → [`get_top_accounts`]
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/dashboard/kpi", get(get_kpi))
        .route("/api/dashboard/top-users", get(get_top_users))
        .route("/api/dashboard/top-accounts", get(get_top_accounts))
}

/// GET /api/dashboard/kpi
///
/// 返回 4 张 KPI 卡（请求数 / 词元量 / 活跃成员数 / 缓存命中率）+ 内嵌 sparkline 序列。
/// 委托给 [`DashboardService::get_kpi`](crate::application::dashboard::DashboardService::get_kpi)。
#[tracing::instrument(skip_all, fields(range = ?query.range))]
async fn get_kpi(
    State(state): State<AppState>,
    Query(query): Query<TimeRangeQuery>,
) -> Result<Json<KpiResponse>, AppError> {
    let resp = state.dashboard_service.get_kpi(query).await?;
    Ok(Json(resp))
}

/// GET /api/dashboard/top-users
///
/// 返回成员请求量排行 Top 10。
/// 委托给 [`DashboardService::get_top_users`](crate::application::dashboard::DashboardService::get_top_users)。
#[tracing::instrument(skip_all, fields(range = ?query.range))]
async fn get_top_users(
    State(state): State<AppState>,
    Query(query): Query<TimeRangeQuery>,
) -> Result<Json<TopUsersResponse>, AppError> {
    let resp = state.dashboard_service.get_top_users(query).await?;
    Ok(Json(resp))
}

/// GET /api/dashboard/top-accounts
///
/// 返回上游账号词元消耗排行 Top 10。
/// 委托给 [`DashboardService::get_top_accounts`](crate::application::dashboard::DashboardService::get_top_accounts)。
#[tracing::instrument(skip_all, fields(range = ?query.range))]
async fn get_top_accounts(
    State(state): State<AppState>,
    Query(query): Query<TimeRangeQuery>,
) -> Result<Json<TopAccountsResponse>, AppError> {
    let resp = state.dashboard_service.get_top_accounts(query).await?;
    Ok(Json(resp))
}

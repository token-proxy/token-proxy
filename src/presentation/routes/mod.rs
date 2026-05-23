use axum::{middleware, routing::get, Json, Router};

use crate::application::AppState;
use crate::presentation::middleware::jwt_auth;

pub mod access_point_routes;
pub mod account_routes;
pub mod auth_routes;
pub mod log_routes;
pub mod me_routes;
pub mod provider_routes;
pub mod proxy_routes;
pub mod stats_routes;
pub mod user_routes;

/// 构建应用所有路由并注入共享状态
///
/// 聚合所有子路由模块，添加全局中间件（JWT 认证、CORS、Tracing），
/// 最后调用 `.with_state(state)` 绑定 `AppState`。
///
/// 认证中间件会通过 `PUBLIC_PATHS` 和 `/ap/*` 前缀自动跳过公开路径。
pub fn build(state: AppState) -> Router {
    let app = Router::new()
        .merge(auth_routes::routes())
        .merge(provider_routes::routes())
        .merge(account_routes::routes())
        .merge(user_routes::routes())
        .merge(me_routes::routes())
        .merge(access_point_routes::routes())
        .merge(proxy_routes::routes())
        .merge(log_routes::routes())
        .merge(stats_routes::routes())
        .route("/api/health", get(health_check));

    // 添加 JWT 认证中间件（公开路径会在中间件内部放行）
    app.layer(middleware::from_fn_with_state(
        state.clone(),
        jwt_auth::auth_middleware,
    ))
    .with_state(state)
}

/// GET /api/health
///
/// 健康检查端点（公开路径）
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}
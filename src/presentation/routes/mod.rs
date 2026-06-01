use axum::{middleware, routing::get, Json, Router};

use crate::application::AppState;
use crate::presentation::middleware::{jwt_auth, user_api_key_auth};

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
/// 路由分为三组，各自应用不同的认证中间件：
/// - 公开路由：无需认证
/// - JWT 保护路由：需 JWT 认证
/// - 代理路由：需用户 API key 认证
pub fn build(state: AppState) -> Router {
    // 公开路由 — 不应用任何认证中间件
    let public = Router::new()
        .merge(auth_routes::public_routes())
        .route("/api/health", get(health_check));

    // JWT 保护的路由
    let jwt_protected = Router::new()
        .merge(auth_routes::protected_routes())
        .merge(provider_routes::routes())
        .merge(account_routes::routes())
        .merge(user_routes::routes())
        .merge(me_routes::routes())
        .merge(access_point_routes::routes())
        .merge(log_routes::routes())
        .merge(stats_routes::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            jwt_auth::auth_middleware,
        ));

    // API key 保护的路由（代理转发）
    let proxy = Router::new()
        .merge(proxy_routes::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            user_api_key_auth::middleware,
        ));

    Router::new()
        .merge(public)
        .merge(jwt_protected)
        .merge(proxy)
        .with_state(state)
}

/// GET /api/health
///
/// 健康检查端点（公开）
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

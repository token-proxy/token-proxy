use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
    routing::post,
    Router,
};

use crate::application::AppState;
use crate::presentation::middleware::user_api_key_auth::ApiKeyUser;
use crate::shared::error::AppError;

/// 构建代理转发路由（API key 认证中间件在路由分组层应用）
///
/// - `POST /ap/{short_code}/{*remainder}` → proxy_messages
///   remainder 捕获所有后续路径段（如 v1/messages），透明透传到上游
pub fn routes() -> Router<AppState> {
    Router::new().route("/ap/{short_code}/{*remainder}", post(proxy_messages))
}

/// POST /ap/{short_code}/{*remainder}
///
/// 核心代理转发入口。认证由 API key 中间件完成，
/// 转发编排由 ProxyPipeline 负责。
async fn proxy_messages(
    State(state): State<AppState>,
    Path((short_code, remainder)): Path<(String, String)>,
    ApiKeyUser(user_id): ApiKeyUser,
    headers: HeaderMap,
    body: String,
) -> Result<Response, AppError> {
    state
        .proxy_pipeline
        .execute(&short_code, &remainder, headers, body, user_id)
        .await
}

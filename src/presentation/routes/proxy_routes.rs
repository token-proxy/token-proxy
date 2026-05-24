use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
    routing::post,
    Router,
};
use bytes::Bytes;
use futures::StreamExt;
use serde_json::{Map, Value};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::application::services::proxy_service::ProxyContext;
use crate::application::AppState;
use crate::domain::entities::log_entry::LogEntry;
use crate::domain::services::model_mapping_service::{find_matching_mapping, resolve_final_model};
use crate::domain::value_objects::model_mapping::ModelMapping;
use crate::shared::error::AppError;

/// 构建代理转发路由（公开路径，不经过 JWT 中间件）
///
/// - `POST /ap/{short_code}/v1/messages` → proxy_messages
pub fn routes() -> Router<AppState> {
    Router::new().route("/ap/{short_code}/v1/messages", post(proxy_messages))
}

/// POST /ap/{short_code}/v1/messages
///
/// 核心代理转发入口。根据 short_code 解析代理上下文后，
/// 根据请求类型（流式/非流式）分别处理转发逻辑。
async fn proxy_messages(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    headers: HeaderMap,
    body: String,
) -> Result<Response, AppError> {
    // 0. 认证用户 API key（必须携带有效的 Authorization: Bearer <user_api_key>）
    let user_id = state.proxy_service.authenticate_request(&headers).await?;

    // 1. 解析代理上下文（验证 AP/Provider/Account 状态）
    let ctx = state.proxy_service.resolve_context(&short_code).await?;

    // 2. 提取 session_id
    let session_id = headers
        .get("x-claude-code-session-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // 3. 提取请求中的原始 model 名称
    let model_original = extract_model_from_body(&body);

    // 4. 应用统一模型匹配逻辑
    //    优先级：精确匹配 > 前缀匹配 > __unmatched__ 规则 > Provider.default_model > 原始模型
    let requested_model = model_original.as_deref().unwrap_or("");
    let mapped_model = if !requested_model.is_empty() {
        find_matching_mapping(&ctx.access_point.model_mappings, requested_model)
            .map(|m| m.target_model.clone())
    } else {
        None
    };
    let final_model = resolve_final_model(
        mapped_model.clone(),
        ctx.provider.default_model.as_deref(),
        requested_model,
    );

    // 5. 根据映射结果替换请求体中的 model 字段
    let modified_body = if mapped_model.is_some() {
        // 找到匹配映射，应用最终解析后的模型到请求体
        let mapping = ModelMapping {
            source_model: requested_model.to_string(),
            target_model: final_model.clone(),
            match_type: Default::default(),
        };
        let (new_body, _delta) = mapping.apply_to_body(&body);
        new_body
    } else if !requested_model.is_empty() && final_model != requested_model {
        // 使用 Provider.default_model 兜底，替换请求体
        let mapping = ModelMapping {
            source_model: requested_model.to_string(),
            target_model: final_model.clone(),
            match_type: Default::default(),
        };
        let (new_body, _delta) = mapping.apply_to_body(&body);
        new_body
    } else {
        body.clone()
    };

    // 6. 判断是否流式请求
    let is_streaming = modified_body.contains("\"stream\":true")
        || headers
            .get("accept")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.contains("text/event-stream"))
            .unwrap_or(false);

    // 7. 构造上游 URL
    let base_url = ctx
        .provider
        .anthropic_base_url
        .as_deref()
        .ok_or_else(|| AppError::Internal("提供商未配置 Anthropic 基础 URL".to_string()))?;
    let upstream_url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

    if is_streaming {
        handle_streaming_proxy(
            state,
            ctx,
            upstream_url,
            modified_body,
            headers,
            session_id,
            model_original,
            final_model,
            user_id,
        )
        .await
    } else {
        handle_non_streaming_proxy(
            state,
            ctx,
            upstream_url,
            modified_body,
            headers,
            session_id,
            model_original,
            final_model,
            user_id,
        )
        .await
    }
}

/// 非流式转发
#[allow(clippy::too_many_arguments)]
async fn handle_non_streaming_proxy(
    state: AppState,
    ctx: ProxyContext,
    upstream_url: String,
    body: String,
    headers: HeaderMap,
    session_id: String,
    model_original: Option<String>,
    model_mapped: String,
    user_id: Uuid,
) -> Result<Response, AppError> {
    let start = Instant::now();
    let body_bytes = Bytes::from(body.clone());
    let request_headers = headers_to_json(&headers);

    // 转发请求到上游
    let (status, resp_headers, resp_body) = state
        .proxy_client
        .forward(&upstream_url, &ctx.decrypted_api_key, body_bytes, headers)
        .await?;

    let duration = start.elapsed();

    // 异步记录日志（不阻塞响应）
    let log_service = state.log_service.clone();
    let resp_body_clone = resp_body.clone();

    tokio::spawn(async move {
        let log_entry = LogEntry {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            session_id,
            user_id: Some(user_id),
            access_point_id: Some(ctx.access_point.id),
            provider_id: Some(ctx.provider.id),
            account_id: Some(ctx.account.id),
            model_original,
            model_mapped: Some(model_mapped),
            status_code: Some(status.as_u16() as i16),
            duration_ms: Some(duration.as_millis() as i32),
            error_message: None,
            ..Default::default()
        };

        log_service
            .record_proxy_log(
                log_entry,
                request_headers,
                serde_json::from_str(&body).unwrap_or_default(),
                String::from_utf8_lossy(&resp_body_clone).to_string(),
            )
            .await
            .ok();
    });

    // 构建 HTTP 响应
    let mut response_builder = Response::builder().status(status);
    for (key, value) in resp_headers.iter() {
        let key_str = key.as_str().to_lowercase();
        if key_str != "transfer-encoding" {
            response_builder = response_builder.header(key, value.clone());
        }
    }

    let response = response_builder
        .body(axum::body::Body::from(resp_body))
        .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)))?;

    Ok(response)
}

/// SSE 流式转发
#[allow(clippy::too_many_arguments)]
async fn handle_streaming_proxy(
    state: AppState,
    ctx: ProxyContext,
    upstream_url: String,
    body: String,
    headers: HeaderMap,
    session_id: String,
    model_original: Option<String>,
    model_mapped: String,
    user_id: Uuid,
) -> Result<Response, AppError> {
    let body_bytes = Bytes::from(body.clone());
    let start = Instant::now();
    let request_headers = headers_to_json(&headers);

    // 获取上游流式响应
    let upstream_resp = state
        .proxy_client
        .forward_streaming(&upstream_url, &ctx.decrypted_api_key, body_bytes, headers)
        .await?;

    let status = upstream_resp.status();
    let resp_headers = upstream_resp.headers().clone();

    let log_service = state.log_service.clone();
    let log_buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let log_buffer_clone = log_buffer.clone();

    // 消费上游响应的字节流，构建转发流
    let byte_stream = upstream_resp.bytes_stream();

    let stream = async_stream::stream! {
        tokio::pin!(byte_stream);

        while let Some(chunk) = byte_stream.next().await {
            match chunk {
                Ok(bytes) => {
                    // 拼接到日志缓冲区
                    let text = String::from_utf8_lossy(&bytes).to_string();
                    log_buffer_clone.lock().await.push_str(&text);
                    yield Ok::<Bytes, axum::Error>(bytes);
                }
                Err(e) => {
                    yield Err(axum::Error::new(e));
                }
            }
        }

        // 流结束后异步写日志
        let buf = log_buffer_clone.lock().await.clone();
        if !buf.is_empty() {
            let elapsed = start.elapsed();
            let log_svc = log_service.clone();
            let ap_id = ctx.access_point.id;
            let provider_id = ctx.provider.id;
            let account_id = ctx.account.id;
            let sess = session_id;
            let model_orig = model_original;
            let model_mapped_val = Some(model_mapped);
            let req_body = body;
            let req_headers = request_headers;
            let uid = user_id;

            tokio::spawn(async move {
                let log_entry = LogEntry {
                    id: Uuid::new_v4(),
                    timestamp: chrono::Utc::now(),
                    session_id: sess,
                    user_id: Some(uid),
                    access_point_id: Some(ap_id),
                    provider_id: Some(provider_id),
                    account_id: Some(account_id),
                    model_original: model_orig,
                    model_mapped: model_mapped_val,
                    status_code: Some(200),
                    duration_ms: Some(elapsed.as_millis() as i32),
                    error_message: None,
                    ..Default::default()
                };

                log_svc
                    .record_proxy_log(
                        log_entry,
                        req_headers,
                        serde_json::from_str(&req_body).unwrap_or_default(),
                        buf,
                    )
                    .await
                    .ok();
            });
        }
    };

    // 构建流式响应
    let mut response_builder = Response::builder()
        .status(status)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("connection", "keep-alive");

    for (key, value) in resp_headers.iter() {
        let key_str = key.as_str().to_lowercase();
        if key_str != "transfer-encoding"
            && key_str != "content-type"
            && key_str != "content-length"
        {
            response_builder = response_builder.header(key, value.clone());
        }
    }

    let response = response_builder
        .body(axum::body::Body::from_stream(stream))
        .map_err(|e| AppError::Internal(format!("构建流式响应失败: {}", e)))?;

    Ok(response)
}

/// 从请求体 JSON 中提取 model 字段值
fn extract_model_from_body(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body).ok().and_then(|v| {
        v.get("model")
            .and_then(|m| m.as_str().map(|s| s.to_string()))
    })
}

fn headers_to_json(headers: &HeaderMap) -> Value {
    let mut map = Map::new();

    for (key, value) in headers {
        let name = key.as_str();
        let header_value = if is_sensitive_header(name) {
            Value::String("[REDACTED]".to_string())
        } else {
            Value::String(value.to_str().unwrap_or("[non-UTF8]").to_string())
        };
        map.insert(name.to_string(), header_value);
    }

    Value::Object(map)
}

fn is_sensitive_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization")
        || name.eq_ignore_ascii_case("x-api-key")
        || name.eq_ignore_ascii_case("proxy-authorization")
        || name.eq_ignore_ascii_case("cookie")
        || name.eq_ignore_ascii_case("set-cookie")
}

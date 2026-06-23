//! 上游转发执行器 — application/proxy/
//!
//! 封装 `ProxyClient::forward` 调用 + 120s 非流式响应体读取超时，
//! 把过去 `proxy_pipeline.rs` 中分散在三条响应分支的 timeout/读取逻辑归一。

use std::sync::Arc;
use std::time::Duration;

use axum::http::HeaderMap;
use bytes::Bytes;

use crate::domain::shared::UpstreamRequest;
use crate::infrastructure::http_client::ProxyClient;
use crate::shared::error::AppError;

/// 非流式响应体读取超时（避免上游永远不结束导致请求挂住）
const BUFFERED_BODY_TIMEOUT_SECS: u64 = 120;

/// 上游转发执行器
pub struct UpstreamDispatcher {
    client: Arc<ProxyClient>,
}

impl UpstreamDispatcher {
    pub fn new(client: Arc<ProxyClient>) -> Self {
        UpstreamDispatcher { client }
    }

    /// 转发上游请求并返回原始响应
    ///
    /// 调用方根据 `response.status()` 和 `response.headers()` 判断后续如何消费响应体
    /// （流式 `bytes_stream()` 或非流式 `read_body_with_timeout()`）。
    pub async fn forward(&self, upstream: &UpstreamRequest) -> Result<reqwest::Response, AppError> {
        let body_bytes = Bytes::from(serde_json::to_string(&upstream.body).unwrap_or_default());
        self.client
            .forward(&upstream.url, upstream.headers.clone(), body_bytes)
            .await
    }

    /// 读取非流式响应体（含 120s 超时保护）
    ///
    /// 与 `forward` 配套使用：判定为非 SSE 响应后调用此方法将完整 body 加载到内存。
    pub async fn read_buffered_body(resp: reqwest::Response) -> Result<Bytes, AppError> {
        tokio::time::timeout(
            Duration::from_secs(BUFFERED_BODY_TIMEOUT_SECS),
            resp.bytes(),
        )
        .await
        .map_err(|_| AppError::Upstream("读取上游响应超时".to_string()))?
        .map_err(|e| AppError::Upstream(format!("读取上游响应失败: {}", e)))
    }
}

/// 从 reqwest 响应头复制 hop-by-hop 之外的所有头到 axum response builder
///
/// 与 `UpstreamDispatcher` 配套使用，由 `ResponseBuilder` 调用，但放在此处只为同一文件内聚。
pub fn copy_passthrough_headers(
    resp_headers: &HeaderMap,
    builder: axum::http::response::Builder,
) -> axum::http::response::Builder {
    use crate::domain::shared::HOP_BY_HOP_HEADERS;
    let mut builder = builder;
    for (key, value) in resp_headers.iter() {
        let key_lower = key.as_str().to_lowercase();
        if !HOP_BY_HOP_HEADERS.contains(&key_lower.as_str()) {
            builder = builder.header(key, value.clone());
        }
    }
    builder
}

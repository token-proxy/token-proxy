//! axum 响应构造器 — application/proxy/
//!
//! 把 `axum::response::Response` 的构造从 `ProxyPipeline::execute` 中抽出，
//! 统一两种响应形态（streaming / buffered），同时承担 hop-by-hop 头过滤。
//!
//! `ProxyCallRecord` 的生命周期管理也在这里收敛：
//! - streaming：`record` move 进 `async_stream` 内部，流末尾 `finish`，
//!   Drop 兜底处理客户端中断
//! - buffered：record.set_body + finish 同步完成

use axum::http::{HeaderMap, StatusCode};
use bytes::Bytes;
use futures::StreamExt;

use crate::shared::error::AppError;

use super::proxy_call_record::ProxyCallRecord;
use super::upstream_dispatcher::copy_passthrough_headers;

/// 构造流式（SSE）响应
///
/// `record` 被 move 进异步生成器，每个 chunk 通过 `append_body` 累积到日志，
/// 流结束时 `finish()`；若客户端中断（stream 被 axum drop），
/// `record` 的 `Drop` 兜底标记 `is_interrupted` 并落库。
pub fn build_streaming_response(
    status: StatusCode,
    resp_headers: &HeaderMap,
    upstream_resp: reqwest::Response,
    record: ProxyCallRecord,
) -> Result<axum::response::Response, AppError> {
    let byte_stream = upstream_resp.bytes_stream();
    let stream = async_stream::stream! {
        tokio::pin!(byte_stream);
        let mut record = record;
        while let Some(chunk) = byte_stream.next().await {
            match chunk {
                Ok(bytes) => {
                    record.append_body(&bytes);
                    yield Ok(bytes);
                }
                Err(e) => {
                    yield Err(axum::Error::new(e));
                }
            }
        }
        record.finish();
    };

    let builder = copy_passthrough_headers(
        resp_headers,
        axum::response::Response::builder().status(status),
    );
    builder
        .body(axum::body::Body::from_stream(stream))
        .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)))
}

/// 构造非流式（buffered）响应
///
/// 直接把已读完的 body 交给 axum；同步 `record.set_body + finish`。
pub fn build_buffered_response(
    status: StatusCode,
    resp_headers: &HeaderMap,
    body: Bytes,
    mut record: ProxyCallRecord,
) -> Result<axum::response::Response, AppError> {
    record.set_body(&body);
    record.finish();

    let builder = copy_passthrough_headers(
        resp_headers,
        axum::response::Response::builder().status(status),
    );
    builder
        .body(axum::body::Body::from(body))
        .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)))
}

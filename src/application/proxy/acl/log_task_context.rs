use std::sync::Arc;

use axum::http::HeaderMap;

use crate::application::log::LogService;
use crate::domain::log::LogEntry;

pub struct LogTaskContext {
    pub(crate) log_service: Arc<LogService>,
    pub(crate) entry: LogEntry,
    pub(crate) request_headers: HeaderMap,
    pub(crate) request_body: serde_json::Value,
    pub(crate) response_text: String,
    pub(crate) resp_headers: HeaderMap,
}

/// 异步写日志
pub fn spawn_log_task(ctx: LogTaskContext) {
    tokio::spawn(async move {
        if let Err(e) = ctx
            .log_service
            .record_proxy_log(
                ctx.entry,
                &ctx.request_headers,
                ctx.request_body,
                ctx.response_text,
                ctx.resp_headers,
            )
            .await
        {
            tracing::error!(error = %e, "代理日志写入失败");
        }
    });
}

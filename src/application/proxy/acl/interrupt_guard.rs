use std::sync::Arc;
use std::time::Instant;

use axum::http::HeaderMap;

use crate::application::log::LogService;

use super::log_context::LogContext;

/// 客户端断开检测守卫
///
/// 当 generator 被提前 drop（客户端断开连接）时，
/// Drop 中写入一条标记了 is_interrupted 的部分日志。
pub struct InterruptGuard {
    pub(crate) completed: bool,
    pub(crate) log_service: Arc<LogService>,
    pub(crate) log_ctx: LogContext,
    pub(crate) status_code: u16,
    pub(crate) start: Instant,
    pub(crate) buffer: Arc<std::sync::Mutex<String>>,
    pub(crate) resp_headers: HeaderMap,
    pub(crate) runtime: tokio::runtime::Handle,
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        if self.completed {
            return;
        }

        let buf = self.buffer.lock().unwrap().clone();
        let elapsed = self.start.elapsed();
        let mut entry = self.log_ctx.build_log_entry();
        entry.is_interrupted = true;
        entry.status_code = Some(self.status_code as i16);
        entry.duration_ms = Some(elapsed.as_millis() as i32);
        entry.error_message = Some("客户端断开连接".to_string());

        let log_service = self.log_service.clone();
        let request_headers = self.log_ctx.request_headers.clone();
        let request_body = self.log_ctx.request_body.clone();
        let resp_headers = self.resp_headers.clone();

        self.runtime.spawn(async move {
            if let Err(e) = log_service
                .record_proxy_log(entry, &request_headers, request_body, buf, resp_headers)
                .await
            {
                tracing::error!(error = %e, "中断日志写入失败");
            }
        });
    }
}

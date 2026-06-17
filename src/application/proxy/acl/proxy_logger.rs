use std::sync::Arc;
use std::time::Instant;

use axum::http::HeaderMap;
use bytes::Bytes;
use tokio::runtime::Handle;

use crate::application::log::LogService;
use crate::domain::log::LogEntry;

use super::log_context::LogContext;

/// 代理日志积累器
///
/// 贯穿一次代理转发的完整生命周期，有数据就推入：
/// 请求头体（构造时）→ 响应头（构造时）→ body 片段（逐段追加）。
///
/// 生命周期结束时（正常完成或客户端断开），统一 flush 到数据库。
/// 不区分"正常日志"和"中断日志"——中断只是 Drop 时多打一个 is_interrupted 标记。
pub struct ProxyLogger {
    log_service: Arc<LogService>,
    entry: LogEntry,
    request_headers: HeaderMap,
    request_body: serde_json::Value,
    resp_headers: HeaderMap,
    response_body: String,
    start: Instant,
    is_interrupted: bool,
    runtime: Handle,
    flushed: bool,
}

impl ProxyLogger {
    /// 构造 Logger，从 LogContext 提取请求侧信息，注入响应侧元数据
    pub fn new(
        log_ctx: LogContext,
        status_code: u16,
        start: Instant,
        resp_headers: HeaderMap,
        log_service: Arc<LogService>,
        runtime: Handle,
    ) -> Self {
        let mut entry = log_ctx.build_log_entry();
        entry.status_code = Some(status_code as i16);

        ProxyLogger {
            log_service,
            entry,
            request_headers: log_ctx.request_headers,
            request_body: log_ctx.request_body,
            resp_headers,
            response_body: String::new(),
            start,
            is_interrupted: false,
            runtime,
            flushed: false,
        }
    }

    /// 流式路径：逐段追加响应体
    pub fn append_body(&mut self, bytes: &Bytes) {
        self.response_body.push_str(&String::from_utf8_lossy(bytes));
    }

    /// 非流式路径：一次性设置响应体
    pub fn set_body(&mut self, bytes: &Bytes) {
        self.response_body = String::from_utf8_lossy(bytes).to_string();
    }

    /// 正常完成时调用：计算耗时，spawn 异步写入数据库
    pub fn flush(&mut self) {
        if self.flushed {
            return;
        }
        self.flushed = true;

        self.entry.duration_ms = Some(self.start.elapsed().as_millis() as i32);
        if self.is_interrupted {
            self.entry.is_interrupted = true;
            self.entry.error_message = Some("客户端断开连接".to_string());
        }

        let log_service = self.log_service.clone();
        let entry = self.entry.clone();
        let request_headers = self.request_headers.clone();
        let request_body = self.request_body.clone();
        let response_body = std::mem::take(&mut self.response_body);
        let resp_headers = self.resp_headers.clone();

        self.runtime.spawn(async move {
            if let Err(e) = log_service
                .record_proxy_log(
                    entry,
                    &request_headers,
                    request_body,
                    response_body,
                    resp_headers,
                )
                .await
            {
                tracing::error!(error = %e, "代理日志写入失败");
            }
        });
    }
}

impl Drop for ProxyLogger {
    fn drop(&mut self) {
        if !self.flushed {
            self.is_interrupted = true;
            self.flush();
        }
    }
}

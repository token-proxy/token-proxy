use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use uuid::Uuid;

use crate::application::log::service::LogService;
use crate::domain::access_point::AccessPointEx;
use crate::domain::log::LogEntry;
use crate::infrastructure::http_client::request_transform::ProcessedRequest;

/// 防腐层：封装 ProcessedRequest + AccessPointEx → 日志参数的转换
///
/// 从领域对象中一次性提取日志记录所需的所有字段，
/// 隔离代理转发逻辑与日志格式细节。
#[derive(Clone)]
pub struct LogContext {
    user_id: Uuid,
    access_point_id: Uuid,
    provider_id: Uuid,
    account_id: Uuid,
    session_id: String,
    original_model: String,
    mapped_model: String,
    api_type: String,
    request_headers: HeaderMap,
    request_body: serde_json::Value,
}

impl LogContext {
    /// 唯一构造入口：从领域对象提取所有日志参数
    pub fn from_request(
        processed: &ProcessedRequest,
        access_point: &AccessPointEx,
        user_id: Uuid,
    ) -> Self {
        LogContext {
            user_id,
            access_point_id: access_point.id,
            provider_id: access_point.provider_id,
            account_id: access_point.account_id,
            session_id: processed.session_id.clone(),
            original_model: processed.inbound.model().to_string(),
            mapped_model: processed.outbound.model().to_string(),
            api_type: access_point.api_type.to_string(),
            request_headers: processed.inbound.headers().clone(),
            request_body: processed.inbound.body().clone(),
        }
    }

    /// 构造 LogEntry（不含 status_code / duration_ms，由调用方补充）
    pub fn build_log_entry(&self) -> LogEntry {
        LogEntry {
            id: Uuid::new_v4(),
            session_id: self.session_id.clone(),
            user_id: Some(self.user_id),
            access_point_id: Some(self.access_point_id),
            provider_id: Some(self.provider_id),
            account_id: Some(self.account_id),
            model_original: Some(self.original_model.clone()),
            model_mapped: Some(self.mapped_model.clone()),
            api_type: self.api_type.clone(),
            error_message: None,
            ..LogEntry::new_proxy_entry()
        }
    }

    /// 组装 LogTaskContext（消费 self，注入 response 阶段数据）
    pub fn into_log_task_context(
        self,
        log_service: Arc<LogService>,
        status_code: u16,
        duration: Duration,
        response_text: String,
        resp_headers: HeaderMap,
    ) -> LogTaskContext {
        let mut entry = self.build_log_entry();
        entry.status_code = Some(status_code as i16);
        entry.duration_ms = Some(duration.as_millis() as i32);

        LogTaskContext {
            log_service,
            entry,
            request_headers: self.request_headers,
            request_body: self.request_body,
            response_text,
            resp_headers,
        }
    }

    /// 创建客户端断开检测守卫（流式场景使用）
    pub fn into_interrupt_guard(
        self,
        log_service: Arc<LogService>,
        status_code: u16,
        start: Instant,
        buffer: Arc<std::sync::Mutex<String>>,
        resp_headers: HeaderMap,
        runtime: tokio::runtime::Handle,
    ) -> InterruptGuard {
        InterruptGuard {
            completed: false,
            log_service,
            log_ctx: self,
            status_code,
            start,
            buffer,
            resp_headers,
            runtime,
        }
    }
}

// ─── LogTaskContext ────────────────────────────────────────────────────

pub struct LogTaskContext {
    log_service: Arc<LogService>,
    entry: LogEntry,
    request_headers: HeaderMap,
    request_body: serde_json::Value,
    response_text: String,
    resp_headers: HeaderMap,
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

// ─── InterruptGuard ────────────────────────────────────────────────────

/// 客户端断开检测守卫
///
/// 当 generator 被提前 drop（客户端断开连接）时，
/// Drop 中写入一条标记了 is_interrupted 的部分日志。
pub struct InterruptGuard {
    pub(crate) completed: bool,
    log_service: Arc<LogService>,
    log_ctx: LogContext,
    status_code: u16,
    start: Instant,
    pub(crate) buffer: Arc<std::sync::Mutex<String>>,
    resp_headers: HeaderMap,
    runtime: tokio::runtime::Handle,
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

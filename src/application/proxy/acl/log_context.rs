use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use uuid::Uuid;

use crate::application::log::LogService;
use crate::domain::access_point::AccessPointEx;
use crate::domain::log::LogEntry;
use crate::infrastructure::http_client::ProcessedRequest;

use super::interrupt_guard::InterruptGuard;
use super::log_task_context::LogTaskContext;

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
    pub(crate) request_headers: HeaderMap,
    pub(crate) request_body: serde_json::Value,
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

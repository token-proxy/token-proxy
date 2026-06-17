use std::sync::Arc;
use std::time::Instant;

use axum::http::HeaderMap;
use bytes::Bytes;
use uuid::Uuid;

use crate::application::log::dto::ProxyLogData;
use crate::application::log::LogService;
use crate::domain::access_point::AccessPointEx;

use super::processed_request::ProcessedRequest;

/// 代理日志积累器（防腐 + 累积）
///
/// 贯穿一次代理转发的完整生命周期，逐步填充 `ProxyLogData`：
/// 请求头体（构造时）→ 响应头（构造时）→ body 片段（逐段追加）。
///
/// 构造时从 `ProcessedRequest` 和 `AccessPointEx` 提取请求侧字段，
/// 翻译为 `ProxyLogData` 的对应字段（防腐职责）。
/// 生命周期结束时（正常 flush 或 Drop 自动兜底），
/// 将完整的 DTO 交给 `LogService::record_proxy_log()`。
pub struct ProxyLogger {
    log_service: Arc<LogService>,
    data: ProxyLogData,
    start: Instant,
    flushed: bool,
}

impl ProxyLogger {
    /// 构造 Logger，从域对象 / 基础设施类型翻译为 ProxyLogData
    pub fn new(
        processed: ProcessedRequest,
        access_point: &AccessPointEx,
        user_id: Uuid,
        status_code: u16,
        start: Instant,
        resp_headers: HeaderMap,
        log_service: Arc<LogService>,
    ) -> Self {
        let timestamp = chrono::Utc::now().fixed_offset();

        let data = ProxyLogData {
            timestamp,
            session_id: processed.session_id,
            user_id,
            access_point_id: access_point.id,
            provider_id: access_point.provider_id,
            account_id: access_point.account_id,
            model_original: processed.inbound.model().to_string(),
            model_mapped: processed.outbound.model().to_string(),
            api_type: access_point.api_type.to_string(),
            status_code,
            request_headers: processed.inbound.headers().clone(),
            request_body: processed.inbound.body().clone(),
            resp_headers,
            response_body: String::new(),
            duration_ms: 0,
            is_interrupted: false,
            error_message: None,
        };

        ProxyLogger {
            log_service,
            data,
            start,
            flushed: false,
        }
    }

    /// 流式路径：逐段追加响应体
    pub fn append_body(&mut self, bytes: &Bytes) {
        self.data
            .response_body
            .push_str(&String::from_utf8_lossy(bytes));
    }

    /// 非流式路径：一次性设置响应体
    pub fn set_body(&mut self, bytes: &Bytes) {
        self.data.response_body = String::from_utf8_lossy(bytes).to_string();
    }

    /// 正常完成时调用：计算耗时标记 → 交出 DTO → spawn 异步写入
    pub fn flush(&mut self) {
        if self.flushed {
            return;
        }
        self.flushed = true;

        self.data.duration_ms = self.start.elapsed().as_millis() as i32;
        if self.data.is_interrupted {
            self.data.error_message = Some("客户端断开连接".to_string());
        }

        let log_service = self.log_service.clone();
        let data = self.data.clone();

        tokio::spawn(async move {
            if let Err(e) = log_service.record_proxy_log(data).await {
                tracing::error!(error = %e, "代理日志写入失败");
            }
        });
    }
}

impl Drop for ProxyLogger {
    fn drop(&mut self) {
        if !self.flushed {
            self.data.is_interrupted = true;
            self.flush();
        }
    }
}

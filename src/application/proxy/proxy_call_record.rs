//! 代理调用记录器 — application/proxy/
//!
//! 贯穿一次代理转发的完整生命周期，按业务时序累积日志数据。
//! 取代旧的 `ProxyLogger` + `ProxyLogData` + `build_proxy_log_data` 三件套，
//! 把"构造时一次性传齐 + 运行时改写占位字段"的双重职责拆开，
//! API 形态对应业务时序：
//!
//! ```text
//!     start(请求侧已知)
//!       └─→ attach_response(响应头到达后)
//!             └─→ append_body / set_body (响应体逐段或一次性)
//!                   └─→ finish() (正常完成；spawn 异步落库)
//!                         └─→ Drop 兜底 (未 finish 则标记 is_interrupted 并落库)
//! ```
//!
//! 落库通过 `TrackedSpawner` 异步进行，主进程优雅关闭时轮询计数器归零。

use std::time::Instant;

use axum::http::{HeaderMap, StatusCode};
use bytes::Bytes;
use chrono::{DateTime, FixedOffset};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::application::log::dto::ProxyLogInput;
use crate::application::log::LogService;
use crate::domain::access_point::AccessPointEx;
use crate::domain::shared::{InboundRequest, UpstreamRequest};

use super::tracked_spawner::TrackedSpawner;

/// 响应侧切片（attach_response 后填充）
struct ResponseSlice {
    status_code: u16,
    resp_headers: HeaderMap,
}

/// 代理调用记录器
///
/// 通过 `start` 在请求侧已知时立即启动计时，随后按业务时序逐步登记响应信息。
/// 调用 `finish` 表示正常完成；若 owner 在 `finish` 之前被 drop（如客户端
/// 中断 SSE 流），`Drop` 兜底标记 `is_interrupted=true` 并完成落库。
pub struct ProxyCallRecord {
    log_service: Arc<LogService>,
    spawner: TrackedSpawner,

    // ── 请求侧（start 时锁定，不再变动）────────────────────────────
    timestamp: DateTime<FixedOffset>,
    session_id: Option<String>,
    user_id: Uuid,
    access_point_id: Uuid,
    provider_id: Uuid,
    account_id: Uuid,
    api_type: String,
    model_original: String,
    model_mapped: String,
    request_headers: HeaderMap,
    request_body: Value,

    // ── 响应侧（attach_response 后填充）─────────────────────────────
    response: Option<ResponseSlice>,

    // ── 累积态 ─────────────────────────────────────────────────────
    body: String,

    // ── 终态 ───────────────────────────────────────────────────────
    start: Instant,
    finished: bool,
}

impl ProxyCallRecord {
    /// 阶段 1：在上游 forward 之前调用
    ///
    /// 从入站请求与即将发出的上游请求提取请求侧信息，启动计时。
    /// 此时响应侧（status_code / resp_headers）尚未到达，由后续 `attach_response` 登记。
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        inbound: &InboundRequest,
        upstream: &UpstreamRequest,
        access_point: &AccessPointEx,
        session_id: Option<String>,
        user_id: Uuid,
        provider_id: Uuid,
        account_id: Uuid,
        log_service: Arc<LogService>,
        spawner: TrackedSpawner,
    ) -> Self {
        ProxyCallRecord {
            log_service,
            spawner,
            timestamp: chrono::Utc::now().fixed_offset(),
            session_id,
            user_id,
            access_point_id: access_point.id,
            provider_id,
            account_id,
            api_type: access_point.api_type.to_string(),
            model_original: inbound.model.clone(),
            model_mapped: upstream.mapped_model.clone(),
            request_headers: inbound.headers.clone(),
            request_body: inbound.body.clone(),
            response: None,
            body: String::new(),
            start: Instant::now(),
            finished: false,
        }
    }

    /// 阶段 2：上游响应头到达后登记
    pub fn attach_response(&mut self, status: StatusCode, headers: &HeaderMap) {
        self.response = Some(ResponseSlice {
            status_code: status.as_u16(),
            resp_headers: headers.clone(),
        });
    }

    /// 阶段 3a：SSE 流式逐段追加响应体（过滤 PostgreSQL 不接受的 null 字节）
    pub fn append_body(&mut self, bytes: &Bytes) {
        self.body.push_str(&sanitize_body_bytes(bytes));
    }

    /// 阶段 3b：非流式一次性写入响应体
    pub fn set_body(&mut self, bytes: &Bytes) {
        self.body = sanitize_body_bytes(bytes);
    }

    /// 阶段 4：正常完成调用，spawn 异步落库
    ///
    /// 调用后 `Drop` 不再兜底（`finished=true` 后 drop 是 no-op）。
    pub fn finish(mut self) {
        self.flush(false);
    }

    /// 内部 flush：构造 `ProxyLogInput` 并通过 `TrackedSpawner` spawn 异步落库
    ///
    /// `interrupted=true` 时（由 Drop 触发）会写 `is_interrupted=true` + 默认错误消息。
    /// runtime 已关闭场景由 `TrackedSpawner` 内部守卫降级，不 panic。
    fn flush(&mut self, interrupted: bool) {
        if self.finished {
            return;
        }
        self.finished = true;

        let duration_ms = self.start.elapsed().as_millis() as i32;
        let is_interrupted = interrupted;
        let error_message = if is_interrupted {
            Some("客户端断开连接".to_string())
        } else {
            None
        };

        // 响应侧未登记的边界情况（如 attach_response 之前就被 drop）：
        // 用 0 占位 status_code + 空 headers，让日志至少记下"调用发起过"。
        let (status_code, resp_headers) = match self.response.take() {
            Some(r) => (r.status_code, r.resp_headers),
            None => (0, HeaderMap::new()),
        };

        let input = ProxyLogInput {
            timestamp: self.timestamp,
            session_id: self.session_id.clone(),
            user_id: self.user_id,
            access_point_id: self.access_point_id,
            provider_id: self.provider_id,
            account_id: self.account_id,
            model_original: std::mem::take(&mut self.model_original),
            model_mapped: std::mem::take(&mut self.model_mapped),
            api_type: std::mem::take(&mut self.api_type),
            status_code,
            request_headers: std::mem::take(&mut self.request_headers),
            request_body: std::mem::take(&mut self.request_body),
            resp_headers,
            response_body: std::mem::take(&mut self.body),
            duration_ms,
            is_interrupted,
            error_message,
        };

        let log_service = self.log_service.clone();
        self.spawner.spawn("proxy_log", async move {
            log_service.record_proxy_log(input).await.map(|_| ())
        });
    }
}

impl Drop for ProxyCallRecord {
    fn drop(&mut self) {
        if !self.finished {
            self.flush(true);
        }
    }
}

/// 过滤响应体中的 null 字节（0x00），避免 PostgreSQL TEXT 列拒绝存储
fn sanitize_body_bytes(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    if s.contains('\0') {
        s.replace('\0', "")
    } else {
        s.into_owned()
    }
}

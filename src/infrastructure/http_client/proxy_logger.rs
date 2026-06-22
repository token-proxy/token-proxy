//! 代理日志积累器（基础设施层）
//!
//! 贯穿一次代理转发的完整生命周期，逐步填充 `ProxyLogData`。
//! 生命周期结束时（正常 flush 或 Drop 自动兜底），将完整的 DTO 交给 `LogService`。
//! 自身不记录 tracing 日志——日志记录由 `LogService` 在写入时统一处理。

use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;

use crate::application::log::dto::ProxyLogData;
use crate::application::log::LogService;

/// 代理日志积累器（累积 + 防腐）
///
/// 贯穿一次代理转发的完整生命周期，逐步填充 `ProxyLogData`：
/// 请求头体（外部构造 ProxyLogData 时完成）→ body 片段（逐段追加）。
///
/// 调用方负责在构造前组装好 `ProxyLogData`（从 `ProcessedRequest`、
/// `AccessPointEx` 等提取字段），Logger 只负责运行时的 body 积累和最终 flush。
/// 生命周期结束时（正常 flush 或 Drop 自动兜底），
/// 将完整的 DTO 交给 `LogService::record_proxy_log()`。
pub struct ProxyLogger {
    log_service: Arc<LogService>,
    data: ProxyLogData,
    start: Instant,
    flushed: bool,
    /// 飞行中写入计数器，用于优雅关闭时等待异步写入完成
    in_flight_writes: Arc<AtomicI64>,
}

impl ProxyLogger {
    /// 构造 Logger，接收已组装好的 ProxyLogData
    pub fn new(
        data: ProxyLogData,
        log_service: Arc<LogService>,
        start: Instant,
        in_flight_writes: Arc<AtomicI64>,
    ) -> Self {
        ProxyLogger {
            log_service,
            data,
            start,
            flushed: false,
            in_flight_writes,
        }
    }

    /// 流式路径：逐段追加响应体（过滤 PostgreSQL 不接受的 null 字节）
    pub fn append_body(&mut self, bytes: &Bytes) {
        self.data
            .response_body
            .push_str(&sanitize_body_bytes(bytes));
    }

    /// 非流式路径：一次性设置响应体（过滤 PostgreSQL 不接受的 null 字节）
    pub fn set_body(&mut self, bytes: &Bytes) {
        self.data.response_body = sanitize_body_bytes(bytes);
    }

    /// 正常完成时调用：计算耗时标记 → 交出 DTO → spawn 异步写入
    ///
    /// 写入任务通过 `in_flight_writes` 计数器追踪，
    /// 主线程在优雅关闭时轮询计数器归零，确保数据落库。
    /// 若 runtime 已关闭（例如某些 Drop 场景），仅记录警告日志，避免 panic。
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
        let counter = self.in_flight_writes.clone();

        // 守卫：runtime 关闭时 try_current 返回 Err，避免 tokio::spawn panic
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                counter.fetch_add(1, Ordering::Release);
                handle.spawn(async move {
                    if let Err(e) = log_service.record_proxy_log(data).await {
                        tracing::error!(error = %e, "代理日志写入失败");
                    }
                    counter.fetch_sub(1, Ordering::Release);
                });
            }
            Err(_) => {
                tracing::warn!(
                    status_code = self.data.status_code,
                    duration_ms = self.data.duration_ms,
                    "运行时已关闭，代理日志丢弃",
                );
            }
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

impl Drop for ProxyLogger {
    fn drop(&mut self) {
        if !self.flushed {
            self.data.is_interrupted = true;
            self.flush();
        }
    }
}

//! 上游响应分类 — domain/proxy/upstream_outcome.rs
//!
//! 将一次上游响应按"业务后续动作"分为四种 outcome，
//! 取代 `proxy_pipeline.rs` 中过去基于 `status.is_success() + is_sse` 的嵌套 if 树。
//! 让响应分类成为类型系统中的一阶概念，调用方通过 `match` 穷尽处理。

use axum::http::{HeaderMap, StatusCode};
use chrono::{DateTime, FixedOffset};

use crate::domain::provider::provider::Model as Provider;
use crate::domain::provider::{DisabledReason, FaultOutcome, FaultService};

/// 上游响应的业务分类
///
/// `sse` 标记响应是否为 SSE 流式响应，决定后续如何消费响应体。
#[derive(Debug)]
pub enum UpstreamOutcome {
    /// 2xx 成功响应
    Success { sse: bool },
    /// 4xx 客户端错误（非故障类，如 400/401/404），透传给客户端不禁用账号不重试
    ClientError { sse: bool },
    /// 命中 FaultService 故障配置 —— 账号需禁用
    ///
    /// `sse=true` 时由于响应已开始流式输出，不可切换账号，
    /// 仅完成禁用后透传剩余流给客户端；`sse=false` 时禁用后切换下一账号重试。
    Fault {
        sse: bool,
        reason: DisabledReason,
        available_at: Option<DateTime<FixedOffset>>,
    },
    /// 其他 5xx，未命中故障配置 —— 透传不禁用
    ServerError { sse: bool },
}

impl UpstreamOutcome {
    /// 根据上游响应状态码、headers 和（可选的）body 进行分类
    ///
    /// `resp_body` 仅在非 SSE 路径可读 —— SSE 响应在分类时流尚未消费，
    /// 传 `None` 时 body-based 的故障规则会被静默忽略（仅匹配 status + headers）。
    pub fn classify(
        provider: &Provider,
        status: StatusCode,
        resp_headers: &HeaderMap,
        resp_body: Option<&[u8]>,
        sse: bool,
    ) -> Self {
        if status.is_success() {
            return UpstreamOutcome::Success { sse };
        }

        // 故障检测仅对非 2xx 响应触发。SSE 错误路径传空切片（body 未读）；
        // 非 SSE 错误路径传读到的 body。
        let body_for_detect = resp_body.unwrap_or(&[]);
        let fault = FaultService::detect(
            status.as_u16(),
            provider.rate_limit_config.as_ref(),
            provider.balance_exhausted_config.as_ref(),
            resp_headers,
            body_for_detect,
        );

        if let FaultOutcome::Fault {
            reason,
            available_at,
        } = fault
        {
            return UpstreamOutcome::Fault {
                sse,
                reason,
                available_at,
            };
        }

        // 4xx 非故障 → ClientError；5xx 非故障 → ServerError
        if status.is_client_error() {
            UpstreamOutcome::ClientError { sse }
        } else {
            UpstreamOutcome::ServerError { sse }
        }
    }

    /// 是否为 SSE 响应
    pub fn is_sse(&self) -> bool {
        match self {
            UpstreamOutcome::Success { sse }
            | UpstreamOutcome::ClientError { sse }
            | UpstreamOutcome::Fault { sse, .. }
            | UpstreamOutcome::ServerError { sse } => *sse,
        }
    }

    /// 是否命中故障（账号需禁用）
    pub fn is_fault(&self) -> bool {
        matches!(self, UpstreamOutcome::Fault { .. })
    }
}

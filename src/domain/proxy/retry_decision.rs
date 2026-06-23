//! 重试决策 — domain/proxy/retry_decision.rs
//!
//! 表示尝试一个候选账号后的处理结果，由 `ProxyPipeline::try_one_account` 返回，
//! `ProxyPipeline::execute` 的重试循环消费。
//!
//! 类型系统强制完整性：你不可能 `continue` 而不构造 `Continue(AppError)`，
//! 彻底消除过去 `last_error = Some(...); continue;` 配对易遗漏的隐患。

use crate::shared::error::AppError;

/// 重试决策
pub enum RetryDecision {
    /// 终止重试循环，把这个响应交给客户端
    Return(axum::response::Response),
    /// 当前账号已禁用，继续尝试下一个候选账号
    Continue(AppError),
}

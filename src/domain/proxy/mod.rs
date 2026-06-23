//! 代理转发领域决策 — domain/proxy/
//!
//! 包含上游响应分类（`UpstreamOutcome`）和重试决策（`RetryDecision`）两个一阶领域概念，
//! 取代过去散落在 `proxy_pipeline.rs` 嵌套 if 树中的隐式状态。

pub mod retry_decision;
pub mod upstream_outcome;

pub use retry_decision::RetryDecision;
pub use upstream_outcome::UpstreamOutcome;

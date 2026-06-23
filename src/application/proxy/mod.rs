//! 代理转发应用服务 — application/proxy/
//!
//! 编排 LLM API 代理转发的完整流程。
//!
//! 主要组件：
//! - `ProxyPipeline`：调度入口
//! - `ProxyCallRecord`：贯穿一次代理调用全生命周期的日志记录器
//! - `TrackedSpawner`：统一管理 fire-and-forget 后台写入任务
//! - `AccountSelector`：候选账号迭代器
//! - `UpstreamDispatcher`：上游请求转发执行器
//! - `response_builder`：axum 响应构造（流式 + 非流式）

pub mod account_selector;
pub mod proxy_call_record;
pub mod proxy_pipeline;
pub mod response_builder;
pub mod tracked_spawner;
pub mod upstream_dispatcher;

pub use account_selector::{AccountCandidate, AccountSelector};
pub use proxy_call_record::ProxyCallRecord;
pub use proxy_pipeline::ProxyPipeline;
pub use tracked_spawner::TrackedSpawner;
pub use upstream_dispatcher::UpstreamDispatcher;

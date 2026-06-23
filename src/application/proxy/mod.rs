//! 代理转发应用服务 — application/proxy/
//!
//! 编排 LLM API 代理转发的完整流程。
//! `ProxyPipeline` 是入口编排器，`ProxyCallRecord` 贯穿一次代理调用的全生命周期，
//! `TrackedSpawner` 统一管理 fire-and-forget 后台写入任务。

pub mod proxy_call_record;
pub mod proxy_pipeline;
pub mod tracked_spawner;

pub use proxy_call_record::ProxyCallRecord;
pub use proxy_pipeline::ProxyPipeline;
pub use tracked_spawner::TrackedSpawner;

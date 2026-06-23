//! HTTP 客户端基础设施（基础设施层）
//!
//! 纯 HTTP 执行器 `ProxyClient`。
//! 上游代理调用的日志记录器已经迁移到 `application/proxy/proxy_call_record.rs`，
//! 因为它直接编排领域聚合根，本质属于应用层。

pub mod proxy_client;

pub use proxy_client::ProxyClient;

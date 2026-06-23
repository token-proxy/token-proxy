//! HTTP 客户端基础设施（基础设施层）
//!
//! 包括纯 HTTP 执行器（`ProxyClient`）和代理日志积累器（`ProxyLogger`）。
//! `ProxyLogger` 在 PR 2 中将迁移到 application/proxy/ 并改名 ProxyCallRecord。

pub mod proxy_client;
pub mod proxy_logger;

pub use proxy_client::ProxyClient;
pub use proxy_logger::ProxyLogger;

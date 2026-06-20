//! HTTP 客户端基础设施（基础设施层）
//!
//! 包括上游请求变换（`ProcessedRequest`）、纯 HTTP 执行器（`ProxyClient`）
//! 和代理日志积累器（`ProxyLogger`）。

pub mod processed_request;
pub mod proxy_client;
pub mod proxy_logger;

pub use processed_request::ProcessedRequest;
pub use proxy_client::ProxyClient;
pub use proxy_logger::ProxyLogger;

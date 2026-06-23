pub mod api_key;
pub mod api_type;
pub mod client_type;
pub mod encryption;
pub mod inbound_request;
pub(crate) mod protocols;
pub mod status;
pub mod upstream_request;

pub use api_key::ApiKey;
pub use api_type::AccessPointType;
pub use client_type::ClientType;
pub use encryption::EncryptionService;
pub use inbound_request::InboundRequest;
pub use status::Status;
pub use upstream_request::UpstreamRequest;

/// 标准 hop-by-hop 头（RFC 2616 Section 13.5.1）+ host / content-length
///
/// 代理转发时入站请求头和上游响应头均需过滤这些头。
pub const HOP_BY_HOP_HEADERS: &[&str] = &[
    "transfer-encoding",
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "upgrade",
    "host",
    "content-length",
];

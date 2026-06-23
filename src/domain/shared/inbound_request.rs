//! 入站请求数据结构 — domain/shared/
//!
//! 表示一次代理转发中由客户端发来的入站请求经协议解析后的数据形态。
//! 纯数据 struct，无任何业务行为。
//! 协议感知的解析与变换逻辑挂在 `AccessPointType` 的方法上，
//! 具体实现位于 `src/domain/shared/protocols/<name>.rs`。

use axum::http::HeaderMap;
use serde_json::Value;

use super::AccessPointType;

/// 入站请求快照
///
/// 由 `AccessPointType::parse_inbound` 构造，承载客户端原始请求经协议解析后的数据。
/// 字段已是协议无关的统一形态：headers / body(JSON) / 提取出的 model。
#[derive(Clone, Debug)]
pub struct InboundRequest {
    /// 接入点 API 类型，用于后续协议分发（提取 session_id、注入 api key 等）
    pub api_type: AccessPointType,
    /// 入站请求头（已经移除 hop-by-hop 头之外的原始版本）
    pub headers: HeaderMap,
    /// 入站请求体（JSON 解析后）
    pub body: Value,
    /// 从 body 中提取的模型名称
    pub model: String,
}

//! 上游请求数据结构 — domain/shared/
//!
//! 表示一次代理转发中即将发送给上游 LLM API 的请求数据。
//! 由 `AccessPointEx::build_upstream_request` 构造，
//! 是入站请求经过 URL 拼接、模型路由、协议适配后的产物。
//! 纯数据 struct，无业务行为。

use axum::http::HeaderMap;
use serde_json::Value;

/// 上游请求快照
///
/// 携带向上游 LLM API 发起 HTTP 请求所需的全部数据。
/// `mapped_model` 是路由网格查表后的目标模型名（用于日志审计与可观测性）。
#[derive(Clone, Debug)]
pub struct UpstreamRequest {
    /// 上游 API 完整 URL（Provider base_url + 入站路径剩余部分）
    pub url: String,
    /// 出站请求头（已注入 API key，已过滤 hop-by-hop 头）
    pub headers: HeaderMap,
    /// 出站请求体（已替换 model 字段）
    pub body: Value,
    /// 模型路由后的目标模型名
    pub mapped_model: String,
}

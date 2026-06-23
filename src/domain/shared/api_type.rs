//! 接入点 API 类型枚举 — domain/shared/
//!
//! 定义 `AccessPointType` 枚举（目前仅 Anthropic）以及挂载在其上的协议方法。
//! 协议方法（`parse_inbound` / `extract_session_id` / `inject_api_key` /
//! `replace_model_in_body` / `is_sse_response`）通过 match 分发到
//! `src/domain/shared/protocols/<name>.rs` 中的具体实现。
//!
//! 新增协议类型需同步修改：本枚举 + 数据库列约束 + 前端 Select；
//! Rust 端编译器会自动指出所有 match 分支需要补充的位置。

use axum::http::HeaderMap;
use serde_json::Value;

use crate::shared::error::AppError;
use sea_orm::prelude::StringLen;
use sea_orm::DeriveActiveEnum;
use sea_orm::EnumIter;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::inbound_request::InboundRequest;
use super::protocols::anthropic;

/// 接入点 API 类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum AccessPointType {
    #[sea_orm(string_value = "anthropic")]
    Anthropic,
}

impl AccessPointType {
    pub fn all_variants() -> Vec<AccessPointType> {
        vec![AccessPointType::Anthropic]
    }
}

// ─── 协议适配方法 ──────────────────────────────────────────────────
//
// 每个方法 match self 后调用对应协议模块的 pub(super) fn。
// 加新协议时只需补充 match 分支（编译器会指出所有需要补的位置）。

impl AccessPointType {
    /// 解析入站请求（JSON body 解析 + 协议特定的 model 字段提取）
    pub fn parse_inbound(
        &self,
        headers: HeaderMap,
        body: String,
    ) -> Result<InboundRequest, AppError> {
        match self {
            AccessPointType::Anthropic => anthropic::parse_inbound(self.clone(), headers, body),
        }
    }

    /// 提取客户端会话标识（不同协议读取不同 header 名）
    ///
    /// 返回 `None` 表示请求未携带会话标识；类型化的 Option 取代了之前的 "unknown" sentinel。
    pub fn extract_session_id(&self, inbound: &InboundRequest) -> Option<String> {
        match self {
            AccessPointType::Anthropic => anthropic::extract_session_id(&inbound.headers),
        }
    }

    /// 向上游请求头注入 API key（不同协议用不同 header 名/格式）
    pub fn inject_api_key(&self, headers: &mut HeaderMap, key: &str) {
        match self {
            AccessPointType::Anthropic => anthropic::inject_api_key(headers, key),
        }
    }

    /// 替换请求体中的 model 字段（用于模型路由网格的映射）
    pub fn replace_model_in_body(&self, body: &Value, new_model: &str) -> Value {
        match self {
            AccessPointType::Anthropic => anthropic::replace_model_in_body(body, new_model),
        }
    }

    /// 判断上游响应是否为 SSE 流式响应（基于 Content-Type）
    pub fn is_sse_response(&self, resp_headers: &HeaderMap) -> bool {
        match self {
            AccessPointType::Anthropic => anthropic::is_sse_response(resp_headers),
        }
    }
}

impl fmt::Display for AccessPointType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessPointType::Anthropic => write!(f, "anthropic"),
        }
    }
}

impl FromStr for AccessPointType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(AccessPointType::Anthropic),
            _ => Err(AppError::Validation(format!("不支持的接入点类型: {}", s))),
        }
    }
}

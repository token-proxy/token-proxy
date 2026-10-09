//! 接入点 API 类型枚举 — domain/shared/
//!
//! 定义 `AccessPointType`（接入点类型，落库 + 前端 Select 使用）以及
//! 它与请求级协议值对象 [`ApiProtocol`] 的关系。
//!
//! 职责边界：
//! - `AccessPointType` 回答「这个接入点属于哪个协议家族」——是实体字段，
//!   决定 Provider 上取哪个 base_url、前端新建接入点时选哪一项
//! - [`ApiProtocol`] 回答「这一次请求实际走哪个上游协议」——是请求级值对象，
//!   由入站路径推导，承载全部协议行为（解析 / 变换 / 判定）
//!
//! OpenAI 家族含两条线协议（Chat Completions / Responses），二者共用
//! `openai` 接入点与同一 base_url，因此**不需要**为 Responses 新建接入点类型，
//! 协议由 [`AccessPointType::resolve_protocol`] 按入站路径推导。
//!
//! 新增协议家族需同步修改：本枚举 + 数据库列约束（VARCHAR）+ 前端 Select；
//! Rust 端编译器会自动指出所有 match 分支需要补充的位置。

use sea_orm::prelude::StringLen;
use sea_orm::DeriveActiveEnum;
use sea_orm::EnumIter;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::shared::error::AppError;

use super::api_protocol::ApiProtocol;

/// 接入点 API 类型（协议家族）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum AccessPointType {
    #[sea_orm(string_value = "anthropic")]
    Anthropic,
    /// OpenAI 协议家族（Chat Completions + Responses 两条线协议）
    #[sea_orm(string_value = "openai")]
    OpenAi,
}

impl AccessPointType {
    /// 全部类型变体
    pub fn all_variants() -> Vec<AccessPointType> {
        vec![AccessPointType::Anthropic, AccessPointType::OpenAi]
    }

    /// 该接入点家族允许的请求级协议集合
    ///
    /// 用于管道第 1 步的家族校验：`anthropic` 接入点收到 `/v1/responses` 路径时
    /// 应被明确拒绝，而不是把 Anthropic body 发去 OpenAI 端点。
    pub fn allowed_protocols(&self) -> &'static [ApiProtocol] {
        match self {
            AccessPointType::Anthropic => &[ApiProtocol::Anthropic],
            AccessPointType::OpenAi => &[ApiProtocol::OpenAi, ApiProtocol::OpenAiResponse],
        }
    }

    /// 按入站路径推导本次请求的请求级协议
    ///
    /// Anthropic 家族只有一种协议，直接返回；OpenAI 家族按路径区分
    /// Responses 与 Chat Completions（见 [`ApiProtocol::detect_openai`]）。
    ///
    /// 返回 `None` 表示路径明显属于**另一个协议家族**（如 `anthropic` 接入点收到
    /// `/v1/responses`）——此时不应回落成本家族协议把请求发去错误的端点，
    /// 而应由调用方明确拒绝。判定依据是路径中的 OpenAI 专属端点标记。
    pub fn resolve_protocol(&self, remainder: &str) -> Option<ApiProtocol> {
        match self {
            AccessPointType::Anthropic => {
                if is_openai_only_endpoint(remainder) {
                    None
                } else {
                    Some(ApiProtocol::Anthropic)
                }
            }
            AccessPointType::OpenAi => Some(ApiProtocol::detect_openai(remainder)),
        }
    }

    /// 校验该接入点是否接受指定请求级协议
    pub fn allows(&self, protocol: ApiProtocol) -> bool {
        self.allowed_protocols().contains(&protocol)
    }
}

impl fmt::Display for AccessPointType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessPointType::Anthropic => write!(f, "anthropic"),
            AccessPointType::OpenAi => write!(f, "openai"),
        }
    }
}

/// 判断路径是否命中 OpenAI 专属端点
///
/// 用于 Anthropic 接入点的家族守卫：`/v1/responses`、`/v1/chat/completions`
/// 这类路径只可能属于 OpenAI 家族，不应被当成 Anthropic 协议转发。
fn is_openai_only_endpoint(remainder: &str) -> bool {
    let path = remainder.to_lowercase();
    path.contains("responses") || path.contains("chat/completions")
}

impl FromStr for AccessPointType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(AccessPointType::Anthropic),
            "openai" => Ok(AccessPointType::OpenAi),
            _ => Err(AppError::Validation(format!("不支持的接入点类型: {}", s))),
        }
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_resolves_to_anthropic_protocol() {
        assert_eq!(
            AccessPointType::Anthropic.resolve_protocol("v1/messages"),
            Some(ApiProtocol::Anthropic)
        );
    }

    #[test]
    fn openai_resolves_responses_path() {
        assert_eq!(
            AccessPointType::OpenAi.resolve_protocol("v1/responses"),
            Some(ApiProtocol::OpenAiResponse)
        );
    }

    #[test]
    fn openai_resolves_chat_path() {
        assert_eq!(
            AccessPointType::OpenAi.resolve_protocol("v1/chat/completions"),
            Some(ApiProtocol::OpenAi)
        );
    }

    #[test]
    fn anthropic_rejects_openai_only_endpoints() {
        // 家族守卫：Anthropic 接入点收到 OpenAI 专属路径应不可解析
        assert_eq!(
            AccessPointType::Anthropic.resolve_protocol("v1/responses"),
            None
        );
        assert_eq!(
            AccessPointType::Anthropic.resolve_protocol("v1/chat/completions"),
            None
        );
    }

    #[test]
    fn openai_accepts_unknown_path_as_chat() {
        // OpenAI 家族对未知路径兜底为 Chat Completions（保持既有行为）
        assert_eq!(
            AccessPointType::OpenAi.resolve_protocol("v1/models"),
            Some(ApiProtocol::OpenAi)
        );
    }

    #[test]
    fn family_guard_rejects_cross_family_protocol() {
        assert!(AccessPointType::Anthropic.allows(ApiProtocol::Anthropic));
        assert!(!AccessPointType::Anthropic.allows(ApiProtocol::OpenAi));
        assert!(!AccessPointType::Anthropic.allows(ApiProtocol::OpenAiResponse));
    }

    #[test]
    fn openai_family_allows_both_wire_protocols() {
        assert!(AccessPointType::OpenAi.allows(ApiProtocol::OpenAi));
        assert!(AccessPointType::OpenAi.allows(ApiProtocol::OpenAiResponse));
        assert!(!AccessPointType::OpenAi.allows(ApiProtocol::Anthropic));
    }

    #[test]
    fn from_str_roundtrip() {
        for t in AccessPointType::all_variants() {
            assert_eq!(AccessPointType::from_str(&t.to_string()).unwrap(), t);
        }
    }
}

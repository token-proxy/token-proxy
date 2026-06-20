//! 上游请求变换（基础设施层防腐）
//!
//! 将原始入站请求变换为可直接发送给上游 API 的请求，
//! 内聚 URL 构造、流检测、session 提取等编排逻辑。

use axum::http::HeaderMap;

use crate::domain::access_point::AccessPointEx;
use crate::domain::provider::provider::Model as Provider;
use crate::domain::shared::RequestSnapshot;
use crate::shared::error::AppError;

/// 处理后的上游请求
///
/// 将原始入站请求变换为可直接发送给上游 API 的请求，
/// 内聚 URL 构造、流检测、session 提取等编排逻辑。
/// 模型映射和 header 变换委托给 `AccessPointEx::transform_request_snapshot`。
pub struct ProcessedRequest {
    /// 目标上游 URL（Provider base_url + remainder 路径拼接）
    pub upstream_url: String,
    /// 原始入站请求快照
    pub inbound: RequestSnapshot,
    /// 变换后的出站请求快照（已替换 Authorization header 等）
    pub outbound: RequestSnapshot,
    /// 从入站请求中提取的会话 ID
    pub session_id: String,
}

impl ProcessedRequest {
    /// 从原始入站请求变换为上游请求
    pub fn prepare(
        access_point: &AccessPointEx,
        upstream_key: &str,
        remainder: &str,
        headers: HeaderMap,
        body: &str,
        provider: &Provider,
    ) -> Result<Self, AppError> {
        // URL 构造：base_url 来自 Provider（按 api_type 选择）
        let base_url = provider.base_url_for(&access_point.api_type)?;
        let upstream_url = format!("{}/{}", base_url.trim_end_matches('/'), remainder);

        let inbound = RequestSnapshot::parse(&access_point.api_type, headers, body.to_string())?;

        let session_id = inbound.session_id();

        let outbound =
            access_point.transform_request_snapshot(&inbound, upstream_key, &provider.id)?;

        Ok(ProcessedRequest {
            upstream_url,
            inbound,
            outbound,
            session_id,
        })
    }
}

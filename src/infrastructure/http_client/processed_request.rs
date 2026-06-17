use axum::http::HeaderMap;

use crate::domain::access_point::AccessPointEx;
use crate::domain::shared::RequestSnapshot;
use crate::shared::error::AppError;

/// 处理后的上游请求
///
/// 将原始入站请求变换为可直接发送给上游 API 的请求，
/// 内聚 URL 构造、流检测、session 提取等编排逻辑。
/// 模型映射和 header 变换委托给 `AccessPointEx::transform_request_snapshot`。
pub struct ProcessedRequest {
    pub upstream_url: String,
    pub inbound: RequestSnapshot,
    pub outbound: RequestSnapshot,
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
    ) -> Result<Self, AppError> {
        let base_url = access_point.base_url()?;
        let upstream_url = format!("{}/{}", base_url.trim_end_matches('/'), remainder);

        let inbound = RequestSnapshot::parse(&access_point.api_type, headers, body.to_string())?;

        let session_id = inbound.session_id();

        let outbound = access_point.transform_request_snapshot(&inbound, upstream_key)?;

        Ok(ProcessedRequest {
            upstream_url,
            inbound,
            outbound,
            session_id,
        })
    }
}

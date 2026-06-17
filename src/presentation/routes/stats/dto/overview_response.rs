use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct OverviewResponse {
    /// 请求总数
    pub total_requests: u64,
    /// 活跃接入点数量（有日志记录的接入点）
    pub active_access_points: u64,
}

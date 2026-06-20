use serde::Deserialize;

/// 设置账号状态请求体
#[derive(Debug, Clone, Deserialize)]
pub struct SetAccountStatusRequest {
    /// 目标状态（enabled / disabled）
    pub status: String,
}

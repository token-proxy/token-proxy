use serde::Deserialize;

/// 更新当前用户 profile 请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: String,
}

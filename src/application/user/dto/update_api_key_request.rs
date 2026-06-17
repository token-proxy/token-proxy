use serde::Deserialize;

/// 更新 API key 备注请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateApiKeyRequest {
    pub description: String,
}

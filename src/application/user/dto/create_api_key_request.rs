use serde::Deserialize;

/// 创建 API key 请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub description: String,
}

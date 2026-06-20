use serde::Deserialize;

/// 创建账号请求体
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAccountRequest {
    /// 账号名称（可选，不提供则自动生成）
    pub name: Option<String>,
    /// API Key 明文（必填，服务端加密存储）
    pub api_key: String,
}

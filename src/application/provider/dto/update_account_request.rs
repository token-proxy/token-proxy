use serde::Deserialize;

/// 更新账号请求体
///
/// 所有字段可选，仅提供的字段会被更新。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAccountRequest {
    /// 账号名称
    pub name: Option<String>,
    /// API Key 明文（提供时触发重新加密和更新后缀）
    pub api_key: Option<String>,
}

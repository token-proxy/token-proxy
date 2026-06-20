use serde::Deserialize;

/// 创建用户请求体
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    /// 用户名（必填，唯一）
    pub username: String,
    /// 显示名称
    pub display_name: String,
    /// 密码（明文，至少 6 位）
    pub password: String,
}

use serde::Deserialize;

/// 更新用户请求体
///
/// 所有字段可选，仅提供的字段会被更新。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    /// 显示名称
    pub display_name: Option<String>,
    /// 新密码（提供时触发密码重哈希）
    pub password: Option<String>,
    /// 状态（enabled / disabled）
    pub status: Option<String>,
}

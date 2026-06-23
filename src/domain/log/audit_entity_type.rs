//! 审计实体类型枚举 — domain/log/
//!
//! 定义 `AuditEntityType` 枚举，统一所有审计日志中受操作的实体类型。
//! 每个 variant 通过 `Display` 序列化为 snake_case 字符串写入数据库。

use std::fmt;

/// 审计的实体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEntityType {
    /// 接入点
    AccessPoint,
    /// 账号
    Account,
    /// 服务商
    Provider,
    /// 用户
    User,
    /// 用户 API key
    UserApiKey,
    /// 系统设置
    SystemSettings,
    /// 认证会话（登录 / 登出）
    AuthSession,
    /// Refresh Token（刷新被拒）
    RefreshToken,
}

impl fmt::Display for AuditEntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AuditEntityType::AccessPoint => "access_point",
            AuditEntityType::Account => "account",
            AuditEntityType::Provider => "provider",
            AuditEntityType::User => "user",
            AuditEntityType::UserApiKey => "user_api_key",
            AuditEntityType::SystemSettings => "system_settings",
            AuditEntityType::AuthSession => "auth_session",
            AuditEntityType::RefreshToken => "refresh_token",
        };
        write!(f, "{}", s)
    }
}

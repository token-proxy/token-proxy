//! 审计操作类型枚举 — domain/log/
//!
//! 定义 `AuditAction` 枚举，统一所有审计日志的操作类型。
//! 每个 variant 通过 `Display` 序列化为 snake_case 字符串写入数据库，
//! 保持 VARCHAR 列的兼容性。

use std::fmt;

/// 审计操作类型，囊括系统所有需要审计的写操作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    /// 创建实体
    Create,
    /// 非状态类更新（如修改名称、配置）
    Update,
    /// 删除实体
    Delete,
    /// 启用实体
    Enable,
    /// 禁用实体
    Disable,
    /// 手动恢复账号
    Recover,
    /// 系统自动恢复账号（定时任务触发）
    AutoRecover,
    /// 创建 API key
    CreateApiKey,
    /// 吊销 API key
    RevokeApiKey,
    /// 更新 API key 备注
    UpdateApiKeyDescription,
    /// 修改密码
    ChangePassword,
    /// 更新个人资料
    UpdateProfile,
    /// 系统设置变更
    UpdateSettings,
    /// 登录成功
    Login,
    /// 登录失败（含失败原因）
    LoginFailed,
    /// 登出
    Logout,
    /// refresh token 被拒绝（过期、已吊销、无效）
    RefreshRejected,
    /// 模型自动发现（Provider 模型列表变更）
    DiscoverModels,
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AuditAction::Create => "create",
            AuditAction::Update => "update",
            AuditAction::Delete => "delete",
            AuditAction::Enable => "enable",
            AuditAction::Disable => "disable",
            AuditAction::Recover => "recover",
            AuditAction::AutoRecover => "auto_recover",
            AuditAction::CreateApiKey => "create_api_key",
            AuditAction::RevokeApiKey => "revoke_api_key",
            AuditAction::UpdateApiKeyDescription => "update_api_key_description",
            AuditAction::ChangePassword => "change_password",
            AuditAction::UpdateProfile => "update_profile",
            AuditAction::UpdateSettings => "update_settings",
            AuditAction::Login => "login",
            AuditAction::LoginFailed => "login_failed",
            AuditAction::Logout => "logout",
            AuditAction::RefreshRejected => "refresh_rejected",
            AuditAction::DiscoverModels => "discover_models",
        };
        write!(f, "{}", s)
    }
}

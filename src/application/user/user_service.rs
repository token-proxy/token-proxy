//! 用户应用服务 — application/user/
//!
//! 编排系统用户的 CRUD、Profile 更新和密码修改操作。

use std::sync::Arc;

use uuid::Uuid;

use super::dto::{
    ChangePasswordRequest, CreateUserRequest, UpdateProfileRequest, UpdateUserRequest, UserResponse,
};
use crate::domain::log::AuditAction;
use crate::domain::log::AuditEntityType;
use crate::domain::log::AuditLog;
use crate::domain::log::AuditLogRepository;
use crate::domain::shared::Status;
use crate::domain::user::User;
use crate::domain::user::UserRepository;
use crate::domain::user::{RefreshTokenRepository, UserDependents};
use crate::infrastructure::auth::password::{hash_password, verify_password};
use crate::shared::error::AppError;

/// 用户应用服务
///
/// 编排系统用户 CRUD、Profile 更新和密码修改操作。
pub struct UserService {
    user_repo: Arc<dyn UserRepository>,
    refresh_token_repo: Arc<dyn RefreshTokenRepository>,
    audit_log_repo: Arc<dyn AuditLogRepository>,
}

impl UserService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        refresh_token_repo: Arc<dyn RefreshTokenRepository>,
        audit_log_repo: Arc<dyn AuditLogRepository>,
    ) -> Self {
        UserService {
            user_repo,
            refresh_token_repo,
            audit_log_repo,
        }
    }

    fn to_response(user: &User) -> UserResponse {
        UserResponse {
            id: user.id,
            username: user.username.clone(),
            display_name: user.display_name.clone(),
            status: user.status.to_string(),
            created_at: user.created_at.with_timezone(&chrono::Utc),
            updated_at: user.updated_at.with_timezone(&chrono::Utc),
        }
    }

    /// 创建用户
    ///
    /// 校验用户名唯一性和密码长度，哈希密码后保存。
    pub async fn create(
        &self,
        req: CreateUserRequest,
        operator_id: Option<Uuid>,
    ) -> Result<UserResponse, AppError> {
        let trimmed_username = req.username.trim().to_string();
        if trimmed_username.is_empty() {
            return Err(AppError::Validation("用户名不能为空".to_string()));
        }
        if req.password.len() < 6 {
            return Err(AppError::Validation("密码长度不能少于 6 位".to_string()));
        }

        // 检查用户名唯一性
        let exists = self.user_repo.exists_by_username(&trimmed_username).await?;

        if exists {
            return Err(AppError::Conflict(format!(
                "用户名 '{}' 已存在",
                trimmed_username
            )));
        }

        // 哈希密码
        let password_hash =
            hash_password(&req.password).map_err(|e| AppError::Internal(e.to_string()))?;

        let user = User::new(trimmed_username, req.display_name, password_hash);

        let saved = self.user_repo.save(&user).await?;

        self.log_audit(
            operator_id,
            AuditAction::Create,
            AuditEntityType::User,
            Some(saved.id),
            Some(serde_json::json!({"username": saved.username})),
        )
        .await;

        Ok(Self::to_response(&saved))
    }

    /// 更新用户
    ///
    /// 支持更新显示名称、密码和状态（enabled/disabled）。
    pub async fn update(
        &self,
        id: Uuid,
        req: UpdateUserRequest,
        operator_id: Option<Uuid>,
    ) -> Result<UserResponse, AppError> {
        let mut user = self
            .user_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("用户 {} 未找到", id)))?;

        // 记录更新前的状态和用户名，用于审计日志
        let old_status = user.status.to_string();
        let username = user.username.clone();

        if let Some(display_name) = req.display_name {
            user.set_display_name(display_name)?;
        }

        if let Some(password) = req.password {
            if password.len() < 6 {
                return Err(AppError::Validation("密码长度不能少于 6 位".to_string()));
            }
            let hash = hash_password(&password).map_err(|e| AppError::Internal(e.to_string()))?;
            user.set_password_hash(hash);
        }

        let mut status_changed = false;
        if let Some(status_str) = req.status {
            let status: Status = status_str
                .parse()
                .map_err(|e: AppError| AppError::Validation(e.to_string()))?;
            status_changed = status_str != old_status;
            match status {
                Status::Enabled => user.enable(),
                Status::Disabled => user.disable(),
            }
        }

        let saved = self.user_repo.save(&user).await?;

        // 禁用时吊销其全部 refresh token：已登录会话在 access token 过期后无法续期，
        // 与 JWT 校验处的状态检查形成双保险（access token 有 TTL，刷新是续命入口）
        if status_changed && saved.status == Status::Disabled {
            if let Err(e) = self.refresh_token_repo.revoke_all_for_user(saved.id).await {
                tracing::error!(error = %e, user_id = %saved.id, "禁用用户时吊销 refresh token 失败");
            }
        }

        // 按状态变化选择审计操作类型
        let action = if status_changed {
            match saved.status {
                Status::Enabled => AuditAction::Enable,
                Status::Disabled => AuditAction::Disable,
            }
        } else {
            AuditAction::Update
        };

        self.log_audit(
            operator_id,
            action,
            AuditEntityType::User,
            Some(id),
            Some(serde_json::json!({
                "username": username,
                "old_status": old_status,
                "new_status": saved.status.to_string(),
            })),
        )
        .await;

        Ok(Self::to_response(&saved))
    }

    /// 根据 ID 查询用户
    pub async fn get_by_id(&self, id: Uuid) -> Result<UserResponse, AppError> {
        let user = self
            .user_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("用户 {} 未找到", id)))?;

        Ok(Self::to_response(&user))
    }

    /// 查询所有用户列表
    pub async fn list_all(&self) -> Result<Vec<UserResponse>, AppError> {
        let users = self.user_repo.find_all().await?;

        Ok(users.iter().map(Self::to_response).collect())
    }

    /// 删除用户
    pub async fn delete(&self, id: Uuid, operator_id: Option<Uuid>) -> Result<(), AppError> {
        let user = self
            .user_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("用户 {} 未找到", id)))?;
        let username = Some(user.username.clone());

        let enabled_count = self.user_repo.count_enabled().await?;
        let dependents = self.user_repo.count_dependents(id).await?;
        check_delete_allowed(&user, operator_id, enabled_count, dependents)?;

        self.user_repo.delete(id).await?;

        self.log_audit(
            operator_id,
            AuditAction::Delete,
            AuditEntityType::User,
            Some(id),
            username.map(|u| serde_json::json!({"username": u})),
        )
        .await;

        Ok(())
    }

    /// 更新当前用户 profile（仅 display_name）
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        req: UpdateProfileRequest,
    ) -> Result<UserResponse, AppError> {
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("用户 {} 未找到", user_id)))?;

        user.set_display_name(req.display_name)?;

        let saved = self.user_repo.save(&user).await?;

        self.log_audit(
            Some(user_id),
            AuditAction::UpdateProfile,
            AuditEntityType::User,
            Some(user_id),
            Some(serde_json::json!({"display_name": saved.display_name})),
        )
        .await;

        Ok(Self::to_response(&saved))
    }

    /// 修改当前用户密码（需验证旧密码）
    pub async fn change_password(
        &self,
        user_id: Uuid,
        req: ChangePasswordRequest,
    ) -> Result<(), AppError> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("用户 {} 未找到", user_id)))?;

        // 验证旧密码
        let valid = verify_password(&req.old_password, &user.password_hash)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if !valid {
            return Err(AppError::Unauthorized("旧密码错误".to_string()));
        }

        // 校验新密码长度
        if req.new_password.len() < 6 {
            return Err(AppError::Validation("新密码长度不能少于 6 位".to_string()));
        }

        let new_hash =
            hash_password(&req.new_password).map_err(|e| AppError::Internal(e.to_string()))?;

        let mut mutable_user = user;
        mutable_user.set_password_hash(new_hash);

        self.user_repo.save(&mutable_user).await?;

        // 记录审计日志（fire-and-forget 模式）
        self.log_audit(
            Some(user_id),
            AuditAction::ChangePassword,
            AuditEntityType::User,
            Some(user_id),
            Some(serde_json::json!({"action": "change_password"})),
        )
        .await;

        Ok(())
    }

    // ─── 私有辅助方法 ──────────────────────────────────────────────────

    /// 写入审计日志（fire-and-forget，不阻塞主业务）
    async fn log_audit(
        &self,
        operator_id: Option<Uuid>,
        action: AuditAction,
        entity_type: AuditEntityType,
        entity_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) {
        let log = AuditLog::new(operator_id, "user", action, entity_type, entity_id, details);
        if let Err(e) = self.audit_log_repo.save(&log).await {
            tracing::error!(error = %e, action = %action, entity_type = %entity_type, "审计日志写入失败");
        }
    }
}

/// 删除用户的守卫判定（纯函数，便于单测）
///
/// 顺序固定为「先拦自己、再拦最后一名启用用户、最后拦有审计线索的用户」，
/// 每一条都返回可操作的说明：前两条提示改由其它账号操作，第三条引导改用禁用。
pub fn check_delete_allowed(
    target: &User,
    operator_id: Option<Uuid>,
    enabled_count: i64,
    dependents: UserDependents,
) -> Result<(), AppError> {
    if operator_id == Some(target.id) {
        return Err(AppError::Conflict(
            "不能删除当前登录用户，请改用其它管理员账号".to_string(),
        ));
    }

    if target.status.is_enabled() && enabled_count <= 1 {
        return Err(AppError::Conflict(
            "该用户是唯一启用中的用户，删除后将无人可登录".to_string(),
        ));
    }

    // 有审计线索（创建的接入点 / 请求日志 / 审计记录）时禁止物理删除：
    // 此前直接删除会被 access_points.created_by 的 ON DELETE RESTRICT 拒绝，
    // 对外只表现为无信息的 500
    if let Some(message) = dependents.violation_message() {
        return Err(AppError::Conflict(message));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(status: Status) -> User {
        let mut u = User::new(
            "victim".to_string(),
            "离职同事".to_string(),
            "hash".to_string(),
        );
        u.status = status;
        u
    }

    #[test]
    fn test_delete_self_rejected() {
        let u = user(Status::Enabled);
        let err = check_delete_allowed(&u, Some(u.id), 5, UserDependents::default()).unwrap_err();
        assert!(err.to_string().contains("当前登录用户"));
    }

    #[test]
    fn test_last_enabled_user_rejected() {
        let u = user(Status::Enabled);
        let err = check_delete_allowed(&u, None, 1, UserDependents::default()).unwrap_err();
        assert!(err.to_string().contains("唯一启用中的用户"));
    }

    #[test]
    fn test_disabled_user_can_be_deleted_when_no_dependents() {
        let u = user(Status::Disabled);
        // 已经是禁用状态、且无关联数据：允许删除（不占用"唯一启用用户"的判断）
        assert!(check_delete_allowed(&u, None, 1, UserDependents::default()).is_ok());
    }

    #[test]
    fn test_dependents_block_delete_with_actionable_message() {
        let u = user(Status::Disabled);
        let dependents = UserDependents {
            access_points: 1,
            logs: 31733,
            audits: 95,
        };
        let err = check_delete_allowed(&u, None, 6, dependents).unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)), "应为 409 Conflict");
        let msg = err.to_string();
        assert!(msg.contains("接入点 1 个") && msg.contains("请求日志 31733 条"));
        assert!(msg.contains("改为禁用"));
    }

    #[test]
    fn test_delete_allowed_for_enabled_user_when_others_remain() {
        let u = user(Status::Enabled);
        assert!(check_delete_allowed(&u, None, 3, UserDependents::default()).is_ok());
    }
}

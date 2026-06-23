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
use crate::infrastructure::auth::password::{hash_password, verify_password};
use crate::shared::error::AppError;

/// 用户应用服务
///
/// 编排系统用户 CRUD、Profile 更新和密码修改操作。
pub struct UserService {
    user_repo: Arc<dyn UserRepository>,
    audit_log_repo: Arc<dyn AuditLogRepository>,
}

impl UserService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        audit_log_repo: Arc<dyn AuditLogRepository>,
    ) -> Self {
        UserService {
            user_repo,
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
        // 先查询用户名，用于审计日志
        let username = self
            .user_repo
            .find_by_id(id)
            .await?
            .map(|u| u.username.clone());

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

//! 认证应用服务 — application/auth/
//!
//! 编排用户登录、token 刷新、退出登录和 token 验证操作。

use std::sync::Arc;

use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::claims::Claims;
use super::dto::{LoginRequest, LoginResponse, RefreshRequest};
use crate::domain::log::AuditAction;
use crate::domain::log::AuditEntityType;
use crate::domain::log::AuditLog;
use crate::domain::log::AuditLogRepository;
use crate::domain::user::RefreshToken;
use crate::domain::user::RefreshTokenRepository;
use crate::domain::user::UserRepository;
use crate::infrastructure::auth::password::verify_password;
use crate::infrastructure::auth::JwtService;
use crate::shared::error::AppError;

/// 认证应用服务
///
/// 编排登录、token 刷新、退出和 token 验证操作。
pub struct AuthService {
    user_repo: Arc<dyn UserRepository>,
    refresh_token_repo: Arc<dyn RefreshTokenRepository>,
    jwt_service: Arc<JwtService>,
    audit_log_repo: Arc<dyn AuditLogRepository>,
}

impl AuthService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        refresh_token_repo: Arc<dyn RefreshTokenRepository>,
        jwt_service: Arc<JwtService>,
        audit_log_repo: Arc<dyn AuditLogRepository>,
    ) -> Self {
        AuthService {
            user_repo,
            refresh_token_repo,
            jwt_service,
            audit_log_repo,
        }
    }

    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// 写入审计日志（fire-and-forget 风格，失败仅记 error 不阻断主流程）
    ///
    /// `operator_type` 使用 `"system"` 因为登录 / 登出 / 刷新场景下请求方尚未经过身份认证。
    async fn log_audit(
        &self,
        operator_id: Option<Uuid>,
        action: AuditAction,
        entity_type: AuditEntityType,
        entity_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) {
        let log = AuditLog::new(
            operator_id,
            "system",
            action,
            entity_type,
            entity_id,
            details,
        );
        if let Err(e) = self.audit_log_repo.save(&log).await {
            tracing::error!(error = %e, action = %action, entity_type = %entity_type, "审计日志写入失败");
        }
    }

    /// 用户登录
    ///
    /// 验证用户名密码，生成 access_token 和 refresh_token。
    pub async fn login(&self, req: LoginRequest) -> Result<LoginResponse, AppError> {
        let user = match self.user_repo.find_by_username(&req.username).await? {
            Some(user) => user,
            None => {
                // 用户名不存在
                self.log_audit(
                    None,
                    AuditAction::LoginFailed,
                    AuditEntityType::AuthSession,
                    None,
                    Some(
                        serde_json::json!({"username": req.username, "reason": "用户名或密码错误"}),
                    ),
                )
                .await;
                return Err(AppError::Unauthorized("用户名或密码错误".to_string()));
            }
        };

        if !user.status.is_enabled() {
            // 用户已被禁用
            self.log_audit(
                None,
                AuditAction::LoginFailed,
                AuditEntityType::AuthSession,
                None,
                Some(serde_json::json!({"username": req.username, "reason": "用户已被禁用"})),
            )
            .await;
            return Err(AppError::Unauthorized("用户已被禁用".to_string()));
        }

        let valid = verify_password(&req.password, &user.password_hash)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if !valid {
            // 密码错误
            self.log_audit(
                None,
                AuditAction::LoginFailed,
                AuditEntityType::AuthSession,
                None,
                Some(serde_json::json!({"username": req.username, "reason": "用户名或密码错误"})),
            )
            .await;
            return Err(AppError::Unauthorized("用户名或密码错误".to_string()));
        }

        let now = chrono::Utc::now();

        // 生成 access_token
        let access_token = self
            .jwt_service
            .create_access_token(user.id, &user.username)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let expires_in = self.jwt_service.access_expiry_secs();

        // 生成 refresh_token
        let refresh_token_str = self
            .jwt_service
            .create_refresh_token(user.id, &user.username)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // 计算 refresh token hash 并存储
        let token_hash = Self::hash_token(&refresh_token_str);
        let expires_at =
            now + chrono::Duration::seconds(self.jwt_service.refresh_expiry_secs() as i64);

        let refresh_token_entity = RefreshToken::new(user.id, token_hash, expires_at);
        self.refresh_token_repo.save(&refresh_token_entity).await?;

        // 登录成功
        self.log_audit(
            None,
            AuditAction::Login,
            AuditEntityType::AuthSession,
            None,
            Some(serde_json::json!({"username": req.username})),
        )
        .await;

        Ok(LoginResponse {
            access_token,
            refresh_token: refresh_token_str,
            token_type: "Bearer".to_string(),
            expires_in,
            username: user.username.clone(),
            display_name: user.display_name.clone(),
        })
    }

    /// 刷新 token
    ///
    /// 验证 refresh token，原子吊销旧 token 并生成新的 token pair。
    pub async fn refresh(&self, req: RefreshRequest) -> Result<LoginResponse, AppError> {
        let token_hash = Self::hash_token(&req.refresh_token);

        let stored_token = match self
            .refresh_token_repo
            .find_by_token_hash(&token_hash)
            .await?
        {
            Some(token) => token,
            None => {
                // refresh token 不存在
                self.log_audit(
                    None,
                    AuditAction::RefreshRejected,
                    AuditEntityType::RefreshToken,
                    None,
                    Some(serde_json::json!({"reason": "无效的 refresh token"})),
                )
                .await;
                return Err(AppError::Unauthorized("无效的 refresh token".to_string()));
            }
        };

        if !stored_token.is_valid() {
            // refresh token 已过期或已吊销
            self.log_audit(
                None,
                AuditAction::RefreshRejected,
                AuditEntityType::RefreshToken,
                None,
                Some(serde_json::json!({"reason": "refresh token 已过期或已吊销"})),
            )
            .await;
            return Err(AppError::Unauthorized(
                "refresh token 已过期或已吊销".to_string(),
            ));
        }

        // 原子吊销旧 token
        self.refresh_token_repo.revoke(stored_token.id).await?;

        // 获取用户信息
        let user = match self.user_repo.find_by_id(stored_token.user_id).await? {
            Some(user) => user,
            None => {
                // 用户不存在
                self.log_audit(
                    None,
                    AuditAction::RefreshRejected,
                    AuditEntityType::RefreshToken,
                    None,
                    Some(serde_json::json!({"reason": "用户不存在"})),
                )
                .await;
                return Err(AppError::Unauthorized("用户不存在".to_string()));
            }
        };

        if !user.status.is_enabled() {
            // 用户已被禁用
            self.log_audit(
                None,
                AuditAction::RefreshRejected,
                AuditEntityType::RefreshToken,
                None,
                Some(serde_json::json!({"reason": "用户已被禁用"})),
            )
            .await;
            return Err(AppError::Unauthorized("用户已被禁用".to_string()));
        }

        let now = chrono::Utc::now();

        // 生成新的 token pair
        let access_token = self
            .jwt_service
            .create_access_token(user.id, &user.username)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let expires_in = self.jwt_service.access_expiry_secs();

        let refresh_token_str = self
            .jwt_service
            .create_refresh_token(user.id, &user.username)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let new_token_hash = Self::hash_token(&refresh_token_str);
        let expires_at =
            now + chrono::Duration::seconds(self.jwt_service.refresh_expiry_secs() as i64);

        let new_token_entity = RefreshToken::new(user.id, new_token_hash, expires_at);
        self.refresh_token_repo.save(&new_token_entity).await?;

        Ok(LoginResponse {
            access_token,
            refresh_token: refresh_token_str,
            token_type: "Bearer".to_string(),
            expires_in,
            username: user.username.clone(),
            display_name: user.display_name.clone(),
        })
    }

    /// 退出登录（吊销该用户的所有 refresh token）
    pub async fn logout(&self, user_id: Uuid) -> Result<(), AppError> {
        self.refresh_token_repo.revoke_all_for_user(user_id).await?;

        self.log_audit(
            None,
            AuditAction::Logout,
            AuditEntityType::AuthSession,
            None,
            Some(serde_json::json!({"user_id": user_id.to_string()})),
        )
        .await;

        Ok(())
    }

    /// 验证 access token 并返回 JWT 声明
    pub async fn validate_access_token(&self, token: &str) -> Result<Claims, AppError> {
        let jwt_claims = self
            .jwt_service
            .validate_token(token)
            .map_err(|_| AppError::Unauthorized("无效的 access token".to_string()))?;

        Ok(Claims {
            sub: jwt_claims.sub,
            user_id: jwt_claims.user_id,
            username: jwt_claims.username,
            exp: jwt_claims.exp,
            iat: jwt_claims.iat,
        })
    }
}

use std::sync::Arc;

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::application::dto::auth_dto::{Claims, LoginRequest, LoginResponse, RefreshRequest};
use crate::domain::entities::refresh_token::RefreshToken;
use crate::domain::repositories::refresh_token_repository::RefreshTokenRepository;
use crate::domain::repositories::user_repository::UserRepository;
use crate::infrastructure::auth::jwt::JwtService;
use crate::infrastructure::auth::password::verify_password;
use crate::shared::error::AppError;

pub struct AuthService {
    user_repo: Arc<dyn UserRepository>,
    refresh_token_repo: Arc<dyn RefreshTokenRepository>,
    jwt_service: Arc<JwtService>,
}

impl AuthService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        refresh_token_repo: Arc<dyn RefreshTokenRepository>,
        jwt_service: Arc<JwtService>,
    ) -> Self {
        AuthService {
            user_repo,
            refresh_token_repo,
            jwt_service,
        }
    }

    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub async fn login(&self, req: LoginRequest) -> Result<LoginResponse, AppError> {
        let user = self
            .user_repo
            .find_by_username(&req.username)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::Unauthorized("用户名或密码错误".to_string()))?;

        if !user.status.is_enabled() {
            return Err(AppError::Unauthorized("用户已被禁用".to_string()));
        }

        let valid = verify_password(&req.password, &user.password_hash)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if !valid {
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
        let expires_at = now + chrono::Duration::seconds(expires_in as i64);

        let refresh_token_entity = RefreshToken::new(user.id, token_hash, expires_at);
        self.refresh_token_repo
            .save(&refresh_token_entity)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(LoginResponse {
            access_token,
            refresh_token: refresh_token_str,
            token_type: "Bearer".to_string(),
            expires_in,
            username: user.username.clone(),
            display_name: user.display_name.clone(),
        })
    }

    pub async fn refresh(&self, req: RefreshRequest) -> Result<LoginResponse, AppError> {
        let token_hash = Self::hash_token(&req.refresh_token);

        let stored_token = self
            .refresh_token_repo
            .find_by_token_hash(&token_hash)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::Unauthorized("无效的 refresh token".to_string()))?;

        if !stored_token.is_valid() {
            return Err(AppError::Unauthorized(
                "refresh token 已过期或已吊销".to_string(),
            ));
        }

        // 原子吊销旧 token
        self.refresh_token_repo
            .revoke(stored_token.id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 获取用户信息
        let user = self
            .user_repo
            .find_by_id(stored_token.user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::Unauthorized("用户不存在".to_string()))?;

        if !user.status.is_enabled() {
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
        let expires_at = now + chrono::Duration::seconds(expires_in as i64);

        let new_token_entity = RefreshToken::new(user.id, new_token_hash, expires_at);
        self.refresh_token_repo
            .save(&new_token_entity)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(LoginResponse {
            access_token,
            refresh_token: refresh_token_str,
            token_type: "Bearer".to_string(),
            expires_in,
            username: user.username.clone(),
            display_name: user.display_name.clone(),
        })
    }

    pub async fn logout(&self, user_id: Uuid) -> Result<(), AppError> {
        self.refresh_token_repo
            .revoke_all_for_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

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

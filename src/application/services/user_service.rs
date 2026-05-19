use std::sync::Arc;

use uuid::Uuid;

use crate::application::dto::user_dto::{CreateUserRequest, UpdateUserRequest, UserResponse};
use crate::domain::entities::user::User;
use crate::domain::repositories::user_repository::UserRepository;
use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;

pub struct UserService {
    user_repo: Arc<dyn UserRepository>,
}

impl UserService {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        UserService { user_repo }
    }

    fn to_response(user: &User) -> UserResponse {
        UserResponse {
            id: user.id,
            username: user.username.clone(),
            display_name: user.display_name.clone(),
            status: user.status.to_string(),
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }

    pub async fn create(&self, req: CreateUserRequest) -> Result<UserResponse, AppError> {
        let trimmed_username = req.username.trim().to_string();
        if trimmed_username.is_empty() {
            return Err(AppError::Validation("用户名不能为空".to_string()));
        }
        if req.password.len() < 6 {
            return Err(AppError::Validation("密码长度不能少于 6 位".to_string()));
        }

        // 检查用户名唯一性
        let exists = self
            .user_repo
            .exists_by_username(&trimmed_username)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if exists {
            return Err(AppError::Conflict(format!(
                "用户名 '{}' 已存在",
                trimmed_username
            )));
        }

        // 哈希密码
        let password_hash =
            crate::infrastructure::auth::password::hash_password(&req.password)
                .map_err(|e| AppError::Internal(e.to_string()))?;

        let user = User::new(trimmed_username, req.display_name, password_hash);

        let saved = self
            .user_repo
            .save(&user)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Self::to_response(&saved))
    }

    pub async fn update(
        &self,
        id: Uuid,
        req: UpdateUserRequest,
    ) -> Result<UserResponse, AppError> {
        let mut user = self
            .user_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("用户 {} 未找到", id)))?;

        if let Some(display_name) = req.display_name {
            let trimmed = display_name.trim().to_string();
            if trimmed.is_empty() {
                return Err(AppError::Validation("显示名称不能为空".to_string()));
            }
            user.display_name = trimmed;
        }

        if let Some(password) = req.password {
            if password.len() < 6 {
                return Err(AppError::Validation("密码长度不能少于 6 位".to_string()));
            }
            user.password_hash =
                crate::infrastructure::auth::password::hash_password(&password)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        if let Some(status_str) = req.status {
            let status: Status = status_str
                .parse()
                .map_err(|e: AppError| AppError::Validation(e.to_string()))?;
            user.status = status;
        }

        user.updated_at = chrono::Utc::now();

        let saved = self
            .user_repo
            .save(&user)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Self::to_response(&saved))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<UserResponse, AppError> {
        let user = self
            .user_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("用户 {} 未找到", id)))?;

        Ok(Self::to_response(&user))
    }

    pub async fn list_all(&self) -> Result<Vec<UserResponse>, AppError> {
        let users = self
            .user_repo
            .find_all()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(users.iter().map(Self::to_response).collect())
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        self.user_repo
            .delete(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}
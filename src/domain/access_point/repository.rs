use crate::domain::access_point::access_point::{Model as AccessPoint, ModelEx as AccessPointEx};
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait AccessPointRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AccessPoint>, AppError>;

    /// 按短码查找接入点，返回已加载 Provider 和 Account 关联的完整聚合
    async fn find_by_short_code(&self, short_code: &str)
        -> Result<Option<AccessPointEx>, AppError>;

    async fn find_all(&self) -> Result<Vec<AccessPoint>, AppError>;
    async fn find_enabled(&self) -> Result<Vec<AccessPoint>, AppError>;
    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<AccessPoint>, AppError>;
    async fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<AccessPoint>, AppError>;
    async fn save(&self, access_point: &AccessPoint) -> Result<AccessPoint, AppError>;
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
}

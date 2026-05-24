use crate::domain::entities::access_point::AccessPoint;
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait AccessPointRepository: Send + Sync {
    /// 根据 ID 查找接入点
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AccessPoint>, AppError>;

    /// 根据短码查找接入点
    async fn find_by_short_code(&self, short_code: &str) -> Result<Option<AccessPoint>, AppError>;

    /// 查找所有接入点
    async fn find_all(&self) -> Result<Vec<AccessPoint>, AppError>;

    /// 查找所有已启用的接入点
    async fn find_enabled(&self) -> Result<Vec<AccessPoint>, AppError>;

    /// 根据提供商 ID 查找接入点
    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<AccessPoint>, AppError>;

    /// 根据账号 ID 查找接入点
    async fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<AccessPoint>, AppError>;

    /// 保存接入点（创建或更新）
    async fn save(&self, access_point: &AccessPoint) -> Result<AccessPoint, AppError>;

    /// 根据 ID 删除接入点
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
}

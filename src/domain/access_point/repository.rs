//! 接入点仓储接口 — domain/access_point/
//!
//! 定义 `AccessPointRepository` trait，提供接入点及其账户池的持久化契约。

use crate::domain::access_point::access_point::{AccessPointEx, Model as AccessPoint};
use crate::domain::access_point::access_point_account::AccessPointAccount;
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

/// 接入点仓储接口
#[async_trait]
pub trait AccessPointRepository: Send + Sync {
    /// 根据 ID 查找接入点
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AccessPoint>, AppError>;

    /// 按短码查找接入点，返回含账户池的完整聚合
    async fn find_by_short_code(&self, short_code: &str)
        -> Result<Option<AccessPointEx>, AppError>;

    /// 查询所有接入点
    async fn find_all(&self) -> Result<Vec<AccessPoint>, AppError>;
    /// 查询所有已启用的接入点
    async fn find_enabled(&self) -> Result<Vec<AccessPoint>, AppError>;
    /// 根据 Provider ID 查找关联的接入点
    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<AccessPoint>, AppError>;
    /// 根据账号 ID 查找关联的接入点
    async fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<AccessPoint>, AppError>;
    /// 保存或更新接入点
    async fn save(&self, access_point: &AccessPoint) -> Result<AccessPoint, AppError>;
    /// 删除接入点
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;

    /// 查询接入点的账户池
    async fn find_accounts_by_access_point(
        &self,
        access_point_id: Uuid,
    ) -> Result<Vec<AccessPointAccount>, AppError>;

    /// 保存接入点的账户池（全量替换）
    async fn save_accounts(
        &self,
        access_point_id: Uuid,
        accounts: &[AccessPointAccount],
    ) -> Result<(), AppError>;
}

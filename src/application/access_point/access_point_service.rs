//! 接入点应用服务 — application/access_point/
//!
//! 负责接入点 CRUD 操作的用例编排。
//! 核心职责：创建/更新/查询/删除接入点及其关联的账户池和模型路由网格。

use std::sync::Arc;

use uuid::Uuid;

use super::dto::{
    AccessPointResponse, AccountDto, CreateAccessPointRequest, ModelRoutingGridDto,
    ModelRoutingRowDto, UpdateAccessPointRequest,
};
use crate::domain::access_point::access_point_account::AccessPointAccount;
use crate::domain::access_point::model_routing_grid::{ModelRoutingGrid, ModelRoutingRow};
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::access_point::ShortCode;
use crate::domain::access_point::{AccessPoint, AccessPointEx};
use crate::domain::shared::AccessPointType;
use crate::domain::shared::Status;
use crate::shared::error::AppError;

/// 接入点应用服务
///
/// 编排接入点 CRUD 操作，包括账户池和模型路由网格的关联管理。
pub struct AccessPointService {
    access_point_repo: Arc<dyn AccessPointRepository>,
}

impl AccessPointService {
    pub fn new(access_point_repo: Arc<dyn AccessPointRepository>) -> Self {
        AccessPointService { access_point_repo }
    }

    fn base_response_fields(ap: &AccessPoint) -> AccessPointResponse {
        AccessPointResponse {
            id: ap.id,
            name: ap.name.clone(),
            api_type: ap.api_type.to_string(),
            short_code: ap.short_code.to_string(),
            accounts: Vec::new(),
            routing_strategy: ap.routing_strategy.as_str().to_string(),
            model_routing_grid: Self::grid_to_dto(&ap.model_routing_grid),
            access_url: format!("/ap/{}", ap.short_code),
            status: ap.status.to_string(),
            created_at: ap.created_at_utc(),
            updated_at: ap.updated_at_utc(),
        }
    }

    async fn load_accounts(
        repo: &Arc<dyn AccessPointRepository>,
        access_point_id: Uuid,
    ) -> Result<Vec<AccountDto>, AppError> {
        let rows = repo.find_accounts_by_access_point(access_point_id).await?;
        Ok(rows
            .into_iter()
            .map(|a| AccountDto {
                account_id: a.account_id,
                weight: Some(a.weight),
                priority: Some(a.priority),
            })
            .collect())
    }

    async fn to_response(
        repo: &Arc<dyn AccessPointRepository>,
        ap: &AccessPoint,
    ) -> AccessPointResponse {
        let accounts = Self::load_accounts(repo, ap.id).await.unwrap_or_default();
        let mut resp = Self::base_response_fields(ap);
        resp.accounts = accounts;
        resp
    }

    fn grid_to_dto(grid: &ModelRoutingGrid) -> ModelRoutingGridDto {
        ModelRoutingGridDto {
            provider_ids: grid.provider_ids.clone(),
            rows: grid
                .rows
                .iter()
                .map(|r| ModelRoutingRowDto {
                    source_model: r.source_model.clone(),
                    targets: r.targets.clone(),
                })
                .collect(),
        }
    }

    fn dto_to_grid(dto: &ModelRoutingGridDto) -> ModelRoutingGrid {
        ModelRoutingGrid {
            provider_ids: dto.provider_ids.clone(),
            rows: dto
                .rows
                .iter()
                .map(|r| ModelRoutingRow {
                    source_model: r.source_model.clone(),
                    targets: r.targets.clone(),
                })
                .collect(),
        }
    }

    /// 创建接入点
    ///
    /// 1. 校验短码唯一性（提供时）或自动生成
    /// 2. 解析接入类型（默认 Anthropic）
    /// 3. 保存接入点及关联账户池
    pub async fn create(
        &self,
        req: CreateAccessPointRequest,
        created_by: Uuid,
    ) -> Result<AccessPointResponse, AppError> {
        // 检查短码唯一性（如果提供了短码）
        let short_code = if let Some(ref code_str) = req.short_code {
            let code = ShortCode::new(code_str)?;
            let existing = self
                .access_point_repo
                .find_by_short_code(code.as_str())
                .await?;
            if existing.is_some() {
                return Err(AppError::Conflict(format!("短码 '{}' 已被使用", code)));
            }
            code
        } else {
            // 自动生成，并确保唯一性
            loop {
                let code = ShortCode::generate();
                let existing = self
                    .access_point_repo
                    .find_by_short_code(code.as_str())
                    .await?;
                if existing.is_none() {
                    break code;
                }
            }
        };

        // 解析接入类型，默认 Anthropic
        let api_type = match req.api_type {
            Some(ref t) => t.parse::<AccessPointType>()?,
            None => AccessPointType::Anthropic,
        };

        let mut access_point = AccessPoint::new(req.name, api_type, short_code, created_by);

        // 设置路由策略
        if let Some(ref strategy_str) = req.routing_strategy {
            access_point.set_routing_strategy(strategy_str.parse()?);
        }

        // 设置模型路由网格
        if let Some(ref grid_dto) = req.model_routing_grid {
            access_point.set_model_routing_grid(Self::dto_to_grid(grid_dto));
        }

        let saved = self.access_point_repo.save(&access_point).await?;

        // 保存账户池
        if let Some(ref accounts) = req.accounts {
            let account_entries: Vec<AccessPointAccount> = accounts
                .iter()
                .map(|a| {
                    AccessPointAccount::new(
                        a.account_id,
                        a.weight.unwrap_or(0),
                        a.priority.unwrap_or(0),
                    )
                })
                .collect();
            self.access_point_repo
                .save_accounts(saved.id, &account_entries)
                .await?;
        }

        Ok(Self::to_response(&self.access_point_repo, &saved).await)
    }

    /// 更新接入点
    ///
    /// 仅更新请求中提供的字段（名称、路由策略、模型网格、状态），
    /// 账户池全量替换。
    pub async fn update(
        &self,
        id: Uuid,
        req: UpdateAccessPointRequest,
    ) -> Result<AccessPointResponse, AppError> {
        let mut ap = self
            .access_point_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("接入点 {} 未找到", id)))?;

        if let Some(name) = req.name {
            ap.rename(name)?;
        }

        if let Some(ref api_type_str) = req.api_type {
            let api_type: AccessPointType = api_type_str.parse()?;
            ap.set_api_type(api_type);
        }

        if let Some(ref strategy_str) = req.routing_strategy {
            ap.set_routing_strategy(strategy_str.parse()?);
        }

        if let Some(ref grid_dto) = req.model_routing_grid {
            ap.set_model_routing_grid(Self::dto_to_grid(grid_dto));
        }

        if let Some(status_str) = req.status {
            let status: Status = status_str
                .parse()
                .map_err(|e: AppError| AppError::Validation(e.to_string()))?;
            ap.set_status(status);
        }

        let saved = self.access_point_repo.save(&ap).await?;

        // 更新账户池
        if let Some(ref accounts) = req.accounts {
            let account_entries: Vec<AccessPointAccount> = accounts
                .iter()
                .map(|a| {
                    AccessPointAccount::new(
                        a.account_id,
                        a.weight.unwrap_or(0),
                        a.priority.unwrap_or(0),
                    )
                })
                .collect();
            self.access_point_repo
                .save_accounts(saved.id, &account_entries)
                .await?;
        }

        Ok(Self::to_response(&self.access_point_repo, &saved).await)
    }

    /// 根据 ID 查询接入点（含账户池）
    pub async fn get_by_id(&self, id: Uuid) -> Result<AccessPointResponse, AppError> {
        let ap = self
            .access_point_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("接入点 {} 未找到", id)))?;

        Ok(Self::to_response(&self.access_point_repo, &ap).await)
    }

    /// 根据短码查询接入点（含账户池和聚合根信息）
    pub async fn get_by_short_code(
        &self,
        short_code: &str,
    ) -> Result<AccessPointResponse, AppError> {
        let ap: AccessPointEx = self
            .access_point_repo
            .find_by_short_code(short_code)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("接入点 {} 未找到", short_code)))?;

        let ap_inner = &ap.access_point;
        let accounts: Vec<AccountDto> = ap
            .accounts
            .into_iter()
            .map(|a| AccountDto {
                account_id: a.account_id,
                weight: Some(a.weight),
                priority: Some(a.priority),
            })
            .collect();

        let resp = AccessPointResponse {
            id: ap_inner.id,
            name: ap_inner.name.clone(),
            api_type: ap_inner.api_type.to_string(),
            short_code: ap_inner.short_code.to_string(),
            accounts,
            routing_strategy: ap_inner.routing_strategy.as_str().to_string(),
            model_routing_grid: Self::grid_to_dto(&ap_inner.model_routing_grid),
            access_url: format!("/ap/{}", ap_inner.short_code),
            status: ap_inner.status.to_string(),
            created_at: ap_inner.created_at_utc(),
            updated_at: ap_inner.updated_at_utc(),
        };

        Ok(resp)
    }

    /// 查询所有接入点列表
    pub async fn list_all(&self) -> Result<Vec<AccessPointResponse>, AppError> {
        let points = self.access_point_repo.find_all().await?;

        let mut results = Vec::with_capacity(points.len());
        for ap in &points {
            results.push(Self::to_response(&self.access_point_repo, ap).await);
        }

        Ok(results)
    }

    /// 删除接入点（级联删除关联的账户池和路由配置）
    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        self.access_point_repo.delete(id).await?;
        Ok(())
    }
}

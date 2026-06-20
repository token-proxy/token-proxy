//! 接入点实体与聚合根 — domain/access_point/
//!
//! 本模块定义 `AccessPoint`（SeaORM 实体映射 `access_points` 表）
//! 和 `AccessPointEx`（接入点 + 账户池的聚合根）。
//!
//! 聚合根 `AccessPointEx` 对外暴露路由、模型映射、会话粘滞等行为，
//! 内部委托给 `RoutingStrategy`、`ModelRoutingGrid` 等值对象。

use sea_orm::entity::prelude::*;

use crate::domain::access_point::access_point_account::AccessPointAccount;
use crate::domain::access_point::model_routing_grid::ModelRoutingGrid;
use crate::domain::access_point::routing_strategy::RoutingStrategy;
use crate::domain::access_point::short_code::ShortCode;
use crate::domain::shared::status::Status;
use crate::domain::shared::{AccessPointType, RequestSnapshot};
use crate::shared::error::AppError;
use chrono::{DateTime, FixedOffset, Utc};
use uuid::Uuid;

/// SeaORM 实体映射 access_points 表
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "access_points")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    /// 接入点 API 类型（Anthropic / OpenAI 等）
    pub api_type: AccessPointType,
    #[sea_orm(unique)]
    pub short_code: ShortCode,
    pub routing_strategy: RoutingStrategy,
    pub model_routing_grid: ModelRoutingGrid,
    pub status: Status,
    pub created_by: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── Model 基础行为 ────────────────────────────────────────────────

impl Model {
    /// 创建新的接入点实体，自动生成 ID 和时间戳
    pub fn new(
        name: String,
        api_type: AccessPointType,
        short_code: ShortCode,
        created_by: Uuid,
    ) -> Self {
        let now = Utc::now();
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        Model {
            id: Uuid::new_v4(),
            name,
            api_type,
            short_code,
            routing_strategy: RoutingStrategy::Priority,
            model_routing_grid: ModelRoutingGrid::default(),
            status: Status::Enabled,
            created_by,
            created_at: now.with_timezone(&offset),
            updated_at: now.with_timezone(&offset),
        }
    }

    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }

    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.updated_at.with_timezone(&Utc)
    }

    /// 重命名接入点，名称不可为空
    pub fn rename(&mut self, name: String) -> Result<(), AppError> {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::Validation("接入点名称不能为空".to_string()));
        }
        self.name = trimmed;
        self.touch();
        Ok(())
    }

    /// 设置路由策略
    pub fn set_routing_strategy(&mut self, strategy: RoutingStrategy) {
        self.routing_strategy = strategy;
        self.touch();
    }

    /// 设置模型路由网格
    pub fn set_model_routing_grid(&mut self, grid: ModelRoutingGrid) {
        self.model_routing_grid = grid;
        self.touch();
    }

    /// 设置状态
    pub fn set_status(&mut self, status: Status) {
        self.status = status;
        self.touch();
    }

    /// 更新 updated_at 为当前时间
    fn touch(&mut self) {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        self.updated_at = chrono::Utc::now().with_timezone(&offset);
    }
}

// ─── AccessPointEx 聚合根 ──────────────────────────────────────────

/// 接入点聚合根
///
/// 包含接入点实体和账户池。`Deref` 到 `Model` 以透明访问实体字段。
pub struct AccessPointEx {
    pub access_point: Model,
    pub accounts: Vec<AccessPointAccount>,
}

impl std::ops::Deref for AccessPointEx {
    type Target = Model;

    fn deref(&self) -> &Self::Target {
        &self.access_point
    }
}

impl AccessPointEx {
    /// 从实体和账户池构造聚合根
    pub fn from_model(access_point: Model, accounts: Vec<AccessPointAccount>) -> Self {
        AccessPointEx {
            access_point,
            accounts,
        }
    }

    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.access_point.created_at_utc()
    }

    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.access_point.updated_at_utc()
    }

    /// 验证接入点自身是否可用（不检查关联）
    pub fn validate_usable(&self) -> Result<(), AppError> {
        if !self.access_point.status.is_enabled() {
            return Err(AppError::Forbidden(format!(
                "接入点 '{}' 已被禁用",
                self.access_point.short_code
            )));
        }
        Ok(())
    }

    /// 检查账户池是否为空
    pub fn has_available_accounts(&self) -> bool {
        !self.accounts.is_empty()
    }

    /// 按路由策略排序账户池
    ///
    /// 委托给 `RoutingStrategy::sort_accounts()` 执行具体排序逻辑。
    pub fn sort_accounts(&mut self) {
        self.access_point
            .routing_strategy
            .sort_accounts(&mut self.accounts);
    }

    /// 将指定账户提到账户池最前（会话粘滞优先）
    ///
    /// 如果账户池中不存在该 account_id，则静默跳过。
    pub fn apply_session_affinity(&mut self, bound_account_id: Uuid) {
        if let Some(pos) = self
            .accounts
            .iter()
            .position(|a| a.account_id == bound_account_id)
        {
            let entry = self.accounts.remove(pos);
            self.accounts.insert(0, entry);
        }
    }

    /// 在模型路由网格中查找目标模型
    pub fn resolve_model(&self, requested_model: &str, provider_id: &Uuid) -> String {
        self.access_point
            .model_routing_grid
            .resolve_model(requested_model, provider_id)
    }

    /// 变换入站请求为上游请求
    pub fn transform_request_snapshot(
        &self,
        inbound: &RequestSnapshot,
        upstream_key: &str,
        provider_id: &Uuid,
    ) -> Result<RequestSnapshot, AppError> {
        let original_model = inbound.model().to_string();
        let mapped_model = self.resolve_model(&original_model, provider_id);

        let body = if mapped_model != original_model {
            inbound.replace_model_in_body(&mapped_model)
        } else {
            inbound.body().clone()
        };

        let headers = inbound.transform_headers(upstream_key);

        Ok(inbound.with_parts(headers, body, mapped_model))
    }

    /// 从模型路由网格中移除指定 Provider 的列
    /// 当接入点的某个 Provider 的所有账号都被移除后调用
    pub fn remove_provider_from_routing(&mut self, provider_id: &Uuid) {
        self.access_point
            .model_routing_grid
            .remove_provider_column(provider_id);
    }

    /// 同步 provider_ids 到路由网格
    pub fn sync_routing_providers(&mut self) {
        self.access_point.model_routing_grid.sync_providers();
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_access_point() -> Model {
        Model::new(
            "test".to_string(),
            AccessPointType::Anthropic,
            ShortCode::generate(),
            Uuid::new_v4(),
        )
    }

    fn make_access_point_ex() -> AccessPointEx {
        AccessPointEx::from_model(test_access_point(), vec![])
    }

    fn make_multi_provider_grid(pid1: Uuid, pid2: Uuid) -> ModelRoutingGrid {
        use crate::domain::access_point::model_routing_grid::{ModelRoutingRow, UNMATCHED_MODEL};
        use std::collections::HashMap;

        let mut row1 = ModelRoutingRow {
            source_model: "claude-sonnet-4-20250514".to_string(),
            targets: HashMap::new(),
        };
        row1.targets.insert(pid1, Some("claude-sonnet-4-map".to_string()));

        let mut row2 = ModelRoutingRow {
            source_model: "claude-*".to_string(),
            targets: HashMap::new(),
        };
        row2.targets.insert(pid1, Some("claude-prefix-map".to_string()));

        let mut row3 = ModelRoutingRow {
            source_model: UNMATCHED_MODEL.to_string(),
            targets: HashMap::new(),
        };
        row3
            .targets
            .insert(pid1, Some("fallback-model".to_string()));

        ModelRoutingGrid {
            provider_ids: vec![pid1, pid2],
            rows: vec![row1, row2, row3],
        }
    }

    #[test]
    fn test_resolve_model_exact_match() {
        let pid = Uuid::new_v4();
        let mut ap = make_access_point_ex();
        ap.access_point.model_routing_grid = make_multi_provider_grid(pid, Uuid::new_v4());
        let result = ap.resolve_model("claude-sonnet-4-20250514", &pid);
        assert_eq!(result, "claude-sonnet-4-map");
    }

    #[test]
    fn test_resolve_model_prefix_match() {
        let pid = Uuid::new_v4();
        let mut ap = make_access_point_ex();
        ap.access_point.model_routing_grid = make_multi_provider_grid(pid, Uuid::new_v4());
        let result = ap.resolve_model("claude-sonnet-4-20250601", &pid);
        assert_eq!(result, "claude-prefix-map");
    }

    #[test]
    fn test_resolve_model_unmatched_fallback() {
        let pid = Uuid::new_v4();
        let mut ap = make_access_point_ex();
        ap.access_point.model_routing_grid = make_multi_provider_grid(pid, Uuid::new_v4());
        let result = ap.resolve_model("gpt-4", &pid);
        assert_eq!(result, "fallback-model");
    }

    #[test]
    fn test_resolve_model_no_match_no_fallback() {
        let pid1 = Uuid::new_v4();
        let pid2 = Uuid::new_v4();
        let mut ap = make_access_point_ex();
        ap.access_point.model_routing_grid = make_multi_provider_grid(pid1, pid2);
        // pid2 has no fallback in the grid, should return the original
        let result = ap.resolve_model("gpt-4", &pid2);
        assert_eq!(result, "gpt-4");
    }

    #[test]
    fn test_validate_usable_enabled() {
        let ap = make_access_point_ex();
        assert!(ap.validate_usable().is_ok());
    }

    #[test]
    fn test_validate_usable_disabled() {
        let mut ap = make_access_point_ex();
        ap.access_point.status = Status::Disabled;
        let result = ap.validate_usable();
        assert!(result.is_err());
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[test]
    fn test_remove_provider_from_routing() {
        let pid1 = Uuid::new_v4();
        let pid2 = Uuid::new_v4();
        let mut ap = make_access_point_ex();
        ap.access_point.model_routing_grid = make_multi_provider_grid(pid1, pid2);
        ap.remove_provider_from_routing(&pid1);
        assert!(!ap.access_point.model_routing_grid.provider_ids.contains(&pid1));
        for row in &ap.access_point.model_routing_grid.rows {
            assert!(!row.targets.contains_key(&pid1));
        }
    }

    #[test]
    fn test_sync_routing_providers() {
        let pid = Uuid::new_v4();
        let mut ap = make_access_point_ex();
        ap.access_point.model_routing_grid.provider_ids.push(pid);
        ap.sync_routing_providers();
        assert!(ap.access_point.model_routing_grid.rows.is_empty());
        // When rows is empty, sync_providers is a no-op for rows
        // Add a row and test
        ap.access_point
            .model_routing_grid
            .rows
            .push(crate::domain::access_point::model_routing_grid::ModelRoutingRow {
                source_model: "test".to_string(),
                targets: std::collections::HashMap::new(),
            });
        ap.sync_routing_providers();
        assert!(ap.access_point.model_routing_grid.rows[0]
            .targets
            .contains_key(&pid));
    }

    #[test]
    fn test_access_point_ex_with_accounts() {
        let account_id = Uuid::new_v4();
        let accounts = vec![AccessPointAccount {
            account_id,
            weight: 1,
            priority: 0,
        }];
        let ap = AccessPointEx::from_model(test_access_point(), accounts);
        assert_eq!(ap.accounts.len(), 1);
        assert_eq!(ap.accounts[0].account_id, account_id);
        // Deref still works
        assert_eq!(ap.name, "test");
    }
}

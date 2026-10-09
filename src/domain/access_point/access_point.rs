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
use crate::domain::provider::provider::Model as Provider;
use crate::domain::shared::status::Status;
use crate::domain::shared::{AccessPointType, InboundRequest, UpstreamRequest, HOP_BY_HOP_HEADERS};
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

    /// 设置 API 类型（Anthropic / OpenAI）
    pub fn set_api_type(&mut self, api_type: AccessPointType) {
        self.api_type = api_type;
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

    /// 由入站请求构造上游请求（聚合根编排：URL 拼接 + 模型路由 + 协议适配）
    ///
    /// 工作步骤：
    /// 1. 从 Provider 选取与当前接入点 api_type 匹配的 base_url，与 remainder 拼接得到目标 URL
    /// 2. 通过模型路由网格将入站模型名解析为该 Provider 上的目标模型
    /// 3. 若发生模型映射，调用协议方法将新模型写回 body
    /// 4. 按 Provider 的上游兼容策略做能力降级（默认透明；见 `UpstreamCompat`）
    /// 5. 复制入站 headers 并过滤 hop-by-hop 头
    /// 6. 调用协议方法注入 API key
    ///
    /// 协议取自 `inbound.protocol`（请求级值对象，管道已按入站路径解析并做过家族校验），
    /// 聚合根本处不再重复推导，避免"同一事实两处判断"。
    ///
    /// 返回值中的 `normalized_roles` 是本次被降级的 role 数量（0 表示请求体未因兼容策略改动），
    /// 供调用方记日志与审计——降级必须是可观测的，不能静默发生。
    pub fn build_upstream_request(
        &self,
        inbound: &InboundRequest,
        provider: &Provider,
        upstream_key: &str,
        remainder: &str,
    ) -> Result<UpstreamRequest, AppError> {
        // URL 构造
        let base_url = provider.base_url_for(&self.access_point.api_type)?;
        let url = format!("{}/{}", base_url.trim_end_matches('/'), remainder);

        // 模型路由
        let mapped_model = self.resolve_model(&inbound.model, &provider.id);

        // body 变换（仅在确有变换需要时才克隆）
        let mut body = if mapped_model != inbound.model {
            inbound
                .protocol
                .replace_model_in_body(&inbound.body, &mapped_model)
        } else {
            inbound.body.clone()
        };

        // 上游能力降级：默认透明；仅在服务商显式开启时才改写（白名单式）
        let compat = provider.upstream_compat();
        let normalized_roles = if compat.normalize_legacy_roles {
            inbound.protocol.normalize_legacy_roles(&mut body)
        } else {
            0
        };

        // headers 变换：过滤 hop-by-hop + 注入 API key
        let mut headers = axum::http::HeaderMap::new();
        for (k, v) in inbound.headers.iter() {
            let key_lower = k.as_str().to_lowercase();
            if HOP_BY_HOP_HEADERS.contains(&key_lower.as_str()) {
                continue;
            }
            headers.insert(k.clone(), v.clone());
        }
        inbound.protocol.inject_api_key(&mut headers, upstream_key);

        Ok(UpstreamRequest {
            url,
            headers,
            body,
            mapped_model,
            normalized_roles,
        })
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
    use crate::domain::shared::ApiProtocol;

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
        row1.targets
            .insert(pid1, Some("claude-sonnet-4-map".to_string()));

        let mut row2 = ModelRoutingRow {
            source_model: "claude-sonnet-".to_string(),
            targets: HashMap::new(),
        };
        row2.targets
            .insert(pid1, Some("claude-prefix-map".to_string()));

        let mut row3 = ModelRoutingRow {
            source_model: UNMATCHED_MODEL.to_string(),
            targets: HashMap::new(),
        };
        row3.targets
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
        assert!(!ap
            .access_point
            .model_routing_grid
            .provider_ids
            .contains(&pid1));
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
        ap.access_point.model_routing_grid.rows.push(
            crate::domain::access_point::model_routing_grid::ModelRoutingRow {
                source_model: "test".to_string(),
                targets: std::collections::HashMap::new(),
            },
        );
        ap.sync_routing_providers();
        assert!(ap.access_point.model_routing_grid.rows[0]
            .targets
            .contains_key(&pid));
    }

    // ── 上游兼容策略（UpstreamCompat）集成 ──

    /// 构造一个启用 OpenAI base_url 的服务商
    fn openai_provider(normalize_legacy_roles: bool) -> Provider {
        let mut provider = Provider::new(
            "compat-test".to_string(),
            Some("https://relay.example.com".to_string()),
            None,
        )
        .expect("服务商构造应成功");
        provider.set_normalize_legacy_roles(normalize_legacy_roles);
        provider
    }

    /// 构造 OpenAI Chat 入站请求（body 含 developer role）
    fn openai_chat_inbound() -> InboundRequest {
        let body: serde_json::Value = serde_json::from_str(
            r#"{"model":"gpt-4o","messages":[{"role":"developer","content":"be terse"},{"role":"user","content":"hi"}]}"#,
        )
        .unwrap();
        InboundRequest {
            protocol: ApiProtocol::OpenAi,
            headers: axum::http::HeaderMap::new(),
            body,
            model: "gpt-4o".to_string(),
        }
    }

    #[test]
    fn compat_disabled_keeps_developer_role_transparent() {
        let ap = AccessPointEx::from_model(
            Model::new(
                "openai-ap".to_string(),
                AccessPointType::OpenAi,
                ShortCode::generate(),
                Uuid::new_v4(),
            ),
            vec![],
        );
        let provider = openai_provider(false);
        let inbound = openai_chat_inbound();

        let upstream = ap
            .build_upstream_request(&inbound, &provider, "sk-x", "v1/chat/completions")
            .expect("构造上游请求应成功");

        assert_eq!(upstream.normalized_roles, 0);
        assert_eq!(
            upstream.body["messages"][0]["role"],
            serde_json::Value::String("developer".into())
        );
    }

    #[test]
    fn compat_enabled_rewrites_developer_to_system() {
        let ap = AccessPointEx::from_model(
            Model::new(
                "openai-ap".to_string(),
                AccessPointType::OpenAi,
                ShortCode::generate(),
                Uuid::new_v4(),
            ),
            vec![],
        );
        let provider = openai_provider(true);
        let inbound = openai_chat_inbound();

        let upstream = ap
            .build_upstream_request(&inbound, &provider, "sk-x", "v1/chat/completions")
            .expect("构造上游请求应成功");

        assert_eq!(upstream.normalized_roles, 1);
        assert_eq!(
            upstream.body["messages"][0]["role"],
            serde_json::Value::String("system".into())
        );
        // 其余消息不受影响
        assert_eq!(
            upstream.body["messages"][1]["role"],
            serde_json::Value::String("user".into())
        );
        // URL 与鉴权仍正确注入
        assert_eq!(
            upstream.url,
            "https://relay.example.com/v1/chat/completions"
        );
        assert_eq!(
            upstream
                .headers
                .get(axum::http::header::AUTHORIZATION)
                .unwrap(),
            "Bearer sk-x"
        );
    }

    #[test]
    fn compat_enabled_is_noop_for_anthropic_protocol() {
        // Anthropic 协议没有 role 落差，即便开关打开也不应改写请求体
        let ap = make_access_point_ex();
        let mut provider = Provider::new(
            "compat-anthropic".to_string(),
            None,
            Some("https://api.anthropic.com".to_string()),
        )
        .expect("服务商构造应成功");
        provider.set_normalize_legacy_roles(true);

        let body: serde_json::Value = serde_json::from_str(
            r#"{"model":"claude-sonnet-4-20250514","messages":[{"role":"developer","content":"x"}]}"#,
        )
        .unwrap();
        let inbound = InboundRequest {
            protocol: ApiProtocol::Anthropic,
            headers: axum::http::HeaderMap::new(),
            body,
            model: "claude-sonnet-4-20250514".to_string(),
        };

        let upstream = ap
            .build_upstream_request(&inbound, &provider, "sk-a", "v1/messages")
            .expect("构造上游请求应成功");

        assert_eq!(upstream.normalized_roles, 0);
        assert_eq!(
            upstream.body["messages"][0]["role"],
            serde_json::Value::String("developer".into())
        );
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

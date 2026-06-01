use sea_orm::entity::prelude::*;

use crate::domain::value_objects::access_point_type::AccessPointType;
use crate::domain::value_objects::model_mapping::ModelMappingCollection;
use crate::domain::value_objects::short_code::ShortCode;
use crate::domain::value_objects::status::Status;
use chrono::{DateTime, FixedOffset, Utc};
use uuid::Uuid;

/// SeaORM 实体映射 access_points 表
///
/// 字段直接使用领域类型，通过 SeaORM 的 DeriveActiveEnum / DeriveValueType /
/// FromJsonQueryResult 自动完成数据库列和 Rust 类型之间的转换，无需手动 TryFrom/From。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "access_points")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub api_type: AccessPointType,
    #[sea_orm(unique)]
    pub short_code: ShortCode,
    pub provider_id: Uuid,
    pub account_id: Uuid,
    pub model_mappings: ModelMappingCollection,
    pub status: Status,
    pub created_by: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::provider::Entity",
        from = "Column::ProviderId",
        to = "super::provider::Column::Id"
    )]
    Provider,

    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::AccountId",
        to = "super::account::Column::Id"
    )]
    Account,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── Related trait 实现 ───────────────────────────────────────────

impl Related<super::user::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::User.def()
    }
}

impl Related<super::provider::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::Provider.def()
    }
}

impl Related<super::account::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::Account.def()
    }
}

/// 领域实体 AccessPoint
pub type AccessPoint = Model;

// ─── 领域行为 ──────────────────────────────────────────────────────

impl Model {
    /// 创建新的 AccessPoint
    pub fn new(
        name: String,
        api_type: AccessPointType,
        short_code: ShortCode,
        provider_id: Uuid,
        account_id: Uuid,
        created_by: Uuid,
    ) -> Self {
        let now = Utc::now();
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        Model {
            id: Uuid::new_v4(),
            name,
            api_type,
            short_code,
            provider_id,
            account_id,
            model_mappings: ModelMappingCollection::default(),
            status: Status::Enabled,
            created_by,
            created_at: now.with_timezone(&offset),
            updated_at: now.with_timezone(&offset),
        }
    }

    /// 解析最终使用的模型名称
    ///
    /// 依次查找映射规则并解析哨兵/兜底，优先级：
    /// 精确匹配 > 前缀匹配 > `__unmatched__` 规则 > Provider.default_model > 原始模型
    pub fn resolve_model(
        &self,
        requested_model: &str,
        default_model: Option<&str>,
    ) -> String {
        let mapped = if requested_model.is_empty() {
            None
        } else {
            self.model_mappings.map_model(requested_model)
        };
        ModelMappingCollection::resolve_final_model(
            mapped.as_deref(),
            default_model,
            requested_model,
        )
    }

    /// 获取 created_at 为 DateTime<Utc>
    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }

    /// 获取 updated_at 为 DateTime<Utc>
    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.updated_at.with_timezone(&Utc)
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::model_mapping::{
        ModelMapping, DEFAULT_MODEL_SENTINEL, UNMATCHED_MODEL_SENTINEL,
    };

    fn test_access_point() -> Model {
        Model::new(
            "test".to_string(),
            AccessPointType::Anthropic,
            ShortCode::generate(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        )
    }

    #[test]
    fn test_resolve_model_direct_match() {
        let mut ap = test_access_point();
        ap.model_mappings = vec![ModelMapping::new_exact(
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
        )]
        .into();
        assert_eq!(ap.resolve_model("gpt-4", None), "gpt-4-turbo");
    }

    #[test]
    fn test_resolve_model_unmatched_with_default() {
        let mut ap = test_access_point();
        ap.model_mappings = vec![ModelMapping::new_exact(
            UNMATCHED_MODEL_SENTINEL.to_string(),
            DEFAULT_MODEL_SENTINEL.to_string(),
        )]
        .into();
        assert_eq!(
            ap.resolve_model("unknown-model", Some("claude-sonnet")),
            "claude-sonnet"
        );
    }

    #[test]
    fn test_resolve_model_no_match_default_fallback() {
        let ap = test_access_point();
        assert_eq!(
            ap.resolve_model("unknown-model", Some("default-model")),
            "default-model"
        );
    }

    #[test]
    fn test_resolve_model_no_match_no_default() {
        let ap = test_access_point();
        assert_eq!(ap.resolve_model("my-model", None), "my-model");
    }

    #[test]
    fn test_resolve_model_empty_requested_with_default() {
        let ap = test_access_point();
        assert_eq!(ap.resolve_model("", Some("default")), "default");
    }

    #[test]
    fn test_resolve_model_empty_requested_no_default() {
        let ap = test_access_point();
        assert_eq!(ap.resolve_model("", None), "");
    }
}

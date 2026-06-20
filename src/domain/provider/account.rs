//! API 账号实体 — domain/provider/
//!
//! 定义 `Account`（SeaORM 实体映射 `accounts` 表）和 `DisabledReason` 枚举。
//! 账号属于 Provider，包含加密的 API key、禁用原因、可用性检查等行为。

use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::domain::shared::status::Status;
use crate::shared::error::AppError;

/// 账号禁用原因枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum DisabledReason {
    /// 管理员手动禁用
    #[sea_orm(string_value = "manual")]
    Manual,
    /// 触发频率限制
    #[sea_orm(string_value = "rate_limited")]
    RateLimited,
    /// 余额耗尽
    #[sea_orm(string_value = "balance_exhausted")]
    BalanceExhausted,
    /// 系统检测到故障
    #[sea_orm(string_value = "fault")]
    Fault,
}

impl fmt::Display for DisabledReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisabledReason::Manual => write!(f, "manual"),
            DisabledReason::RateLimited => write!(f, "rate_limited"),
            DisabledReason::BalanceExhausted => write!(f, "balance_exhausted"),
            DisabledReason::Fault => write!(f, "fault"),
        }
    }
}

/// SeaORM 实体映射 accounts 表
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub provider_id: Uuid,
    pub name: String,
    /// 加密后的 API key（AES-256-GCM）
    pub api_key_encrypted: Vec<u8>,
    /// API key 后 6 位，用于标识和显示
    pub api_key_suffix: String,
    pub disabled_reason: Option<DisabledReason>,
    /// 预计恢复时间（自动禁用时可恢复）
    pub available_at: Option<DateTimeWithTimeZone>,
    pub status: Status,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(belongs_to, from = "provider_id", to = "id")]
    pub provider: HasOne<super::provider::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// 创建新的 API 账号实体，状态为启用的、未过期
    pub fn new(provider_id: Uuid, name: String, api_key_suffix: String) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        let now = Utc::now().with_timezone(&offset);
        Model {
            id: Uuid::new_v4(),
            provider_id,
            name,
            api_key_encrypted: Vec::new(),
            api_key_suffix,
            disabled_reason: None,
            available_at: None,
            status: Status::Enabled,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }

    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.updated_at.with_timezone(&Utc)
    }

    /// 可用性检查：仅检查 status == Enabled
    pub fn is_available(&self) -> bool {
        self.status.is_enabled()
    }

    /// 是否被自动禁用（非 manual 类型，用于区分手动禁用和系统自动禁用）
    pub fn is_auto_disabled(&self) -> bool {
        matches!(
            self.disabled_reason,
            Some(DisabledReason::RateLimited)
                | Some(DisabledReason::BalanceExhausted)
                | Some(DisabledReason::Fault)
        )
    }

    /// 因故障原因禁用账户，同时设置预计恢复时间
    ///
    /// 封装 status / disabled_reason / available_at / updated_at 四个字段的原子更新，
    /// 替代外部逐字段赋值的做法。
    pub fn disable_for(
        &mut self,
        reason: DisabledReason,
        available_at: Option<DateTime<FixedOffset>>,
    ) {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        self.status = Status::Disabled;
        self.disabled_reason = Some(reason);
        self.available_at = available_at;
        self.updated_at = chrono::Utc::now().with_timezone(&offset);
    }

    /// 恢复账号：清除禁用原因和恢复时间，重置为启用状态
    ///
    /// 与 `disable_for` 对称，封装三个字段的原子更新和 updated_at 维护，
    /// 替代外部逐字段赋值的做法。
    pub fn recover(&mut self) {
        self.disabled_reason = None;
        self.available_at = None;
        self.status = Status::Enabled;
        self.touch();
    }

    /// 重命名账号，名称不可为空
    pub fn set_name(&mut self, name: String) -> Result<(), AppError> {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::Validation("账号名称不能为空".to_string()));
        }
        self.name = trimmed;
        self.touch();
        Ok(())
    }

    /// 更新 API Key 后缀
    pub fn update_api_key_suffix(&mut self, suffix: String) {
        self.api_key_suffix = suffix;
        self.touch();
    }

    /// 设置状态（启用/禁用），封装禁用时的决策逻辑
    ///
    /// - 启用时清除 disabled_reason 和 available_at
    /// - 禁用时仅在尚未有禁用原因时标记为手动禁用，避免覆盖自动检测原因
    pub fn set_status(&mut self, status: Status) {
        let is_enabled = status.is_enabled();
        self.status = status;
        if is_enabled {
            self.disabled_reason = None;
            self.available_at = None;
        } else if self.disabled_reason.is_none() {
            self.disabled_reason = Some(DisabledReason::Manual);
        }
        self.touch();
    }

    /// 更新 updated_at 为当前时间
    fn touch(&mut self) {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        self.updated_at = chrono::Utc::now().with_timezone(&offset);
    }
}

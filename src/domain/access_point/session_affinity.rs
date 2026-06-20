//! 会话粘滞 — domain/access_point/
//!
//! 定义 `SessionAffinity` 值对象（绑定 access_point_id + session_id → account_id）
//! 和 `SessionAffinityRepository` 仓储 trait。
//!
//! 确保同一会话的请求始终路由到同一上游账号（会话粘滞）。

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use uuid::Uuid;

use crate::shared::error::AppError;

/// 会话粘滞值对象
///
/// 记录 (access_point_id, session_id) → account_id 的绑定关系，
/// 确保同一会话的请求始终路由到同一上游账号。
#[derive(Debug, Clone)]
pub struct SessionAffinity {
    pub id: Uuid,
    pub access_point_id: Uuid,
    pub session_id: String,
    pub account_id: Uuid,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

/// 会话粘滞存储契约
#[async_trait]
pub trait SessionAffinityRepository: Send + Sync {
    /// 根据接入点和会话 ID 查询绑定
    async fn find_by_access_point_and_session(
        &self,
        access_point_id: Uuid,
        session_id: &str,
    ) -> Result<Option<SessionAffinity>, AppError>;

    /// 创建或更新绑定（upsert 语义）
    ///
    /// 若 (access_point_id, session_id) 已存在则更新 account_id + updated_at，
    /// 否则插入新记录。
    async fn upsert(
        &self,
        access_point_id: Uuid,
        session_id: &str,
        account_id: Uuid,
    ) -> Result<SessionAffinity, AppError>;

    /// 删除 updated_at 早于指定时长的过期绑定
    async fn delete_stale(&self, older_than: chrono::Duration) -> Result<u64, AppError>;
}

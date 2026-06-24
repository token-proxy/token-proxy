//! 词元用量仓储接口 — domain/log/
//!
//! 定义 `LogTokenUsageRepository` trait，提供词元用量的持久化契约。

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::log::LogTokenUsage;
use crate::shared::error::AppError;

/// 词元用量仓储接口
#[async_trait]
pub trait LogTokenUsageRepository: Send + Sync {
    /// 保存词元用量记录
    async fn save(&self, usage: &LogTokenUsage) -> Result<(), AppError>;

    /// 根据日志 ID 查找词元用量
    async fn find_by_log_id(&self, log_id: Uuid) -> Result<Option<LogTokenUsage>, AppError>;

    /// 根据会话 ID 查找该会话的所有词元用量记录
    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogTokenUsage>, AppError>;
}

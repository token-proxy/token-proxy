use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::provider::FaultConfig;

/// 服务商响应体
///
/// 包含服务商的基本配置和账号统计信息。
#[derive(Debug, Clone, Serialize)]
pub struct ProviderResponse {
    pub id: Uuid,
    pub name: String,
    /// OpenAI 兼容 API 基础地址
    pub openai_base_url: Option<String>,
    /// Anthropic API 基础地址
    pub anthropic_base_url: Option<String>,
    pub models: Vec<String>,
    /// 限流故障检测配置
    pub rate_limit_config: Option<FaultConfig>,
    /// 余额耗尽故障检测配置
    pub balance_exhausted_config: Option<FaultConfig>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// 关联账号总数
    pub account_count: Option<i64>,
    /// 可用账号数
    pub available_account_count: Option<i64>,
}

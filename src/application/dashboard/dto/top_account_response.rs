//! 上游账号 Token 消耗排行 DTO。

use serde::Serialize;
use uuid::Uuid;

/// 单个账号排行项
///
/// `account_name` / `provider_name` 均为 None 表示该账号已被删除，
/// 前端应降级显示 `已删除账号 · <uuid 前 8 位>`。
#[derive(Debug, Clone, Serialize)]
pub struct TopAccountItem {
    /// 账号 UUID
    pub account_id: Uuid,
    /// 账号名；None = 已删除
    pub account_name: Option<String>,
    /// 所属服务商 UUID
    pub provider_id: Option<Uuid>,
    /// 服务商名；None = 已删除或账号已删除
    pub provider_name: Option<String>,
    /// 当前禁用原因（字符串化的 DisabledReason），None = 正常可用
    pub disabled_reason: Option<String>,
    /// 输入 token 数
    pub input_tokens: i64,
    /// 输出 token 数
    pub output_tokens: i64,
    /// 缓存读取 token 数
    pub cache_read_tokens: i64,
    /// 缓存写入 token 数
    pub cache_creation_tokens: i64,
    /// 总 token 数（用于排序）
    pub total_tokens: i64,
}

/// 账号排行响应
#[derive(Debug, Clone, Serialize)]
pub struct TopAccountsResponse {
    /// 排行项数组（按 total_tokens 降序，最多 limit 条）
    pub items: Vec<TopAccountItem>,
}

//! 成员请求量排行 DTO。

use serde::Serialize;
use uuid::Uuid;

/// 单个成员排行项
///
/// `username` 和 `display_name` 均为 None 表示该用户已被删除，
/// 前端应降级显示 `已删除成员 · <uuid 前 8 位>`。
#[derive(Debug, Clone, Serialize)]
pub struct TopUserItem {
    /// 成员 UUID（即使用户已删除也保留）
    pub user_id: Uuid,
    /// 用户名；None = 已删除
    pub username: Option<String>,
    /// 显示名；None = 已删除或未设置
    pub display_name: Option<String>,
    /// 窗口内请求数
    pub request_count: i64,
    /// 窗口内词元总消耗
    pub total_tokens: i64,
}

/// 成员排行响应
#[derive(Debug, Clone, Serialize)]
pub struct TopUsersResponse {
    /// 排行项数组（按 request_count 降序，最多 limit 条）
    pub items: Vec<TopUserItem>,
}

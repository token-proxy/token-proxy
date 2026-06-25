//! Dashboard 接入点排行响应 DTO。

use serde::Serialize;
use uuid::Uuid;

/// 单个接入点排行项
///
/// `name` / `short_code` 均为 None 表示接入点已被删除，
/// 前端应降级展示 `已删除接入点 · <uuid 前 8 位>`。
#[derive(Debug, Clone, Serialize)]
pub struct TopAccessPointItem {
    /// 接入点 UUID（即使接入点已删除也保留）
    pub access_point_id: Uuid,
    /// 接入点名称（None 表示接入点已被删除，前端降级展示）
    pub name: Option<String>,
    /// 接入点短码（用于已删除时回退展示）
    pub short_code: Option<String>,
    /// 窗口内请求次数
    pub request_count: i64,
    /// 窗口内词元总消耗
    pub total_tokens: i64,
}

/// 接入点 Top N 响应
#[derive(Debug, Clone, Serialize)]
pub struct TopAccessPointsResponse {
    /// 排行项数组（按 total_tokens 降序，最多 limit 条）
    pub items: Vec<TopAccessPointItem>,
}

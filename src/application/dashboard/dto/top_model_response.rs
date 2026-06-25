//! Dashboard 模型排行响应 DTO。

use serde::Serialize;

/// 单个模型排行项
#[derive(Debug, Clone, Serialize)]
pub struct TopModelItem {
    /// 模型标识（如 `claude-sonnet-4-5`）
    pub model: String,
    /// 窗口内请求次数
    pub request_count: i64,
    /// 窗口内词元总消耗
    pub total_tokens: i64,
}

/// 模型 Top N 响应
#[derive(Debug, Clone, Serialize)]
pub struct TopModelsResponse {
    /// 排行项数组（按 total_tokens 降序，最多 limit 条）
    pub items: Vec<TopModelItem>,
}

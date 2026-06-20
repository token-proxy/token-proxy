//! 共享类型定义（共享层）
//!
//! 包括：时间戳别名、通用分页结果、分页请求参数解析。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// UTC 时间戳类型别名
pub type Timestamp = DateTime<Utc>;

/// 通用分页结果
///
/// 用于所有分页查询接口的响应格式。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    /// 当前页数据项列表
    pub items: Vec<T>,
    /// 总记录数
    pub total: u64,
    /// 当前页码（从 1 开始）
    pub page: u64,
    /// 每页大小
    pub page_size: u64,
}

/// 分页请求参数解析器
///
/// 提供 `limit()` 和 `offset()` 便捷方法。
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    /// 页码（从 1 开始，默认 1）
    pub page: Option<u64>,
    /// 每页大小（默认 20，最大 100）
    pub page_size: Option<u64>,
}

impl PaginationParams {
    pub fn limit(&self) -> u64 {
        self.page_size.unwrap_or(20).min(100)
    }

    /// 返回有效的偏移量（基于 page 和 limit 计算）
    pub fn offset(&self) -> u64 {
        let page = self.page.unwrap_or(1).max(1);
        (page - 1) * self.limit()
    }
}

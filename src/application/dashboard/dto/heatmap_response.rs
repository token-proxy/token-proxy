//! Dashboard 热力图响应 DTO。

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// 热力图查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct HeatmapQuery {
    /// IANA 时区名（如 `Asia/Shanghai`），由前端 `Intl.DateTimeFormat().resolvedOptions().timeZone` 提供
    pub tz: String,
}

/// 单日热力图单元格
#[derive(Debug, Clone, Serialize)]
pub struct HeatmapCellDto {
    /// 日期（按用户时区分桶后的本地日期）
    pub day: NaiveDate,
    /// 该日词元总量（决定上色档位）
    pub total_tokens: i64,
    /// 该日请求次数
    pub request_count: i64,
}

/// 热力图响应（固定 365 天）
#[derive(Debug, Clone, Serialize)]
pub struct HeatmapResponse {
    /// 热力图单元格序列（按日期升序，覆盖完整 365 天窗口）
    pub cells: Vec<HeatmapCellDto>,
}

use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TrendItem {
    /// 日期
    pub date: NaiveDate,
    /// 请求量
    pub count: u64,
}

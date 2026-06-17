use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TrendsQuery {
    /// 统计最近 N 天的趋势（默认 7 天，最大 365 天）
    pub days: Option<u64>,
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TopQuery {
    /// 返回 Top N 条记录（默认 10，最大 100）
    pub limit: Option<u64>,
}

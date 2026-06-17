use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TopModelItem {
    /// 模型名称
    pub model: String,
    /// 请求次数
    pub count: u64,
}

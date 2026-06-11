use axum::http::HeaderMap;
use serde_json::Value;

use crate::shared::error::AppError;

/// API 协议行为
///
/// 纯行为 trait，封装特定 API 的请求结构知识
///（模型字段位置、流式判断方式）。
/// 实现者为零大小类型，不持有任何数据。
pub trait ApiProtocol: Send + Sync {
    /// 从请求体中提取模型名称
    fn extract_model(&self, body: &Value) -> Result<String, AppError>;

    /// 替换请求体中的模型名称
    fn replace_model(&self, body: &mut Value, model: &str);

    /// 判断是否为流式请求
    fn is_streaming(&self, body: &Value, headers: &HeaderMap) -> bool;
}

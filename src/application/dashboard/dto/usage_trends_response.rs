//! 用量趋势端点响应 DTO。
//!
//! 该响应面向前端趋势图，按日返回请求数与词元分项，
//! 同时携带按模型拆分的词元用量用于模型消费面积图。

use chrono::{DateTime, Utc};
use serde::Serialize;

/// 单日单模型词元用量
#[derive(Debug, Clone, Serialize)]
pub struct ModelTokenUsageDto {
    /// 模型名（来自 log_metadata.model_mapped 或 model_original，回落 '(未知)'）
    pub model: String,
    /// 该模型在该桶内的总词元数
    pub total_tokens: i64,
}

/// 单个用量趋势桶
#[derive(Debug, Clone, Serialize)]
pub struct UsageTrendBucketDto {
    /// 桶起始时间（UTC）
    pub bucket_start: DateTime<Utc>,
    /// 该桶内请求数
    pub request_count: i64,
    /// 该桶内不重复会话数
    pub session_count: i64,
    /// 该桶内总词元数
    pub total_tokens: i64,
    /// 未命中缓存输入词元数
    pub input_tokens: i64,
    /// 输出词元数
    pub output_tokens: i64,
    /// 缓存创建输入词元数
    pub cache_creation_tokens: i64,
    /// 缓存命中输入词元数
    pub cache_read_tokens: i64,
    /// 思考词元数
    pub thinking_tokens: i64,
    /// 该桶内按模型拆分的词元用量
    pub per_model: Vec<ModelTokenUsageDto>,
}

/// 用量趋势响应
#[derive(Debug, Clone, Serialize)]
pub struct UsageTrendsResponse {
    /// 按日补齐后的趋势桶列表
    pub buckets: Vec<UsageTrendBucketDto>,
}

use serde::Deserialize;

use crate::domain::provider::FaultConfig;

/// 更新服务商请求体
///
/// 所有字段可选，仅提供的字段会被更新。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProviderRequest {
    /// 服务商名称
    pub name: Option<String>,
    /// OpenAI 兼容 API 基础地址
    pub openai_base_url: Option<String>,
    /// Anthropic API 基础地址
    pub anthropic_base_url: Option<String>,
    /// 支持的模型列表（全量替换）
    pub models: Option<Vec<String>>,
    /// 状态（enabled / disabled）
    pub status: Option<String>,
    /// 限流故障检测配置（全量替换）
    pub rate_limit_config: Option<FaultConfig>,
    /// 余额耗尽故障检测配置（全量替换）
    pub balance_exhausted_config: Option<FaultConfig>,
}

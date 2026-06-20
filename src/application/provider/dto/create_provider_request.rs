use serde::Deserialize;

use crate::domain::provider::FaultConfig;

/// 创建服务商请求体
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProviderRequest {
    /// 服务商名称（必填）
    pub name: String,
    /// OpenAI 兼容 API 基础地址（可选）
    pub openai_base_url: Option<String>,
    /// Anthropic API 基础地址（可选）
    pub anthropic_base_url: Option<String>,
    /// 限流故障检测配置（可选）
    pub rate_limit_config: Option<FaultConfig>,
    /// 余额耗尽故障检测配置（可选）
    pub balance_exhausted_config: Option<FaultConfig>,
}

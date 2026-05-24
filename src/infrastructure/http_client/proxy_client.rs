use std::time::Duration;

use axum::http::{HeaderMap, StatusCode};
use bytes::Bytes;
use reqwest::Client;

use crate::shared::error::AppError;

/// 上游 API 代理客户端
///
/// 封装 `reqwest::Client`，提供向 LLM 上游服务转发请求的能力。
/// 支持流式和非流式两种转发方式。
pub struct ProxyClient {
    client: Client,
}

impl ProxyClient {
    /// 创建新的代理客户端
    ///
    /// 使用默认连接池配置：
    /// - 连接超时: 30 秒
    /// - 请求超时: 300 秒
    /// - 保持活动连接
    /// - 默认请求头
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(32)
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .expect("创建 HTTP 客户端失败");

        ProxyClient { client }
    }

    /// 非流式转发请求
    ///
    /// 将请求转发到上游 API，返回响应状态码、响应头和响应体。
    ///
    /// * `base_url` - 上游 API 基础 URL
    /// * `api_key` - API 密钥
    /// * `body` - 请求体（JSON 字节）
    /// * `headers` - 需要透传的请求头
    pub async fn forward(
        &self,
        base_url: &str,
        api_key: &str,
        body: Bytes,
        headers: HeaderMap,
    ) -> Result<(StatusCode, HeaderMap, Bytes), AppError> {
        let mut req_builder = self
            .client
            .post(base_url)
            .header("authorization", format!("Bearer {}", api_key))
            .header("anthropic-version", "2023-06-01")
            .body(body);

        // 透传 Content-Type（如有）
        if let Some(content_type) = headers.get("content-type") {
            req_builder = req_builder.header("content-type", content_type.clone());
        }

        // 复制上游请求需要的业务头
        for (key, value) in headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if key_str.starts_with("x-") || key_str == "accept" {
                if key_str != "authorization" && key_str != "x-api-key" {
                    req_builder = req_builder.header(key, value.clone());
                }
            }
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("上游请求失败: {}", e)))?;

        let status = response.status();
        let resp_headers = response.headers().clone();
        let resp_body = response
            .bytes()
            .await
            .map_err(|e| AppError::Upstream(format!("读取上游响应失败: {}", e)))?;

        Ok((status, resp_headers, resp_body))
    }

    /// 流式转发请求，返回原始 `reqwest::Response` 以便逐块读取
    ///
    /// 适用于 SSE 等流式场景。调用方需自行处理 `response.bytes_stream()`。
    pub async fn forward_streaming(
        &self,
        base_url: &str,
        api_key: &str,
        body: Bytes,
        headers: HeaderMap,
    ) -> Result<reqwest::Response, AppError> {
        let mut req_builder = self
            .client
            .post(base_url)
            .header("authorization", format!("Bearer {}", api_key))
            .header("anthropic-version", "2023-06-01")
            .body(body);

        // 透传 Content-Type
        if let Some(content_type) = headers.get("content-type") {
            req_builder = req_builder.header("content-type", content_type.clone());
        }

        // 复制上游请求需要的业务头
        for (key, value) in headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if key_str.starts_with("x-") || key_str == "accept" || key_str == "accept-language" {
                if key_str != "authorization" && key_str != "x-api-key" {
                    req_builder = req_builder.header(key, value.clone());
                }
            }
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("上游流式请求失败: {}", e)))?;

        Ok(response)
    }
}

impl Default for ProxyClient {
    fn default() -> Self {
        Self::new()
    }
}

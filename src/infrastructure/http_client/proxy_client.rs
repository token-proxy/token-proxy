use std::time::Duration;

use axum::http::{HeaderMap, StatusCode};
use bytes::Bytes;
use reqwest::Client;

use crate::shared::error::AppError;

/// 上游 API 代理客户端
///
/// 纯 HTTP 执行器，不关心协议细节。
/// 接收预构造好的 URL、headers 和 body，直接发送请求并返回响应。
/// 协议特定的头构造由上层 `ProxyPipeline` 通过 `ApiProtocol` trait 完成。
pub struct ProxyClient {
    client: Client,
}

impl ProxyClient {
    /// 创建新的代理客户端
    ///
    /// - 连接超时: 30 秒
    /// - 请求超时: 300 秒
    /// - 保持活动连接
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
    /// 返回响应状态码、响应头和响应体。
    pub async fn forward(
        &self,
        url: &str,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<(StatusCode, HeaderMap, Bytes), AppError> {
        let response = self
            .build_request(url, headers, body)
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
    pub async fn forward_streaming(
        &self,
        url: &str,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<reqwest::Response, AppError> {
        self.build_request(url, headers, body)
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("上游流式请求失败: {}", e)))
    }

    /// 构建请求（共享逻辑）
    fn build_request(
        &self,
        url: &str,
        headers: HeaderMap,
        body: Bytes,
    ) -> reqwest::RequestBuilder {
        let mut req_builder = self.client.post(url).body(body);

        for (key, value) in headers.iter() {
            req_builder = req_builder.header(key, value.clone());
        }

        req_builder
    }
}

impl Default for ProxyClient {
    fn default() -> Self {
        Self::new()
    }
}

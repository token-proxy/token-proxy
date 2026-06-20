//! 上游 API 代理客户端
//!
//! 纯 HTTP 执行器，不关心 API 格式细节。
//! 接收预构造好的 URL、headers 和 body，直接发送请求并返回原始响应。

use std::time::Duration;

use axum::http::HeaderMap;
use bytes::Bytes;
use reqwest::Client;

use crate::shared::error::AppError;

/// 上游 API 代理客户端
///
/// 纯 HTTP 执行器，不关心 API 格式细节。
/// 接收预构造好的 URL、headers 和 body，直接发送请求并返回原始 `reqwest::Response`。
/// 调用方自行决定如何消费响应体（`.bytes()` 一次性读取 或 `.bytes_stream()` 逐块流式读取）。
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

    /// 转发请求到上游，返回原始响应
    ///
    /// 不做任何响应体处理——调用方根据 `is_streaming` 决定：
    /// - 非流式：调用 `.bytes().await` 一次性读取
    /// - 流式：调用 `.bytes_stream()` 逐块转发
    pub async fn forward(
        &self,
        url: &str,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<reqwest::Response, AppError> {
        let mut req_builder = self.client.post(url).body(body);

        for (key, value) in headers.iter() {
            req_builder = req_builder.header(key, value.clone());
        }

        req_builder
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("上游请求失败: {}", e)))
    }
}

impl Default for ProxyClient {
    fn default() -> Self {
        Self::new()
    }
}

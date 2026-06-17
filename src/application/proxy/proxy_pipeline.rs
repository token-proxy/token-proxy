use std::sync::Arc;
use std::time::Instant;

use axum::http::HeaderMap;
use bytes::Bytes;
use futures::StreamExt;
use uuid::Uuid;

use crate::application::log::LogService;
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::access_point::AccessPointEx;
use crate::domain::shared::EncryptionService;
use crate::infrastructure::http_client::ProcessedRequest;
use crate::infrastructure::http_client::ProxyClient;
use crate::infrastructure::http_client::ProxyLogger;
use crate::shared::error::AppError;

/// 代理转发管道
///
/// 编排一次 LLM API 代理转发的完整流程：
/// 聚合加载 → 解密 → 请求变换 → 上游转发 → 日志记录
pub struct ProxyPipeline {
    access_point_repo: Arc<dyn AccessPointRepository>,
    encryption_service: Arc<dyn EncryptionService>,
    proxy_client: Arc<ProxyClient>,
    log_service: Arc<LogService>,
}

impl ProxyPipeline {
    pub fn new(
        access_point_repo: Arc<dyn AccessPointRepository>,
        encryption_service: Arc<dyn EncryptionService>,
        proxy_client: Arc<ProxyClient>,
        log_service: Arc<LogService>,
    ) -> Self {
        ProxyPipeline {
            access_point_repo,
            encryption_service,
            proxy_client,
            log_service,
        }
    }

    /// 执行一次代理转发
    pub async fn execute(
        &self,
        short_code: &str,
        remainder: &str,
        headers: HeaderMap,
        body: String,
        user_id: Uuid,
    ) -> Result<axum::response::Response, AppError> {
        let access_point = self
            .access_point_repo
            .find_by_short_code(short_code)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("接入点 '{}' 未找到", short_code)))?;

        access_point.validate_usable()?;

        let upstream_key = access_point
            .decrypt_upstream_key(&*self.encryption_service)
            .await?;

        let processed =
            ProcessedRequest::prepare(&access_point, &upstream_key, remainder, headers, &body)?;

        self.handle(processed, &access_point, user_id).await
    }

    /// 统一处理流式/非流式转发
    async fn handle(
        &self,
        processed: ProcessedRequest,
        access_point: &AccessPointEx,
        user_id: Uuid,
    ) -> Result<axum::response::Response, AppError> {
        let body_bytes =
            Bytes::from(serde_json::to_string(processed.outbound.body()).unwrap_or_default());
        let start = Instant::now();

        let upstream_resp = self
            .proxy_client
            .forward(
                &processed.upstream_url,
                processed.outbound.headers().clone(),
                body_bytes,
            )
            .await?;

        let status = upstream_resp.status();
        let resp_headers = upstream_resp.headers().clone();

        // ── 构建响应 —— 过滤 hop-by-hop 头，其余全部透传 ──
        let hop_by_hop = [
            "transfer-encoding",
            "connection",
            "keep-alive",
            "proxy-authenticate",
            "proxy-authorization",
            "upgrade",
        ];
        let mut response_builder = axum::response::Response::builder().status(status);
        for (key, value) in resp_headers.iter() {
            let key_lower = key.as_str().to_lowercase();
            if !hop_by_hop.contains(&key_lower.as_str()) {
                response_builder = response_builder.header(key, value.clone());
            }
        }

        // ── 运输方式由响应头决定，不用请求特征预设 ──
        let is_sse = resp_headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("text/event-stream"))
            .unwrap_or(false);

        let logger = ProxyLogger::new(
            processed,
            access_point,
            user_id,
            status.as_u16(),
            start,
            resp_headers,
            self.log_service.clone(),
        );

        if is_sse {
            let byte_stream = upstream_resp.bytes_stream();

            let stream = async_stream::stream! {
                tokio::pin!(byte_stream);
                // logger 在闭包内：客户端断开时闭包被 drop → logger 被 drop → 自动标记 is_interrupted
                let mut logger = logger;
                while let Some(chunk) = byte_stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            logger.append_body(&bytes);
                            yield Ok(bytes);
                        }
                        Err(e) => {
                            yield Err(axum::Error::new(e));
                        }
                    }
                }
                logger.flush();
            };

            response_builder
                .body(axum::body::Body::from_stream(stream))
                .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)))
        } else {
            let resp_body = upstream_resp
                .bytes()
                .await
                .map_err(|e| AppError::Upstream(format!("读取上游响应失败: {}", e)))?;

            let mut logger = logger;
            logger.set_body(&resp_body);
            logger.flush();

            response_builder
                .body(axum::body::Body::from(resp_body))
                .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)))
        }
    }
}

use axum::http::HeaderMap;
use serde_json::Value;

use super::AccessPointType;
use crate::shared::error::AppError;

/// 请求快照值对象
///
/// 封装一次 HTTP 请求的核心数据（headers、body、model），
/// 同时携带特定 API 类型的格式知识（模型提取、流检测、header 变换等）。
///
/// body 以 `serde_json::Value` 存储，避免反复解析 JSON 字符串。
/// 不同 `AccessPointType` 对应不同变体，解析和变换逻辑内聚在各变体分支中。
#[derive(Clone, Debug)]
pub enum RequestSnapshot {
    Anthropic {
        headers: HeaderMap,
        body: Value,
        model: String,
    },
}

impl RequestSnapshot {
    /// 根据 API 类型解析入站请求，提取模型名称
    pub fn parse(
        api_type: &AccessPointType,
        headers: HeaderMap,
        body: String,
    ) -> Result<Self, AppError> {
        match api_type {
            AccessPointType::Anthropic => {
                let body_json: Value = serde_json::from_str(&body)
                    .map_err(|e| AppError::Validation(format!("请求体 JSON 解析失败: {}", e)))?;
                let model = body_json
                    .get("model")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("请求体缺少 model 字段".to_string()))?
                    .to_string();
                Ok(RequestSnapshot::Anthropic {
                    headers,
                    body: body_json,
                    model,
                })
            }
        }
    }

    pub fn model(&self) -> &str {
        match self {
            RequestSnapshot::Anthropic { model, .. } => model,
        }
    }

    pub fn headers(&self) -> &HeaderMap {
        match self {
            RequestSnapshot::Anthropic { headers, .. } => headers,
        }
    }

    pub fn body(&self) -> &Value {
        match self {
            RequestSnapshot::Anthropic { body, .. } => body,
        }
    }

    /// 判断是否为流式请求
    pub fn is_streaming(&self) -> bool {
        match self {
            RequestSnapshot::Anthropic { headers, body, .. } => {
                let is_stream_body = body
                    .get("stream")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let has_accept_sse = headers
                    .get("accept")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.contains("text/event-stream"))
                    .unwrap_or(false);

                is_stream_body || has_accept_sse
            }
        }
    }

    /// 提取会话标识
    pub fn session_id(&self) -> String {
        match self {
            RequestSnapshot::Anthropic { headers, .. } => headers
                .get("x-claude-code-session-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown")
                .to_string(),
        }
    }

    /// 替换请求体中的模型名称，返回修改后的 body Value
    pub fn replace_model_in_body(&self, model: &str) -> Value {
        match self {
            RequestSnapshot::Anthropic { body, .. } => {
                let mut body_value = body.clone();
                if let Some(obj) = body_value.as_object_mut() {
                    obj.insert("model".to_string(), Value::String(model.to_string()));
                }
                body_value
            }
        }
    }

    /// 将入站请求头变换为上游请求头
    ///
    /// 过滤 hop-by-hop 头，注入上游 API key。
    pub fn transform_headers(&self, upstream_key: &str) -> HeaderMap {
        match self {
            RequestSnapshot::Anthropic { headers, .. } => {
                let mut out = HeaderMap::new();

                for (key, value) in headers.iter() {
                    let key_str = key.as_str().to_lowercase();
                    if is_hop_by_hop_header(&key_str) {
                        continue;
                    }
                    out.insert(key.clone(), value.clone());
                }

                out.insert(
                    axum::http::header::AUTHORIZATION,
                    axum::http::HeaderValue::from_str(&format!("Bearer {}", upstream_key))
                        .expect("硬编码的 header 值"),
                );

                out
            }
        }
    }

    /// 用新的 headers、body、model 构造同类型快照
    pub fn with_parts(&self, headers: HeaderMap, body: Value, model: String) -> Self {
        match self {
            RequestSnapshot::Anthropic { .. } => RequestSnapshot::Anthropic {
                headers,
                body,
                model,
            },
        }
    }
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name,
        "host" | "transfer-encoding" | "content-length" | "connection"
    )
}

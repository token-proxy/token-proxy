use std::sync::Arc;

use uuid::Uuid;

use super::dto::{
    LogDetailFullResponse, LogDetailResponse, LogFilterParams, LogSummaryResponse, ProxyLogData,
    SessionContentItemResponse, SessionSummaryResponse, TokenUsageResponse,
};
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::log::LogTokenUsageRepository;
use crate::domain::log::{LogContent, LogMetadata, LogTokenUsage};
use crate::domain::log::{LogQuery, LogRepository, SessionQuery};
use crate::domain::user::UserRepository;
use crate::infrastructure::parsers::{claude_code_context, client_info, parsed_token_usage};
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

pub struct LogService {
    log_repo: Arc<dyn LogRepository>,
    token_usage_repo: Arc<dyn LogTokenUsageRepository>,
    user_repo: Arc<dyn UserRepository>,
    access_point_repo: Arc<dyn AccessPointRepository>,
}

impl LogService {
    pub fn new(
        log_repo: Arc<dyn LogRepository>,
        token_usage_repo: Arc<dyn LogTokenUsageRepository>,
        user_repo: Arc<dyn UserRepository>,
        access_point_repo: Arc<dyn AccessPointRepository>,
    ) -> Self {
        LogService {
            log_repo,
            token_usage_repo,
            user_repo,
            access_point_repo,
        }
    }

    fn to_token_usage_response(usage: &LogTokenUsage) -> TokenUsageResponse {
        TokenUsageResponse {
            id: usage.id,
            log_id: usage.log_id,
            session_id: usage.session_id.clone(),
            timestamp: usage.timestamp.to_utc(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_creation_input_tokens: usage.cache_creation_input_tokens,
            cache_read_input_tokens: usage.cache_read_input_tokens,
            thinking_tokens: usage.thinking_tokens,
            total_tokens: usage.total_tokens,
            raw_usage: usage.raw_usage.clone(),
            server_tool_usage: usage.server_tool_usage.clone(),
            cache_creation: usage.cache_creation.clone(),
        }
    }

    /// 创建日志条目（仅元数据），返回日志 ID
    pub async fn create_log_entry(&self, entry: &LogMetadata) -> Result<Uuid, AppError> {
        let saved = self.log_repo.save(entry).await?;
        Ok(saved.id)
    }

    /// 保存日志内容（请求/响应体）
    pub async fn save_log_content(&self, content: &LogContent) -> Result<(), AppError> {
        self.log_repo.save_content(content).await
    }

    /// 记录代理日志的核心入口
    ///
    /// 接收 ProxyLogData DTO，内部负责：
    /// - 从 DTO 构造 LogMetadata（LogMetadata）
    /// - HTTP 头解析（claude_code、user_agent）
    /// - 请求头脱敏 + JSON 序列化
    /// - Token 用量提取（从 SSE message_delta）
    /// - 原始内容存储（request_body、response_body）
    pub async fn record_proxy_log(&self, data: ProxyLogData) -> Result<Uuid, AppError> {
        // 解析 HTTP 头（会话 ID、Agent ID、conversation_source 等）
        let header_context = claude_code_context::parse_headers(&data.request_headers);

        // 解析 User-Agent 获取客户端信息
        let mut client_name = None;
        let mut client_version = None;
        let mut client_channel = None;
        let mut client_platform = None;
        if let Some(ref ua) = header_context.client_user_agent {
            let info = client_info::parse_user_agent(ua);
            client_name = info.client_name;
            client_version = info.client_version;
            client_channel = info.client_channel;
            client_platform = info.client_platform;
        }

        let entry = LogMetadata {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now().fixed_offset(),
            session_id: data.session_id.clone(),
            user_id: Some(data.user_id),
            access_point_id: Some(data.access_point_id),
            provider_id: Some(data.provider_id),
            account_id: Some(data.account_id),
            model_original: Some(data.model_original.clone()),
            model_mapped: Some(data.model_mapped.clone()),
            api_type: data.api_type.clone(),
            status_code: Some(data.status_code as i16),
            duration_ms: Some(data.duration_ms),
            is_interrupted: data.is_interrupted,
            error_message: data.error_message.clone(),
            request_index: 0,
            client_app: header_context.client_app,
            client_user_agent: header_context.client_user_agent,
            conversation_source: header_context.conversation_source,
            agent_id: header_context.agent_id,
            has_error: false,
            raw_content_available: true,
            client_name,
            client_version,
            client_channel,
            client_platform,
        };

        // ── 阶段 1：保存元数据（主表，失败则后续无意义）──
        let saved = self.log_repo.save(&entry).await?;

        // 请求头脱敏 + 序列化为 JSON
        let request_headers_json = headers_to_json(&data.request_headers);

        // 响应头序列化为 JSON
        let response_headers_json = response_headers_to_json(&data.resp_headers);

        // ── 阶段 2：保存原始内容（阶段 1 已成功，允许此阶段失败）──
        let content = LogContent {
            log_id: saved.id,
            request_headers: Some(request_headers_json),
            request_body: Some(data.request_body),
            response_body: Some(data.response_body.clone()),
            response_headers: Some(response_headers_json),
        };
        if let Err(e) = self.log_repo.save_content(&content).await {
            tracing::error!(error = %e, "日志内容写入失败");
        }

        // ── 阶段 3：提取 token 用量（独立步骤，失败不影响日志完整性）──
        match parsed_token_usage::parse_usage_from_response(&data.response_body) {
            Some(usage_data) => {
                let token_entry = LogTokenUsage {
                    id: Uuid::new_v4(),
                    log_id: saved.id,
                    session_id: saved.session_id.clone(),
                    timestamp: saved.timestamp,
                    user_id: saved.user_id,
                    access_point_id: saved.access_point_id,
                    provider_id: saved.provider_id,
                    account_id: saved.account_id,
                    model_original: saved.model_original.clone(),
                    model_mapped: saved.model_mapped.clone(),
                    conversation_source: Some(saved.conversation_source.clone()),
                    agent_id: saved.agent_id.clone(),
                    agent_type: None,
                    input_tokens: usage_data.input_tokens,
                    output_tokens: usage_data.output_tokens,
                    cache_creation_input_tokens: usage_data.cache_creation_input_tokens,
                    cache_read_input_tokens: usage_data.cache_read_input_tokens,
                    thinking_tokens: usage_data.thinking_tokens,
                    total_tokens: usage_data.total_tokens,
                    raw_usage: Some(usage_data.raw_usage),
                    server_tool_usage: None,
                    cache_creation: None,
                    created_at: chrono::Utc::now().fixed_offset(),
                };
                if let Err(e) = self.token_usage_repo.save(&token_entry).await {
                    tracing::error!(error = %e, "Token 用量写入失败");
                }
            }
            None => {
                tracing::warn!(
                    status_code = ?entry.status_code,
                    body_len = data.response_body.len(),
                    "未从响应体中解析到 token 用量"
                );
            }
        }

        Ok(saved.id)
    }

    pub async fn save_token_usage(&self, usage: &LogTokenUsage) -> Result<(), AppError> {
        self.token_usage_repo.save(usage).await
    }

    pub async fn get_session_token_usage(
        &self,
        session_id: &str,
    ) -> Result<Vec<LogTokenUsage>, AppError> {
        self.token_usage_repo.find_by_session_id(session_id).await
    }

    /// 分页查询日志摘要（含 token 用量摘要）
    pub async fn query_logs(
        &self,
        filters: LogFilterParams,
    ) -> Result<PaginatedResult<LogSummaryResponse>, AppError> {
        let page = filters.page.unwrap_or(1);
        let page_size = filters.page_size.unwrap_or(20);

        let log_query = LogQuery {
            session_id: filters.session_id,
            user_id: filters.user_id,
            access_point_id: filters.access_point_id,
            start_time: filters.start_time,
            end_time: filters.end_time,
            status_code: filters.status_code,
        };

        let result = self
            .log_repo
            .find_all_paginated_with_token_summary(page, page_size, &log_query)
            .await?;

        let items: Vec<LogSummaryResponse> = result
            .items
            .iter()
            .map(|item| LogSummaryResponse {
                id: item.entry.id,
                timestamp: item.entry.timestamp.to_utc(),
                session_id: item.entry.session_id.clone(),
                user_id: item.entry.user_id,
                access_point_id: item.entry.access_point_id,
                model_original: item.entry.model_original.clone(),
                model_mapped: item.entry.model_mapped.clone(),
                status_code: item.entry.status_code,
                duration_ms: item.entry.duration_ms,
                conversation_source: item.entry.conversation_source.clone(),
                agent_id: item.entry.agent_id.clone(),
                client_name: item.entry.client_name.clone(),
                client_version: item.entry.client_version.clone(),
                client_channel: item.entry.client_channel.clone(),
                client_platform: item.entry.client_platform.clone(),
                api_type: item.entry.api_type.clone(),
                token_input_tokens: item.input_tokens,
                token_output_tokens: item.output_tokens,
                token_total_tokens: item.total_tokens,
            })
            .collect();

        Ok(PaginatedResult {
            items,
            total: result.total,
            page: result.page,
            page_size: result.page_size,
        })
    }

    /// 获取日志详情（含请求/响应内容）
    pub async fn get_log_detail(&self, id: Uuid) -> Result<Option<LogDetailResponse>, AppError> {
        let entry = self.log_repo.find_by_id(id).await?;

        let content = self.log_repo.find_content_by_log_id(id).await?;

        match entry {
            Some(entry) => {
                let detail = LogDetailResponse {
                    id: entry.id,
                    timestamp: entry.timestamp.with_timezone(&chrono::Utc),
                    session_id: entry.session_id,
                    user_id: entry.user_id,
                    access_point_id: entry.access_point_id,
                    provider_id: entry.provider_id,
                    account_id: entry.account_id,
                    model_original: entry.model_original,
                    model_mapped: entry.model_mapped,
                    status_code: entry.status_code,
                    duration_ms: entry.duration_ms,
                    error_message: entry.error_message,
                    request_headers: content.as_ref().and_then(|c| c.request_headers.clone()),
                    request_body: content.as_ref().and_then(|c| c.request_body.clone()),
                    response_body: content.as_ref().and_then(|c| c.response_body.clone()),
                };
                Ok(Some(detail))
            }
            None => Ok(None),
        }
    }

    /// 获取日志完整详情
    pub async fn get_log_detail_full(
        &self,
        id: Uuid,
    ) -> Result<Option<LogDetailFullResponse>, AppError> {
        let result = self.log_repo.find_log_detail_full(id).await?;

        match result {
            Some((entry, content, usage)) => {
                let user_name = match entry.user_id {
                    Some(uid) => self
                        .user_repo
                        .find_by_id(uid)
                        .await
                        .ok()
                        .flatten()
                        .map(|u| u.display_name),
                    None => None,
                };
                let access_point_name = match entry.access_point_id {
                    Some(aid) => self
                        .access_point_repo
                        .find_by_id(aid)
                        .await
                        .ok()
                        .flatten()
                        .map(|ap| ap.name),
                    None => None,
                };

                let detail = LogDetailFullResponse {
                    id: entry.id,
                    timestamp: entry.timestamp.with_timezone(&chrono::Utc),
                    session_id: entry.session_id,
                    user_id: entry.user_id,
                    user_name,
                    access_point_id: entry.access_point_id,
                    access_point_name,
                    provider_id: entry.provider_id,
                    account_id: entry.account_id,
                    model_original: entry.model_original.unwrap_or_default(),
                    model_mapped: entry.model_mapped.unwrap_or_default(),
                    status_code: entry.status_code.unwrap_or(0) as i32,
                    duration_ms: entry.duration_ms.unwrap_or(0),
                    error_message: entry.error_message,
                    request_index: entry.request_index,
                    conversation_source: entry.conversation_source,
                    agent_id: entry.agent_id,
                    client_name: entry.client_name,
                    client_version: entry.client_version,
                    client_channel: entry.client_channel,
                    client_platform: entry.client_platform,
                    request_headers: content.request_headers.unwrap_or(serde_json::Value::Null),
                    request_body: content.request_body.unwrap_or(serde_json::Value::Null),
                    response_body: content.response_body.unwrap_or_default(),
                    token_input_tokens: usage.as_ref().map(|u| u.input_tokens),
                    token_output_tokens: usage.as_ref().map(|u| u.output_tokens),
                    token_cache_creation_input_tokens: usage
                        .as_ref()
                        .map(|u| u.cache_creation_input_tokens),
                    token_cache_read_input_tokens: usage
                        .as_ref()
                        .map(|u| u.cache_read_input_tokens),
                    token_thinking_tokens: usage.as_ref().map(|u| u.thinking_tokens),
                    token_total_tokens: usage.as_ref().map(|u| u.total_tokens),
                    token_raw_usage: usage.as_ref().and_then(|u| u.raw_usage.clone()),
                };
                Ok(Some(detail))
            }
            None => Ok(None),
        }
    }

    /// 获取某个会话的所有原始日志内容（含 token 用量）
    pub async fn get_session_contents(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionContentItemResponse>, AppError> {
        let logs = self.log_repo.find_by_session_id(session_id).await?;
        let mut items = Vec::with_capacity(logs.len());

        for entry in logs {
            let content = self
                .log_repo
                .find_content_by_log_id(entry.id)
                .await
                .ok()
                .flatten();
            let usage = self
                .token_usage_repo
                .find_by_log_id(entry.id)
                .await
                .ok()
                .flatten();

            if let Some(c) = content {
                items.push(SessionContentItemResponse {
                    log_id: entry.id,
                    request_index: entry.request_index,
                    timestamp: entry.timestamp.with_timezone(&chrono::Utc),
                    conversation_source: entry.conversation_source.clone(),
                    agent_id: entry.agent_id.clone(),
                    request_headers: c.request_headers.unwrap_or(serde_json::Value::Null),
                    request_body: c.request_body.unwrap_or(serde_json::Value::Null),
                    response_body: c.response_body.unwrap_or_default(),
                    token_usage: usage.as_ref().map(Self::to_token_usage_response),
                });
            }
        }

        Ok(items)
    }

    /// 获取会话摘要列表（含 token 用量汇总）
    pub async fn get_sessions(
        &self,
        filters: LogFilterParams,
    ) -> Result<PaginatedResult<SessionSummaryResponse>, AppError> {
        let page = filters.page.unwrap_or(1);
        let page_size = filters.page_size.unwrap_or(20);

        let session_query = SessionQuery {
            session_id: filters.session_id,
            user_id: filters.user_id,
            access_point_id: filters.access_point_id,
            start_time: filters.start_time,
            end_time: filters.end_time,
            status_code: filters.status_code,
        };

        let result = self
            .log_repo
            .find_sessions_paginated(page, page_size, &session_query)
            .await?;

        let items: Vec<SessionSummaryResponse> = result
            .items
            .iter()
            .map(|s| SessionSummaryResponse {
                session_id: s.session_id.clone(),
                user_id: s.user_id,
                access_point_id: s.access_point_id,
                start_time: s.start_time,
                request_count: s.request_count as u64,
                total_input_tokens: s.total_input_tokens,
                total_output_tokens: s.total_output_tokens,
                total_cache_creation_input_tokens: s.total_cache_creation_input_tokens,
                total_cache_read_input_tokens: s.total_cache_read_input_tokens,
                total_thinking_tokens: s.total_thinking_tokens,
                total_tokens: s.total_tokens,
            })
            .collect();

        Ok(PaginatedResult {
            items,
            total: result.total,
            page: result.page,
            page_size: result.page_size,
        })
    }

    pub async fn get_log_token_usage(
        &self,
        log_id: Uuid,
    ) -> Result<Option<TokenUsageResponse>, AppError> {
        Ok(self
            .token_usage_repo
            .find_by_log_id(log_id)
            .await?
            .as_ref()
            .map(Self::to_token_usage_response))
    }

    pub async fn get_session_token_usage_response(
        &self,
        session_id: &str,
    ) -> Result<Vec<TokenUsageResponse>, AppError> {
        let usages = self.token_usage_repo.find_by_session_id(session_id).await?;
        Ok(usages.iter().map(Self::to_token_usage_response).collect())
    }
}

// ─── 请求头脱敏（仅日志层使用） ───────────────────────────────────

fn headers_to_json(headers: &axum::http::HeaderMap) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (key, value) in headers {
        let name = key.as_str();
        let header_value = if is_sensitive_header(&name.to_lowercase()) {
            serde_json::Value::String("[REDACTED]".to_string())
        } else {
            serde_json::Value::String(value.to_str().unwrap_or("[non-UTF8]").to_string())
        };
        map.insert(name.to_string(), header_value);
    }
    serde_json::Value::Object(map)
}

fn is_sensitive_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization")
        || name.eq_ignore_ascii_case("x-api-key")
        || name.eq_ignore_ascii_case("proxy-authorization")
        || name.eq_ignore_ascii_case("cookie")
        || name.eq_ignore_ascii_case("set-cookie")
}

// ─── 响应头序列化（无需脱敏） ───────────────────────────────────

fn response_headers_to_json(headers: &axum::http::HeaderMap) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (key, value) in headers {
        let name = key.as_str();
        let header_value =
            serde_json::Value::String(value.to_str().unwrap_or("[non-UTF8]").to_string());
        map.insert(name.to_string(), header_value);
    }
    serde_json::Value::Object(map)
}

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::application::dto::log_dto::{
    ConversationEventResponse, LogDetailResponse, LogFilterParams, LogSummaryResponse,
    SessionSummaryResponse, TokenUsageResponse,
};
use crate::domain::entities::log_entry::{
    LogContent, LogConversationEvent, LogEntry, LogTokenUsage,
};
use crate::domain::repositories::log_conversation_event_repository::LogConversationEventRepository;
use crate::domain::repositories::log_repository::{LogQuery, LogRepository};
use crate::domain::repositories::log_token_usage_repository::LogTokenUsageRepository;
use crate::infrastructure::parsers::{claude_code, log_content};
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

pub struct LogService {
    log_repo: Arc<dyn LogRepository>,
    event_repo: Arc<dyn LogConversationEventRepository>,
    token_usage_repo: Arc<dyn LogTokenUsageRepository>,
}

impl LogService {
    pub fn new(
        log_repo: Arc<dyn LogRepository>,
        event_repo: Arc<dyn LogConversationEventRepository>,
        token_usage_repo: Arc<dyn LogTokenUsageRepository>,
    ) -> Self {
        LogService {
            log_repo,
            event_repo,
            token_usage_repo,
        }
    }

    fn to_event_response(event: &LogConversationEvent) -> ConversationEventResponse {
        ConversationEventResponse {
            id: event.id,
            log_id: event.log_id,
            session_id: event.session_id.clone(),
            timestamp: event.timestamp,
            request_index: event.request_index,
            event_index: event.event_index,
            parent_event_id: event.parent_event_id,
            parent_tool_use_id: event.parent_tool_use_id.clone(),
            source: event.source.clone(),
            role: event.role.clone(),
            event_type: event.event_type.clone(),
            agent_id: event.agent_id.clone(),
            agent_type: event.agent_type.clone(),
            tool_use_id: event.tool_use_id.clone(),
            tool_name: event.tool_name.clone(),
            title: event.title.clone(),
            content: event.content.clone(),
            content_preview: event.content_preview.clone(),
            thinking_content: event.thinking_content.clone(),
            hidden_content: event.hidden_content.clone(),
            display_payload: event.display_payload.clone(),
            confidence: event.confidence,
        }
    }

    fn to_token_usage_response(usage: &LogTokenUsage) -> TokenUsageResponse {
        TokenUsageResponse {
            id: usage.id,
            log_id: usage.log_id,
            session_id: usage.session_id.clone(),
            timestamp: usage.timestamp,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_creation_input_tokens: usage.cache_creation_input_tokens,
            cache_read_input_tokens: usage.cache_read_input_tokens,
            thinking_tokens: usage.thinking_tokens,
            total_tokens: usage.total_tokens,
            raw_usage: usage.raw_usage.clone(),
        }
    }

    fn to_summary(entry: &LogEntry) -> LogSummaryResponse {
        LogSummaryResponse {
            id: entry.id,
            timestamp: entry.timestamp,
            session_id: entry.session_id.clone(),
            user_id: entry.user_id,
            access_point_id: entry.access_point_id,
            model_original: entry.model_original.clone(),
            model_mapped: entry.model_mapped.clone(),
            status_code: entry.status_code,
            duration_ms: entry.duration_ms,
            conversation_source: entry.conversation_source.clone(),
            agent_id: entry.agent_id.clone(),
            agent_type: entry.agent_type.clone(),
            request_kind: entry.request_kind.clone(),
            primary_tool_name: entry.primary_tool_name.clone(),
            message_preview: entry.message_preview.clone(),
            message_full: entry.message_full.clone(),
            response_preview: entry.response_preview.clone(),
            has_thinking: entry.has_thinking,
            has_tool_use: entry.has_tool_use,
            raw_content_available: entry.raw_content_available,
        }
    }

    /// 创建日志条目（仅元数据），返回日志 ID
    pub async fn create_log_entry(&self, entry: &LogEntry) -> Result<Uuid, AppError> {
        let saved = self
            .log_repo
            .save(entry)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(saved.id)
    }

    /// 保存日志内容（请求/响应体）
    pub async fn save_log_content(&self, content: &LogContent) -> Result<(), AppError> {
        self.log_repo
            .save_content(content)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    pub async fn record_proxy_log(
        &self,
        mut entry: LogEntry,
        request_headers: serde_json::Value,
        request_body: serde_json::Value,
        response_body: String,
    ) -> Result<Uuid, AppError> {
        let header_context = claude_code::parse_headers(&request_headers);
        let parsed = log_content::parse(&request_body, &response_body);

        entry.client_session_id = header_context.client_session_id;
        entry.client_app = header_context.client_app;
        entry.client_user_agent = header_context.client_user_agent;
        entry.conversation_source = header_context.conversation_source;
        entry.agent_id = header_context.agent_id;
        entry.request_kind = parsed.request_kind;
        entry.primary_tool_name = parsed.primary_tool_name;
        entry.message_preview = parsed.message_preview;
        entry.message_full = parsed.message_full;
        entry.response_preview = parsed.response_preview;
        entry.has_thinking = parsed.has_thinking;
        entry.has_tool_use = parsed.has_tool_use;
        entry.has_error = entry.error_message.is_some();
        entry.agent_type = parsed.agent_type;
        entry.parent_agent_tool_use_id = parsed.parent_agent_tool_use_id;
        entry.raw_content_available = true;

        let saved = self.log_repo.save(&entry).await?;
        let content = LogContent {
            log_id: saved.id,
            request_headers,
            request_body,
            response_body,
        };
        self.log_repo.save_content(&content).await?;

        let events = parsed
            .events
            .into_iter()
            .enumerate()
            .map(|(idx, event)| {
                let content_preview = event
                    .content
                    .as_deref()
                    .map(|content| content.chars().take(200).collect::<String>());

                LogConversationEvent {
                    id: Uuid::new_v4(),
                    log_id: saved.id,
                    session_id: saved.session_id.clone(),
                    timestamp: saved.timestamp,
                    request_index: saved.request_index,
                    event_index: idx as i32,
                    parent_event_id: None,
                    parent_tool_use_id: saved.parent_agent_tool_use_id.clone(),
                    source: saved.conversation_source.clone(),
                    role: event.role,
                    event_type: event.event_type,
                    agent_id: saved.agent_id.clone(),
                    agent_type: saved.agent_type.clone(),
                    tool_use_id: event.tool_use_id,
                    tool_name: event.tool_name,
                    title: event.title,
                    content: event.content,
                    content_preview,
                    thinking_content: event.thinking_content,
                    hidden_content: None,
                    display_payload: event.display_payload,
                    confidence: event.confidence,
                    created_at: Utc::now(),
                }
            })
            .collect::<Vec<_>>();
        self.event_repo.save_many(&events).await?;

        if let Some(usage) = parsed.usage {
            self.token_usage_repo
                .save(&LogTokenUsage {
                    id: Uuid::new_v4(),
                    log_id: saved.id,
                    session_id: saved.session_id,
                    timestamp: saved.timestamp,
                    user_id: saved.user_id,
                    access_point_id: saved.access_point_id,
                    provider_id: saved.provider_id,
                    account_id: saved.account_id,
                    model_original: saved.model_original,
                    model_mapped: saved.model_mapped,
                    conversation_source: Some(saved.conversation_source),
                    agent_id: saved.agent_id,
                    agent_type: saved.agent_type,
                    input_tokens: usage.input_tokens,
                    output_tokens: usage.output_tokens,
                    cache_creation_input_tokens: usage.cache_creation_input_tokens,
                    cache_read_input_tokens: usage.cache_read_input_tokens,
                    thinking_tokens: usage.thinking_tokens,
                    total_tokens: usage.total_tokens,
                    raw_usage: Some(usage.raw_usage),
                    created_at: Utc::now(),
                })
                .await?;
        }

        Ok(saved.id)
    }

    pub async fn save_conversation_events(
        &self,
        events: &[LogConversationEvent],
    ) -> Result<(), AppError> {
        self.event_repo.save_many(events).await
    }

    pub async fn save_token_usage(&self, usage: &LogTokenUsage) -> Result<(), AppError> {
        self.token_usage_repo.save(usage).await
    }

    pub async fn get_session_events(
        &self,
        session_id: &str,
    ) -> Result<Vec<LogConversationEvent>, AppError> {
        self.event_repo.find_by_session_id(session_id).await
    }

    pub async fn get_session_token_usage(
        &self,
        session_id: &str,
    ) -> Result<Vec<LogTokenUsage>, AppError> {
        self.token_usage_repo.find_by_session_id(session_id).await
    }

    /// 分页查询日志摘要
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
            .find_all_paginated(page, page_size, &log_query)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let items: Vec<LogSummaryResponse> = result.items.iter().map(Self::to_summary).collect();

        Ok(PaginatedResult {
            items,
            total: result.total,
            page: result.page,
            page_size: result.page_size,
        })
    }

    /// 获取日志详情（含请求/响应内容）
    pub async fn get_log_detail(&self, id: Uuid) -> Result<Option<LogDetailResponse>, AppError> {
        let entry = self
            .log_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let content = self
            .log_repo
            .find_content_by_log_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match entry {
            Some(entry) => {
                let detail = LogDetailResponse {
                    id: entry.id,
                    timestamp: entry.timestamp,
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
                    request_headers: content.as_ref().map(|c| c.request_headers.clone()),
                    request_body: content.as_ref().map(|c| c.request_body.clone()),
                    response_body: content.map(|c| c.response_body),
                };
                Ok(Some(detail))
            }
            None => Ok(None),
        }
    }

    /// 获取会话摘要列表
    pub async fn get_sessions(
        &self,
        filters: LogFilterParams,
    ) -> Result<PaginatedResult<SessionSummaryResponse>, AppError> {
        // 按 session_id 分组聚合日志条目
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
            .find_all_paginated(page, page_size, &log_query)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 按 session_id 分组
        let mut session_map: std::collections::HashMap<String, Vec<LogEntry>> =
            std::collections::HashMap::new();

        for entry in &result.items {
            session_map
                .entry(entry.session_id.clone())
                .or_default()
                .push(entry.clone());
        }

        let mut sessions: Vec<SessionSummaryResponse> = session_map
            .into_iter()
            .map(|(session_id, entries)| {
                let first = entries.first();
                let request_count = entries.len() as u64;
                SessionSummaryResponse {
                    session_id,
                    user_id: first.and_then(|e| e.user_id),
                    access_point_id: first.and_then(|e| e.access_point_id),
                    start_time: first.map(|e| e.timestamp).unwrap_or_else(chrono::Utc::now),
                    request_count,
                    first_message: first.and_then(|e| e.model_original.clone()),
                }
            })
            .collect();

        // 按开始时间降序排列
        sessions.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        let total = sessions.len() as u64;
        let offset = ((page.max(1) - 1) * page_size) as usize;
        let paginated_items: Vec<SessionSummaryResponse> = sessions
            .into_iter()
            .skip(offset)
            .take(page_size as usize)
            .collect();

        Ok(PaginatedResult {
            items: paginated_items,
            total,
            page,
            page_size,
        })
    }

    /// 获取某个会话的结构化事件流
    pub async fn get_session_detail(
        &self,
        session_id: &str,
    ) -> Result<Vec<ConversationEventResponse>, AppError> {
        let events = self.event_repo.find_by_session_id(session_id).await?;
        Ok(events.iter().map(Self::to_event_response).collect())
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

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::application::dto::log_dto::{
    ConversationEventResponse, LogDetailFullResponse, LogDetailResponse, LogFilterParams,
    LogSummaryResponse, SessionSummaryResponse, TokenUsageResponse,
};
use crate::domain::entities::log_entry::{
    LogContent, LogConversationEvent, LogEntry, LogTokenUsage,
};
use crate::domain::repositories::log_conversation_event_repository::LogConversationEventRepository;
use crate::domain::repositories::log_repository::{LogQuery, LogRepository, SessionQuery};
use crate::domain::repositories::log_token_usage_repository::LogTokenUsageRepository;
use crate::infrastructure::parsers::{claude_code, create_parser, log_content, user_agent};
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
            content_type: event.content_type.clone(),
            signature: event.signature.clone(),
            tool_result_content: event.tool_result_content.clone(),
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
            server_tool_usage: usage.server_tool_usage.clone(),
            cache_creation: usage.cache_creation.clone(),
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
        api_type: &str,
    ) -> Result<Uuid, AppError> {
        let header_context = claude_code::parse_headers(&request_headers);

        // 解析 User-Agent 获取客户端信息
        if let Some(ref ua) = header_context.client_user_agent {
            let client_info = user_agent::parse_user_agent(ua);
            entry.client_name = client_info.client_name;
            entry.client_version = client_info.client_version;
            entry.client_channel = client_info.client_channel;
            entry.client_platform = client_info.client_platform;
        }

        // 使用 API 类型对应的解析器解析请求和响应
        let parser = create_parser(api_type);
        entry.parser_version = Some(parser.version().to_string());
        entry.api_type = api_type.to_string();

        let domain_result = parser
            .parse(&request_body, &response_body)
            .await
            .unwrap_or_else(|_| crate::domain::services::ParsedLogContent {
                parser_version: parser.version().to_string(),
                message_preview: None,
                message_full: None,
                request_kind: "messages".to_string(),
                primary_tool_name: None,
                response_preview: None,
                response_assistant_text: None,
                content_blocks: Vec::new(),
                thinking_content: None,
                has_thinking: false,
                has_tool_use: false,
            });

        // 将领域层解析结果转换为兼容层格式
        let events = log_content::build_events_from_domain(&domain_result, &request_body);
        let usage = log_content::parse_usage_from_response(&response_body);
        let (agent_type, parent_agent_tool_use_id) =
            log_content::extract_agent_info_from_sse(&response_body);

        // 如果没有从 SSE 提取到 agent_type，从 content_blocks 中尝试获取
        let agent_type = agent_type.or_else(|| {
            for block in &domain_result.content_blocks {
                if block.block_type == "tool_use"
                    && block.tool_name.as_deref() == Some("Agent")
                {
                    return domain_result
                        .content_blocks
                        .iter()
                        .find(|b| b.block_type == "tool_use")
                        .and(None); // 保留现有逻辑
                }
            }
            None
        });

        entry.client_session_id = header_context.client_session_id;
        entry.client_app = header_context.client_app;
        entry.client_user_agent = header_context.client_user_agent;
        entry.conversation_source = header_context.conversation_source;
        entry.agent_id = header_context.agent_id;
        entry.request_kind = Some(domain_result.request_kind);
        entry.primary_tool_name = domain_result.primary_tool_name;
        entry.message_preview = domain_result.message_preview;
        entry.message_full = domain_result.message_full;
        entry.response_preview = domain_result.response_preview;
        entry.has_thinking = domain_result.has_thinking;
        entry.has_tool_use = domain_result.has_tool_use;
        entry.has_error = entry.error_message.is_some();
        entry.agent_type = agent_type;
        entry.parent_agent_tool_use_id = parent_agent_tool_use_id;
        entry.raw_content_available = true;

        let saved = self.log_repo.save(&entry).await?;
        let content = LogContent {
            log_id: saved.id,
            request_headers,
            request_body,
            response_body,
        };
        self.log_repo.save_content(&content).await?;

        let conversation_events: Vec<LogConversationEvent> = events
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
                    content_type: None,
                    signature: None,
                    tool_result_content: None,
                    created_at: Utc::now(),
                }
            })
            .collect::<Vec<_>>();
        self.event_repo.save_many(&conversation_events).await?;

        if let Some(usage_data) = usage {
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
                    input_tokens: usage_data.input_tokens,
                    output_tokens: usage_data.output_tokens,
                    cache_creation_input_tokens: usage_data.cache_creation_input_tokens,
                    cache_read_input_tokens: usage_data.cache_read_input_tokens,
                    thinking_tokens: usage_data.thinking_tokens,
                    total_tokens: usage_data.total_tokens,
                    raw_usage: Some(usage_data.raw_usage),
                    server_tool_usage: None,
                    cache_creation: None,
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
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let items: Vec<LogSummaryResponse> = result
            .items
            .iter()
            .map(|item| LogSummaryResponse {
                id: item.entry.id,
                timestamp: item.entry.timestamp,
                session_id: item.entry.session_id.clone(),
                user_id: item.entry.user_id,
                access_point_id: item.entry.access_point_id,
                model_original: item.entry.model_original.clone(),
                model_mapped: item.entry.model_mapped.clone(),
                status_code: item.entry.status_code,
                duration_ms: item.entry.duration_ms,
                conversation_source: item.entry.conversation_source.clone(),
                agent_id: item.entry.agent_id.clone(),
                agent_type: item.entry.agent_type.clone(),
                request_kind: item.entry.request_kind.clone(),
                primary_tool_name: item.entry.primary_tool_name.clone(),
                message_preview: item.entry.message_preview.clone(),
                message_full: item.entry.message_full.clone(),
                response_preview: item.entry.response_preview.clone(),
                has_thinking: item.entry.has_thinking,
                has_tool_use: item.entry.has_tool_use,
                raw_content_available: item.entry.raw_content_available,
                parser_version: item.entry.parser_version.clone(),
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

    /// 获取日志完整详情（含客户端信息、token 用量等）
    pub async fn get_log_detail_full(
        &self,
        id: Uuid,
    ) -> Result<Option<LogDetailFullResponse>, AppError> {
        let result = self
            .log_repo
            .find_log_detail_full(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match result {
            Some((entry, content, usage)) => {
                // 解析请求体获取最后一条 user message 的文本
                let request_message_text = content
                    .request_body
                    .as_object()
                    .and_then(|obj| {
                        obj.get("messages")
                            .and_then(|v| v.as_array())
                            .and_then(|messages| {
                                messages
                                    .iter()
                                    .rev()
                                    .find(|m| {
                                        m.get("role").and_then(|r| r.as_str()) == Some("user")
                                    })
                                    .and_then(|m| m.get("content"))
                                    .and_then(|content_val| {
                                        content_val.as_str().map(|s| s.to_string()).or_else(|| {
                                            // 尝试从 content 数组中提取文本
                                            content_val.as_array().and_then(|blocks| {
                                                blocks
                                                    .iter()
                                                    .find(|b| {
                                                        b.get("type")
                                                            .and_then(|t| t.as_str())
                                                            == Some("text")
                                                    })
                                                    .and_then(|b| {
                                                        b.get("text")
                                                            .and_then(|t| t.as_str())
                                                            .map(|s| s.to_string())
                                                    })
                                            })
                                        })
                                    })
                            })
                    });

                // 从解析结果中提取 assistant 和 thinking 文本
                let parser = create_parser(&entry.api_type);
                let domain_result = parser
                    .parse(&content.request_body, &content.response_body)
                    .await
                    .ok();

                let response_assistant_text = domain_result
                    .as_ref()
                    .and_then(|r| r.response_assistant_text.clone());

                let response_thinking_text =
                    domain_result.as_ref().and_then(|r| r.thinking_content.clone());

                let detail = LogDetailFullResponse {
                    id: entry.id,
                    timestamp: entry.timestamp,
                    session_id: entry.session_id,
                    user_id: entry.user_id,
                    access_point_id: entry.access_point_id,
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
                    agent_type: entry.agent_type,
                    parser_version: entry.parser_version,
                    client_name: entry.client_name,
                    client_version: entry.client_version,
                    client_channel: entry.client_channel,
                    client_platform: entry.client_platform,
                    request_headers: content.request_headers,
                    request_body: content.request_body,
                    request_message_text,
                    response_body: content.response_body,
                    response_assistant_text,
                    response_thinking_text,
                    token_input_tokens: usage.as_ref().map(|u| u.input_tokens),
                    token_output_tokens: usage.as_ref().map(|u| u.output_tokens),
                    token_cache_creation_input_tokens: usage
                        .as_ref()
                        .map(|u| u.cache_creation_input_tokens),
                    token_cache_read_input_tokens: usage.as_ref().map(|u| u.cache_read_input_tokens),
                    token_thinking_tokens: usage.as_ref().map(|u| u.thinking_tokens),
                    token_total_tokens: usage.as_ref().map(|u| u.total_tokens),
                    token_raw_usage: usage.as_ref().and_then(|u| u.raw_usage.clone()),
                };
                Ok(Some(detail))
            }
            None => Ok(None),
        }
    }

    /// 获取会话摘要列表（含 token 用量汇总和第一条用户消息）
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
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let items: Vec<SessionSummaryResponse> = result
            .items
            .iter()
            .map(|s| SessionSummaryResponse {
                session_id: s.session_id.clone(),
                user_id: s.user_id,
                access_point_id: s.access_point_id,
                start_time: s.start_time,
                request_count: s.request_count as u64,
                first_message: None, // 由调用方填充
                total_input_tokens: s.total_input_tokens,
                total_output_tokens: s.total_output_tokens,
                total_cache_creation_input_tokens: s.total_cache_creation_input_tokens,
                total_cache_read_input_tokens: s.total_cache_read_input_tokens,
                total_thinking_tokens: s.total_thinking_tokens,
                total_tokens: s.total_tokens,
            })
            .collect();

        // 填充 first_message：从每条会话的第一条日志中解析最后一条 user message
        // 分批按 session_id 查询日志条目获取消息预览
        let items_with_messages: Vec<SessionSummaryResponse> = {
            let mut enhanced = Vec::with_capacity(items.len());
            for session in items {
                let first_message = if session.session_id.is_empty()
                    || session.session_id == "unknown"
                {
                    None
                } else {
                    // 从日志列表中取该 session 最早的条目，解析 message_full
                    let logs = self
                        .log_repo
                        .find_by_session_id(&session.session_id)
                        .await
                        .ok()
                        .unwrap_or_default();

                    logs.first().and_then(|log| {
                        // 使用 message_full 作为第一条消息预览
                        log.message_full.clone()
                    })
                };

                enhanced.push(SessionSummaryResponse {
                    first_message,
                    ..session
                });
            }
            enhanced
        };

        Ok(PaginatedResult {
            items: items_with_messages,
            total: result.total,
            page: result.page,
            page_size: result.page_size,
        })
    }

    /// 获取某个会话的结构化事件流（按 event_index 排序）
    pub async fn get_session_detail(
        &self,
        session_id: &str,
    ) -> Result<Vec<ConversationEventResponse>, AppError> {
        let mut events = self.event_repo.find_by_session_id(session_id).await?;
        // 按 event_index 升序排列
        events.sort_by_key(|e| e.event_index);
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
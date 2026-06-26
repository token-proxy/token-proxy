//! 日志应用服务 — application/log/
//!
//! 编排代理日志的二阶段写入（请求记录 → 原始内容），
//! 以及日志查询、会话摘要、日志详情、审计日志查询等功能。

use std::sync::Arc;

use tokio::sync::broadcast;
use uuid::Uuid;

use super::dto::{
    AuditLogFilterParams, AuditLogResponse, LogDetailFullResponse, LogDetailResponse,
    LogFilterParams, LogSummaryResponse, NewLogEvent, ProxyLogInput, SessionContentItemResponse,
    SessionSummaryResponse, TokenUsageResponse,
};
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::log::audit_action::AuditAction;
use crate::domain::log::audit_entity_type::AuditEntityType;
use crate::domain::log::repository_audit_log::{
    AuditLogQuery, AuditLogRepository as AuditLogRepoTrait,
};
use crate::domain::log::LogRequest;
use crate::domain::log::{LogContent, LogQuery, LogRepository, SessionQuery};
use crate::domain::shared::model_name::normalize_model_name;
use crate::domain::user::UserRepository;
use crate::infrastructure::parsers::{claude_code_context, parsed_token_usage};
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

/// 日志应用服务
///
/// 编排日志的二阶段写入和查询操作。
/// 写入流程：LogRequest（所有标量 + 词元合一） → LogContent（原始请求/响应体）。
pub struct LogService {
    log_repo: Arc<dyn LogRepository>,
    user_repo: Arc<dyn UserRepository>,
    access_point_repo: Arc<dyn AccessPointRepository>,
    audit_log_repo: Arc<dyn AuditLogRepoTrait>,
    /// 日志事件广播发送端（SSE 实时推送）
    event_tx: broadcast::Sender<NewLogEvent>,
}

impl LogService {
    pub fn new(
        log_repo: Arc<dyn LogRepository>,
        user_repo: Arc<dyn UserRepository>,
        access_point_repo: Arc<dyn AccessPointRepository>,
        audit_log_repo: Arc<dyn AuditLogRepoTrait>,
        event_tx: broadcast::Sender<NewLogEvent>,
    ) -> Self {
        LogService {
            log_repo,
            user_repo,
            access_point_repo,
            audit_log_repo,
            event_tx,
        }
    }

    /// 从 LogRequest 提取词元用量为 TokenUsageResponse
    fn to_token_usage_response(entry: &LogRequest) -> TokenUsageResponse {
        TokenUsageResponse {
            id: entry.id,
            log_id: entry.id,
            session_id: entry.session_id.clone(),
            timestamp: entry.timestamp_utc(),
            input_tokens: entry.input_tokens,
            output_tokens: entry.output_tokens,
            cache_creation_input_tokens: entry.cache_creation_input_tokens,
            cache_read_input_tokens: entry.cache_read_input_tokens,
            thinking_tokens: entry.thinking_tokens,
            total_tokens: entry.total_tokens,
            raw_usage: entry.raw_usage.clone(),
            server_tool_usage: entry.server_tool_usage.clone(),
            cache_creation: entry.cache_creation.clone(),
        }
    }

    /// 保存日志内容（请求/响应体）
    pub async fn save_log_content(&self, content: &LogContent) -> Result<(), AppError> {
        self.log_repo.save_content(content).await
    }

    /// 记录代理日志的核心入口（二阶段写入）
    ///
    /// 接收 `ProxyLogInput` DTO，内部负责：
    /// - 词元用量提取（前置，从 SSE message_delta）
    /// - 模型名称规范化（model_normalized 列）
    /// - HTTP 头解析（claude_code、user_agent）
    /// - 请求头脱敏 + JSON 序列化
    /// - 标量字段一体写入 LogRequest
    /// - 大体积内容异步写入 LogContent
    #[tracing::instrument(skip_all, fields(user_id = %data.user_id, access_point_id = %data.access_point_id, status_code = %data.status_code))]
    pub async fn record_proxy_log(&self, data: ProxyLogInput) -> Result<Uuid, AppError> {
        // 解析 HTTP 头（会话 ID、Agent ID、conversation_source 等）
        let header_context = claude_code_context::parse_headers(&data.request_headers);

        // 从 User-Agent 提取版本号（斜杠 '/' 后、第一个空白字符前的部分）
        let client_version = header_context.client_user_agent.as_ref().and_then(|ua| {
            ua.split_once('/')
                .and_then(|(_, rest)| rest.split_once(' ').map(|(ver, _)| ver))
                .map(|v| v.to_string())
        });

        // session_id 为 None 时存为 "unknown" 字符串
        let session_id_for_db = data
            .session_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        // ── 词元用量解析（前置，失败不阻塞）──
        let usage_data = parsed_token_usage::parse_usage_from_response(&data.response_body);

        // ── 模型名称规范化 ──
        let model_normalized = normalize_model_name(&data.model_mapped);

        // ── 阶段 1：构造并保存 LogRequest（所有标量 + 词元合一）──
        let entry = LogRequest {
            id: Uuid::new_v4(),
            timestamp: data.timestamp,
            session_id: session_id_for_db,
            user_id: Some(data.user_id),
            access_point_id: Some(data.access_point_id),
            provider_id: Some(data.provider_id),
            account_id: Some(data.account_id),
            model_original: Some(data.model_original.clone()),
            model_mapped: Some(data.model_mapped.clone()),
            model_normalized,
            api_type: data.api_type.clone(),
            client_type: data.client_type.clone(),
            status_code: Some(data.status_code as i16),
            duration_ms: Some(data.duration_ms),
            is_interrupted: data.is_interrupted,
            error_message: data.error_message.clone(),
            client_user_agent: header_context.client_user_agent,
            conversation_source: header_context.conversation_source,
            agent_id: header_context.agent_id,
            agent_type: None,
            has_error: false,
            client_version,
            input_tokens: usage_data.as_ref().map_or(0, |u| u.input_tokens),
            output_tokens: usage_data.as_ref().map_or(0, |u| u.output_tokens),
            cache_creation_input_tokens: usage_data
                .as_ref()
                .map_or(0, |u| u.cache_creation_input_tokens),
            cache_read_input_tokens: usage_data.as_ref().map_or(0, |u| u.cache_read_input_tokens),
            thinking_tokens: usage_data.as_ref().map_or(0, |u| u.thinking_tokens),
            total_tokens: usage_data.as_ref().map_or(0, |u| u.total_tokens),
            raw_usage: usage_data
                .as_ref()
                .map(|u| u.raw_usage.clone().into())
                .unwrap_or_default(),
            server_tool_usage: None,
            cache_creation: None,
            created_at: chrono::Utc::now().fixed_offset(),
        };

        if usage_data.is_none() {
            tracing::warn!(
                status_code = %data.status_code,
                body_len = data.response_body.len(),
                "未从响应体中解析到词元用量"
            );
        }

        let saved = self.log_repo.save(&entry).await?;

        // ── 阶段 2：保存原始内容（请求/响应体，磁盘重数据）──
        let request_headers_json = headers_to_json(&data.request_headers);
        let response_headers_json = response_headers_to_json(&data.resp_headers);

        let req_body_type = if data.request_body.is_object() {
            "object"
        } else if data.request_body.is_null() {
            "null"
        } else {
            "other"
        };
        let req_headers_count = data.request_headers.len();
        let resp_headers_count = data.resp_headers.len();
        let resp_body_len = data.response_body.len();

        let content = LogContent {
            log_id: saved.id,
            timestamp: data.timestamp,
            request_headers: Some(request_headers_json),
            request_body: Some(data.request_body),
            response_body: Some(data.response_body.clone()),
            response_headers: Some(response_headers_json),
        };
        if let Err(e) = self.log_repo.save_content(&content).await {
            tracing::error!(
                error = %e,
                log_id = %saved.id,
                status_code = %data.status_code,
                ts = %data.timestamp,
                req_headers_count,
                req_body_type,
                resp_headers_count,
                resp_body_len,
                "日志内容写入失败"
            );
        }

        // ─── 广播新日志事件（SSE 实时推送）───
        let event = NewLogEvent {
            log_id: saved.id,
            timestamp: saved.timestamp_utc(),
            session_id: saved.session_id.clone(),
            api_type: saved.api_type.clone(),
            user_id: data.user_id,
            access_point_id: data.access_point_id,
        };
        let _ = self.event_tx.send(event);

        Ok(saved.id)
    }

    /// 分页查询日志摘要（LogRequest 自含词元字段，无需联表）
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
            provider_id: filters.provider_id,
            account_id: filters.account_id,
            is_interrupted: filters.is_interrupted,
        };

        let result = self
            .log_repo
            .find_all_paginated(page, page_size, &log_query)
            .await?;

        let items: Vec<LogSummaryResponse> = result
            .items
            .iter()
            .map(|lr| LogSummaryResponse {
                id: lr.id,
                timestamp: lr.timestamp_utc(),
                session_id: lr.session_id.clone(),
                user_id: lr.user_id,
                access_point_id: lr.access_point_id,
                provider_id: lr.provider_id,
                account_id: lr.account_id,
                model_original: lr.model_original.clone(),
                model_mapped: lr.model_mapped.clone(),
                status_code: lr.status_code,
                duration_ms: lr.duration_ms,
                is_interrupted: lr.is_interrupted,
                conversation_source: lr.conversation_source.clone(),
                agent_id: lr.agent_id.clone(),
                client_version: lr.client_version.clone(),
                api_type: lr.api_type.clone(),
                token_input_tokens: Some(lr.input_tokens),
                token_output_tokens: Some(lr.output_tokens),
                token_cache_creation_input_tokens: Some(lr.cache_creation_input_tokens),
                token_cache_read_input_tokens: Some(lr.cache_read_input_tokens),
                token_thinking_tokens: Some(lr.thinking_tokens),
                token_total_tokens: Some(lr.total_tokens),
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
                    timestamp: entry.timestamp_utc(),
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
    ///
    /// LogRequest 自含词元字段，无需再单独查询 token_usage。
    pub async fn get_log_detail_full(
        &self,
        id: Uuid,
    ) -> Result<Option<LogDetailFullResponse>, AppError> {
        let result = self.log_repo.find_log_detail_full(id).await?;

        match result {
            Some((entry, content)) => {
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
                    timestamp: entry.timestamp_utc(),
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
                    conversation_source: entry.conversation_source.clone(),
                    agent_id: entry.agent_id,
                    client_version: entry.client_version,
                    api_type: Some(entry.api_type.clone()),
                    request_headers: content.request_headers.unwrap_or(serde_json::Value::Null),
                    response_headers: content.response_headers.unwrap_or(serde_json::Value::Null),
                    request_body: content.request_body.unwrap_or(serde_json::Value::Null),
                    response_body: content.response_body.unwrap_or_default(),
                    token_input_tokens: Some(entry.input_tokens),
                    token_output_tokens: Some(entry.output_tokens),
                    token_cache_creation_input_tokens: Some(entry.cache_creation_input_tokens),
                    token_cache_read_input_tokens: Some(entry.cache_read_input_tokens),
                    token_thinking_tokens: Some(entry.thinking_tokens),
                    token_total_tokens: Some(entry.total_tokens),
                    token_raw_usage: entry.raw_usage.clone(),
                };
                Ok(Some(detail))
            }
            None => Ok(None),
        }
    }

    /// 获取某个会话的所有原始日志内容（含词元用量）
    ///
    /// LogRequest 自含词元字段，无需单独查询 token_usage 表。
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

            if let Some(c) = content {
                items.push(SessionContentItemResponse {
                    log_id: entry.id,
                    timestamp: entry.timestamp_utc(),
                    conversation_source: entry.conversation_source.clone(),
                    agent_id: entry.agent_id.clone(),
                    api_type: entry.api_type.clone(),
                    request_headers: c.request_headers.unwrap_or(serde_json::Value::Null),
                    request_body: c.request_body.unwrap_or(serde_json::Value::Null),
                    response_body: c.response_body.unwrap_or_default(),
                    token_usage: Some(Self::to_token_usage_response(&entry)),
                });
            }
        }

        Ok(items)
    }

    /// 获取会话摘要列表（含词元用量汇总）
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

    /// 查询单条日志的词元用量
    pub async fn get_log_token_usage(
        &self,
        log_id: Uuid,
    ) -> Result<Option<TokenUsageResponse>, AppError> {
        Ok(self
            .log_repo
            .find_by_id(log_id)
            .await?
            .as_ref()
            .map(Self::to_token_usage_response))
    }

    /// 查询某个会话的词元用量（以响应 DTO 格式返回）
    pub async fn get_session_token_usage_response(
        &self,
        session_id: &str,
    ) -> Result<Vec<TokenUsageResponse>, AppError> {
        let entries = self.log_repo.find_by_session_id(session_id).await?;
        Ok(entries.iter().map(Self::to_token_usage_response).collect())
    }

    // ─── 审计日志查询 ──────────────────────────────────────────────────

    /// 分页查询审计日志，支持按操作类型、实体类型、操作者、时间范围筛选。
    #[tracing::instrument(skip(self), fields(page = filters.page.unwrap_or(1), page_size = filters.page_size.unwrap_or(20)))]
    pub async fn query_audit_logs(
        &self,
        filters: AuditLogFilterParams,
    ) -> Result<PaginatedResult<AuditLogResponse>, AppError> {
        let page = filters.page.unwrap_or(1);
        let page_size = filters.page_size.unwrap_or(20);

        // 1. 解析 action 逗号分隔字符串 → Vec<AuditAction>
        let actions = filters
            .action
            .as_ref()
            .map(|s| parse_action_list(s))
            .transpose()?;

        // 2. 解析 entity_type 逗号分隔字符串 → Vec<AuditEntityType>
        let entity_types = filters
            .entity_type
            .as_ref()
            .map(|s| parse_entity_type_list(s))
            .transpose()?;

        // 3. 构造领域查询对象
        let query = AuditLogQuery {
            actions,
            entity_types,
            operator_id: filters.operator_id,
            operator_type: filters.operator_type,
            start_time: filters.start_time,
            end_time: filters.end_time,
        };

        // 4. 调用仓储查询
        let result = self
            .audit_log_repo
            .find_all_paginated_with_username(page, page_size, &query)
            .await?;

        // 5. 映射到响应 DTO
        let items: Vec<AuditLogResponse> = result
            .items
            .iter()
            .map(|item| AuditLogResponse {
                id: item.log.id,
                timestamp: item.log.timestamp_utc(),
                operator_id: item.log.operator_id,
                operator_type: item.log.operator_type.clone(),
                operator_name: item.username.clone(),
                action: item.log.action.clone(),
                entity_type: item.log.entity_type.clone(),
                entity_id: item.log.entity_id,
                details: item.log.details.clone(),
            })
            .collect();

        Ok(PaginatedResult {
            items,
            total: result.total,
            page: result.page,
            page_size: result.page_size,
        })
    }
}

// ─── 请求头脱敏（敏感字段替换为 [REDACTED]） ───────────────────────────────────

/// 将 HeaderMap 序列化为 JSON，对敏感字段自动脱敏
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

/// 判断是否为敏感头（需要脱敏的字段）
fn is_sensitive_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization")
        || name.eq_ignore_ascii_case("x-api-key")
        || name.eq_ignore_ascii_case("proxy-authorization")
        || name.eq_ignore_ascii_case("cookie")
        || name.eq_ignore_ascii_case("set-cookie")
}

// ─── 响应头序列化（无需脱敏） ───────────────────────────────────

/// 将响应 HeaderMap 序列化为 JSON（响应头不含敏感信息，直接序列化）
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

// ─── 审计日志筛选项解析 ───────────────────────────────────

/// 解析 action 逗号分隔字符串为 `Vec<AuditAction>`，未知值返回 `AppError::Validation`
fn parse_action_list(input: &str) -> Result<Vec<AuditAction>, AppError> {
    input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| match s {
            "create" => Ok(AuditAction::Create),
            "update" => Ok(AuditAction::Update),
            "delete" => Ok(AuditAction::Delete),
            "enable" => Ok(AuditAction::Enable),
            "disable" => Ok(AuditAction::Disable),
            "recover" => Ok(AuditAction::Recover),
            "auto_recover" => Ok(AuditAction::AutoRecover),
            "create_api_key" => Ok(AuditAction::CreateApiKey),
            "revoke_api_key" => Ok(AuditAction::RevokeApiKey),
            "update_api_key_description" => Ok(AuditAction::UpdateApiKeyDescription),
            "change_password" => Ok(AuditAction::ChangePassword),
            "update_profile" => Ok(AuditAction::UpdateProfile),
            "update_settings" => Ok(AuditAction::UpdateSettings),
            "login" => Ok(AuditAction::Login),
            "login_failed" => Ok(AuditAction::LoginFailed),
            "logout" => Ok(AuditAction::Logout),
            "refresh_rejected" => Ok(AuditAction::RefreshRejected),
            "discover_models" => Ok(AuditAction::DiscoverModels),
            unknown => Err(AppError::Validation(format!("未知的操作类型: {}", unknown))),
        })
        .collect()
}

/// 解析 entity_type 逗号分隔字符串为 `Vec<AuditEntityType>`，未知值返回 `AppError::Validation`
fn parse_entity_type_list(input: &str) -> Result<Vec<AuditEntityType>, AppError> {
    input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| match s {
            "access_point" => Ok(AuditEntityType::AccessPoint),
            "account" => Ok(AuditEntityType::Account),
            "provider" => Ok(AuditEntityType::Provider),
            "user" => Ok(AuditEntityType::User),
            "user_api_key" => Ok(AuditEntityType::UserApiKey),
            "system_settings" => Ok(AuditEntityType::SystemSettings),
            "auth_session" => Ok(AuditEntityType::AuthSession),
            "refresh_token" => Ok(AuditEntityType::RefreshToken),
            unknown => Err(AppError::Validation(format!("未知的实体类型: {}", unknown))),
        })
        .collect()
}

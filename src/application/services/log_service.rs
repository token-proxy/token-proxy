use std::sync::Arc;

use uuid::Uuid;

use crate::application::dto::log_dto::{
    LogDetailResponse, LogFilterParams, LogSummaryResponse, SessionSummaryResponse,
};
use crate::domain::entities::log_entry::{LogContent, LogEntry};
use crate::domain::repositories::log_repository::LogRepository;
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

pub struct LogService {
    log_repo: Arc<dyn LogRepository>,
}

impl LogService {
    pub fn new(log_repo: Arc<dyn LogRepository>) -> Self {
        LogService { log_repo }
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

    /// 分页查询日志摘要
    pub async fn query_logs(
        &self,
        filters: LogFilterParams,
    ) -> Result<PaginatedResult<LogSummaryResponse>, AppError> {
        let page = filters.page.unwrap_or(1);
        let page_size = filters.page_size.unwrap_or(20);

        let result = self
            .log_repo
            .find_all_paginated(page, page_size)
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
        _filters: LogFilterParams,
    ) -> Result<PaginatedResult<SessionSummaryResponse>, AppError> {
        // 按 session_id 分组聚合日志条目
        // 当前仅返回分组后的会话摘要
        let page = _filters.page.unwrap_or(1);
        let page_size = _filters.page_size.unwrap_or(20);

        let result = self
            .log_repo
            .find_all_paginated(page, page_size)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 按 session_id 分组
        let mut session_map: std::collections::HashMap<
            String,
            Vec<LogEntry>,
        > = std::collections::HashMap::new();

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

    /// 获取某个会话的所有日志条目
    pub async fn get_session_detail(
        &self,
        session_id: &str,
    ) -> Result<Vec<LogSummaryResponse>, AppError> {
        let entries = self
            .log_repo
            .find_by_session_id(session_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(entries.iter().map(Self::to_summary).collect())
    }
}
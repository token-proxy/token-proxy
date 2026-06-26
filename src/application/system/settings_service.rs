//! 系统设置应用服务 — application/system/
//!
//! 编排系统配置的读取和更新操作，包括日志保留月数、日志占用上限等全局设置，
//! 并提供日志分区统计查询和按月份删除日志数据的能力。

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Datelike;
use uuid::Uuid;

use crate::domain::log::AuditAction;
use crate::domain::log::AuditEntityType;
use crate::domain::log::AuditLog;
use crate::domain::log::AuditLogRepository;
use crate::domain::system::SystemSettings;
use crate::domain::system::SystemSettingsRepository;
use crate::infrastructure::persistence::partition_manager::PartitionManager;
use crate::shared::error::AppError;

use super::dto::{
    LogStatsResponse, MonthlySummary, PartitionInfo as PartitionInfoDto, SettingsResponse,
    UpdateSettingsRequest,
};

/// 系统设置应用服务
///
/// 编排系统全局配置的读取和更新，更新时记录审计日志。
/// 同时提供日志分区统计查询和按月份删除日志数据的能力。
pub struct SettingsService {
    settings_repo: Arc<dyn SystemSettingsRepository>,
    audit_log_repo: Arc<dyn AuditLogRepository>,
    partition_manager: Arc<PartitionManager>,
}

impl SettingsService {
    pub fn new(
        settings_repo: Arc<dyn SystemSettingsRepository>,
        audit_log_repo: Arc<dyn AuditLogRepository>,
        partition_manager: Arc<PartitionManager>,
    ) -> Self {
        SettingsService {
            settings_repo,
            audit_log_repo,
            partition_manager,
        }
    }

    /// 获取系统设置
    ///
    /// 读取当前系统配置，同时尝试获取日志统计以提供 `log_month_count` 和 `total_size_bytes`。
    /// 若日志统计查询失败，记录 warn 日志并回退为 0/0，确保设置页面仍可正常打开。
    pub async fn get_settings(&self) -> Result<SettingsResponse, AppError> {
        let settings = self.settings_repo.get().await?;

        // 尝试获取日志统计，失败时回退
        let (log_month_count, total_size_bytes) = match self.get_log_stats().await {
            Ok(stats) => (stats.monthly_summary.len(), stats.total_size_bytes),
            Err(e) => {
                tracing::warn!(error = %e, "获取日志统计失败，回退为默认值");
                (0, 0)
            }
        };

        Ok(SettingsResponse {
            log_retention_months: settings.log_retention_months as i16,
            log_storage_cap_gb: settings.log_storage_cap_gb.map(|v| v as i16),
            log_month_count,
            total_size_bytes,
        })
    }

    /// 更新系统设置
    ///
    /// 校验日志保留月数范围（1-36）和日志占用上限（None 或 1-10000），
    /// 保存后记录审计日志。
    pub async fn update_settings(
        &self,
        input: UpdateSettingsRequest,
        user_id: Uuid,
    ) -> Result<SettingsResponse, AppError> {
        // 校验日志保留月数范围
        if !(1..=36).contains(&input.log_retention_months) {
            return Err(AppError::Validation(
                "日志保留月数必须在 1 到 36 之间".to_string(),
            ));
        }

        // 校验日志占用上限（None 或 1-10000）
        if let Some(cap) = input.log_storage_cap_gb {
            if !(1..=10000).contains(&cap) {
                return Err(AppError::Validation(
                    "日志占用上限必须在 1 到 10000 之间".to_string(),
                ));
            }
        }

        let settings = SystemSettings {
            log_retention_months: input.log_retention_months as u32,
            log_storage_cap_gb: input.log_storage_cap_gb.map(|v| v as u32),
        };

        self.settings_repo.save(&settings).await?;

        self.log_audit(
            Some(user_id),
            AuditAction::UpdateSettings,
            AuditEntityType::SystemSettings,
            None,
            Some(serde_json::json!({
                "log_retention_months": input.log_retention_months,
                "log_storage_cap_gb": input.log_storage_cap_gb,
            })),
        )
        .await;

        // 尝试获取日志统计，失败时回退
        let (log_month_count, total_size_bytes) = match self.get_log_stats().await {
            Ok(stats) => (stats.monthly_summary.len(), stats.total_size_bytes),
            Err(e) => {
                tracing::warn!(error = %e, "获取日志统计失败，回退为默认值");
                (0, 0)
            }
        };

        Ok(SettingsResponse {
            log_retention_months: input.log_retention_months,
            log_storage_cap_gb: input.log_storage_cap_gb,
            log_month_count,
            total_size_bytes,
        })
    }

    /// 获取日志分区统计信息
    ///
    /// 返回 `log_metadata` 和 `log_contents` 的所有分区列表、
    /// 按月汇总的磁盘占用（原始字节数），以及总计信息。
    /// 过滤掉未到来月份的预创建空分区。
    pub async fn get_log_stats(&self) -> Result<LogStatsResponse, AppError> {
        // 1. 从 PostgreSQL 系统表查询原始分区统计
        let raw_partitions = self.partition_manager.get_partition_stats().await?;

        // 2. 映射为 DTO，同时过滤掉未到来月份的分区
        let now = chrono::Utc::now().naive_utc().date();
        let current_ym = format!("{:04}-{:02}", now.year(), now.month());

        let partitions: Vec<PartitionInfoDto> = raw_partitions
            .iter()
            .filter_map(|p| {
                extract_month_key(&p.partition_name)
                    .filter(|month_key| month_key <= &current_ym)
                    .map(|_| PartitionInfoDto {
                        partition_name: p.partition_name.clone(),
                        parent_table: p.parent_table.clone(),
                        size_bytes: p.size_bytes,
                        row_count_estimate: p.row_count_estimate,
                    })
            })
            .collect();

        // 3. 计算总计
        let total_partitions = partitions.len();
        let total_size_bytes: i64 = partitions.iter().map(|p| p.size_bytes).sum();

        // 4. 按月份聚合：从 partition_name 提取 `_{YYYY}_{MM}` 后缀
        let mut monthly_map: BTreeMap<String, Vec<&PartitionInfoDto>> = BTreeMap::new();
        for p in &partitions {
            if let Some(month_key) = extract_month_key(&p.partition_name) {
                monthly_map.entry(month_key).or_default().push(p);
            }
        }

        // 4.5 过滤掉未到来的月份（防御性过滤，分区已在上游过滤）
        let monthly_summary: Vec<MonthlySummary> = monthly_map
            .into_iter()
            .filter(|(month, _)| month <= &current_ym)
            .map(|(month, parts)| {
                let size_bytes: i64 = parts.iter().map(|p| p.size_bytes).sum();
                let row_count_estimate: i64 = parts.iter().map(|p| p.row_count_estimate).sum();
                MonthlySummary {
                    month,
                    size_bytes,
                    row_count_estimate,
                }
            })
            .collect();

        Ok(LogStatsResponse {
            partitions,
            monthly_summary,
            total_partitions,
            total_size_bytes,
        })
    }

    /// 删除指定月份的日志分区数据
    ///
    /// 仅删除 `log_contents` 中对应月份的分区（log_requests 为普通表，不可按月删除）。
    /// 当前月份不可删除（保护种子分区），保留窗口内的月份也不可删除。
    pub async fn delete_month_logs(
        &self,
        year_month: &str,
        user_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        // 1. 验证 YYYY-MM 格式
        if year_month.len() != 7 || year_month.chars().nth(4) != Some('-') {
            return Err(AppError::Validation(
                "月份格式不正确，须为 YYYY-MM 格式".to_string(),
            ));
        }
        let (y_str, m_str) = year_month.split_at(4);
        let m_str = &m_str[1..]; // 跳过 '-'
        let (year, month) = match (y_str.parse::<i32>(), m_str.parse::<u32>()) {
            (Ok(y), Ok(m)) if (1..=12).contains(&m) => (y, m),
            _ => {
                return Err(AppError::Validation(
                    "月份格式不正确，须为 YYYY-MM 格式".to_string(),
                ));
            }
        };

        // 2. 检查是否为当前月份
        let now = chrono::Utc::now().naive_utc().date();
        if now.year() == year && now.month() == month {
            return Err(AppError::Validation(
                "当前月份的日志分区不可删除，它是数据库必需的种子分区".to_string(),
            ));
        }

        // 3. 删除 log_contents 分区（log_requests 不分片，无需清理）
        let contents_partition = self
            .partition_manager
            .drop_partition("log_contents", year_month)
            .await?;
        let deleted = vec![contents_partition];

        // 4. 审计日志
        self.log_audit(
            Some(user_id),
            AuditAction::Delete,
            AuditEntityType::SystemSettings,
            None,
            Some(serde_json::json!({
                "action": "delete_month_logs",
                "year_month": year_month,
                "deleted": deleted,
            })),
        )
        .await;

        Ok(serde_json::json!({
            "deleted": deleted,
            "message": format!("已删除 {} 月的日志数据", year_month),
        }))
    }

    /// fire-and-forget 审计日志写入
    async fn log_audit(
        &self,
        operator_id: Option<Uuid>,
        action: AuditAction,
        entity_type: AuditEntityType,
        entity_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) {
        let log = AuditLog::new(operator_id, "user", action, entity_type, entity_id, details);
        if let Err(e) = self.audit_log_repo.save(&log).await {
            tracing::error!(error = %e, action = %action, entity_type = %entity_type, "审计日志写入失败");
        }
    }
}

/// 从分区名中提取月份键（如 `log_metadata_2026_06` → `"2026-06"`）
///
/// 分区名格式为 `{table}_{YYYY}_{MM}`，提取最后两部分组合为 `YYYY-MM`。
/// 若分区名不包含月份信息则返回 `None`。
fn extract_month_key(partition_name: &str) -> Option<String> {
    // 找到倒数第二个下划线：从后往前扫描
    let bytes = partition_name.as_bytes();

    // 找到最后一个 `_' 的位置
    let second_underscore_pos = bytes.iter().rposition(|&b| b == b'_')?;

    // 在第二个下划线之前找到倒数第三个下划线
    let first_underscore_pos = bytes[..second_underscore_pos]
        .iter()
        .rposition(|&b| b == b'_')?;

    let year_str = &partition_name[first_underscore_pos + 1..second_underscore_pos];
    let month_str = &partition_name[second_underscore_pos + 1..];

    // 验证是否为 4 位年份和 2 位月份
    if year_str.len() == 4
        && month_str.len() == 2
        && year_str.chars().all(|c| c.is_ascii_digit())
        && month_str.chars().all(|c| c.is_ascii_digit())
    {
        Some(format!("{}-{}", year_str, month_str))
    } else {
        None
    }
}

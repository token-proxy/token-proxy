//! 故障配置值对象 — domain/provider/
//!
//! 定义 `FaultConfig`（频率限制/余额耗尽配置）、`RecoverType`（恢复策略）、
//! `DurationConfig`、`ExtractConfig` 等类型。
//!
//! 提供状态码匹配、定时/提取恢复时间计算等行为，供 `FaultService` 调用。

use axum::http::HeaderMap;
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── 顶层配置类型（两个 JSONB 列共用）──

/// 故障配置值对象
///
/// 定义触发状态码列表和恢复策略。作为 `rate_limit_config` 和
/// `balance_exhausted_config` 两个 JSONB 列的共用类型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromJsonQueryResult)]
pub struct FaultConfig {
    /// 触发状态码列表，如 ["429"]、["402"]
    pub status_codes: Vec<String>,

    /// 恢复方式（flatten，序列化时内联到父级）
    #[serde(flatten)]
    pub recover_type: RecoverType,
}

// ── 恢复方式（tagged enum，type 字段区分）──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RecoverType {
    /// 手动恢复 — 永久禁用账户，等待管理员手动启用
    Manual,
    /// 定时恢复 — `available_at = now + delay`
    Scheduled {
        delay: DurationConfig,
    },
    /// 从上游响应提取恢复时间
    Extract {
        config: ExtractConfig,
    },
}

// ── 时长配置（scheduled / extract duration 模式共用）──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DurationConfig {
    /// 数值
    pub value: u64,
    /// 时间单位
    pub unit: DurationUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DurationUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}

// ── 提取配置（extract 模式）──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractConfig {
    /// 提取来源类型
    pub source: ExtractSource,
    /// Header 名（如 "Retry-After"）或 JSONPath（如 "$.error.reset_time"）
    pub source_path: String,
    /// 正则表达式，必须含一个捕获组，用于从原始值中提取目标子串
    pub regex_pattern: String,
    /// 提取结果的语义
    pub kind: ExtractKind,
    /// 提取失败时的降级策略
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_extract_failed: Option<OnExtractFailed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractSource {
    Header,
    Body,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ExtractKind {
    /// 提取值是时间间隔 → available_at = now + duration
    Duration {
        unit: DurationUnit,
    },
    /// 提取值是时刻 → 后端自动匹配格式解析后直接作为 available_at
    Timestamp,
}

// ── 提取失败降级策略 ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum OnExtractFailed {
    /// 降级为定时恢复 — 提取失败时按固定延迟重试
    FallbackScheduled {
        delay: DurationConfig,
    },
    /// 降级为手动恢复
    FallbackManual,
}

// ── FaultConfig 行为 ────────────────────────────────────────────────

impl FaultConfig {
    /// 判断上游响应状态码是否命中此故障配置
    pub fn matches_status(&self, status: u16) -> bool {
        self.status_codes
            .iter()
            .any(|c| c.parse::<u16>() == Ok(status))
    }
}

// ── DurationConfig 行为 ─────────────────────────────────────────────

impl DurationConfig {
    /// 将配置值转为 chrono 时长
    pub fn to_duration(&self) -> chrono::Duration {
        match self.unit {
            DurationUnit::Seconds => chrono::Duration::seconds(self.value as i64),
            DurationUnit::Minutes => chrono::Duration::minutes(self.value as i64),
            DurationUnit::Hours => chrono::Duration::hours(self.value as i64),
            DurationUnit::Days => chrono::Duration::days(self.value as i64),
        }
    }
}

// ── RecoverType 行为 ────────────────────────────────────────────────

impl RecoverType {
    /// 根据恢复策略计算账户预计恢复时间
    ///
    /// - Manual → 永久禁用，返回 None
    /// - Scheduled → now + delay
    /// - Extract → 从响应中提取，失败时按降级策略处理
    pub fn calculate_available_at(
        &self,
        resp_headers: &HeaderMap,
        resp_body: &[u8],
    ) -> Option<DateTime<FixedOffset>> {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        let now = Utc::now().with_timezone(&offset);
        match self {
            RecoverType::Manual => None,
            RecoverType::Scheduled { delay } => Some(now + delay.to_duration()),
            RecoverType::Extract { config } => config.extract(resp_headers, resp_body, now),
        }
    }
}

// ── ExtractConfig 行为 ──────────────────────────────────────────────

impl ExtractConfig {
    /// 从上游响应中提取恢复时间（4 步流水线）
    ///
    /// 1. 提取原始值（Header 或 Body JSONPath）
    /// 2. 正则捕获目标子串
    /// 3. 按语义解析为 DateTime
    /// 4. 失败时按降级策略处理
    pub fn extract(
        &self,
        resp_headers: &HeaderMap,
        resp_body: &[u8],
        now: DateTime<FixedOffset>,
    ) -> Option<DateTime<FixedOffset>> {
        // 1. 提取原始值
        let raw = self.extract_value(resp_headers, resp_body);
        match &raw {
            Some(v) => tracing::info!(raw_value = %v, "从上游响应提取原始值成功"),
            None => tracing::warn!("从上游响应提取原始值失败（source_path 在响应中不存在）"),
        }
        // 2. 正则捕获
        let captured = raw.as_deref().and_then(|v| self.apply_regex(v));
        match &captured {
            Some(v) => tracing::info!(captured = %v, "正则捕获成功"),
            None => tracing::warn!(raw = ?raw, regex = %self.regex_pattern, "正则捕获失败"),
        }
        // 3. 解析为 available_at
        let parsed = captured.as_deref().and_then(|v| self.parse_result(v, now));
        match &parsed {
            Some(at) => tracing::info!(available_at = %at, "时间解析成功"),
            None => tracing::warn!(captured = ?captured, kind = ?self.kind, "时间解析失败"),
        }
        // 4. 成功则返回，否则走降级
        match parsed {
            Some(at) => Some(at),
            None => {
                tracing::warn!("从上游响应提取恢复时间失败，使用降级策略");
                match &self.on_extract_failed {
                    Some(OnExtractFailed::FallbackScheduled { delay }) => {
                        Some(now + delay.to_duration())
                    }
                    Some(OnExtractFailed::FallbackManual) | None => None,
                }
            }
        }
    }

    /// 从响应头或响应体 JSONPath 提取原始字符串值
    fn extract_value(&self, resp_headers: &HeaderMap, resp_body: &[u8]) -> Option<String> {
        match self.source {
            ExtractSource::Header => resp_headers
                .get(&self.source_path)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            ExtractSource::Body => {
                let json: Value = serde_json::from_slice(resp_body).ok()?;
                let path = self.source_path.strip_prefix("$.").unwrap_or(&self.source_path);
                let mut current = &json;
                for segment in path.split('.') {
                    current = current.get(segment)?;
                }
                match current {
                    Value::String(s) => Some(s.clone()),
                    other => Some(other.to_string()),
                }
            }
        }
    }

    /// 对原始值应用正则表达式，返回第一个捕获组
    fn apply_regex(&self, raw: &str) -> Option<String> {
        let re = regex::Regex::new(&self.regex_pattern).ok()?;
        re.captures(raw)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// 根据 ExtractKind 解析提取到的值
    fn parse_result(
        &self,
        value: &str,
        now: DateTime<FixedOffset>,
    ) -> Option<DateTime<FixedOffset>> {
        match &self.kind {
            ExtractKind::Duration { unit } => {
                let v: f64 = value.trim().parse().ok()?;
                let dur = match unit {
                    DurationUnit::Seconds => chrono::Duration::seconds(v as i64),
                    DurationUnit::Minutes => chrono::Duration::minutes(v as i64),
                    DurationUnit::Hours => chrono::Duration::hours(v as i64),
                    DurationUnit::Days => chrono::Duration::days(v as i64),
                };
                Some(now + dur)
            }
            ExtractKind::Timestamp => Self::parse_timestamp_auto(value),
        }
    }

    /// 自动尝试多种格式解析时间戳
    fn parse_timestamp_auto(s: &str) -> Option<DateTime<FixedOffset>> {
        let s = s.trim();

        // 0. Unix 时间戳（纯数字）
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        if let Ok(ts) = s.parse::<i64>() {
            // 10 位秒数 / 13 位毫秒数
            let utc_dt = if ts > 1_000_000_000_000 {
                chrono::DateTime::from_timestamp_millis(ts)
            } else {
                chrono::DateTime::from_timestamp(ts, 0)
            };
            return utc_dt.map(|dt| dt.with_timezone(&offset));
        }

        // 1. RFC 3339 / ISO 8601
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt);
        }

        // 2. RFC 2822
        if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
            return Some(dt);
        }

        // 3. 常见日期时间格式（按优先级排列）
        let formats = [
            "%Y-%m-%d %H:%M:%S %#z",
            "%Y-%m-%d %H:%M:%S %z",
            "%Y-%m-%d %H:%M:%S%.f %z",
            "%Y-%m-%dT%H:%M:%S%z",
            "%Y-%m-%dT%H:%M:%S%.f%z",
            "%Y-%m-%dT%H:%M:%S %z",
            "%Y-%m-%d %H:%M:%S",
            "%Y/%m/%d %H:%M:%S",
            "%Y-%m-%d",
        ];

        // 尝试格式化解析（优先带时区的版本）
        for fmt in &formats {
            if let Ok(dt) = DateTime::parse_from_str(s, fmt) {
                return Some(dt);
            }
            // 也尝试不带时区的版本
            if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
                let offset = FixedOffset::east_opt(0).expect("UTC offset");
                return Some(naive.and_utc().with_timezone(&offset));
            }
        }

        // 4. 去掉末尾非数字时区缩写（如 "CST"）再试
        if let Some(last_space) = s.rfind(' ') {
            let after_last = &s[last_space + 1..];
            let is_tz_abbr = after_last
                .chars()
                .all(|c| c.is_alphabetic())
                && !after_last.starts_with('+')
                && !after_last.starts_with('-');
            if is_tz_abbr {
                let stripped = &s[..last_space];
                return Self::parse_timestamp_auto(stripped);
            }
        }

        None
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manual_serialize() {
        let config = FaultConfig {
            status_codes: vec!["429".to_string()],
            recover_type: RecoverType::Manual,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(
            json,
            r#"{"status_codes":["429"],"type":"manual"}"#
        );
    }

    #[test]
    fn test_manual_deserialize() {
        let json = r#"{"status_codes":["429"],"type":"manual"}"#;
        let config: FaultConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.status_codes, vec!["429"]);
        assert!(matches!(config.recover_type, RecoverType::Manual));
    }

    #[test]
    fn test_scheduled_serialize() {
        let config = FaultConfig {
            status_codes: vec!["429".to_string()],
            recover_type: RecoverType::Scheduled {
                delay: DurationConfig {
                    value: 2,
                    unit: DurationUnit::Hours,
                },
            },
        };
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(
            json,
            r#"{"status_codes":["429"],"type":"scheduled","delay":{"value":2,"unit":"hours"}}"#
        );
    }

    #[test]
    fn test_scheduled_deserialize() {
        let json =
            r#"{"status_codes":["429"],"type":"scheduled","delay":{"value":5,"unit":"minutes"}}"#;
        let config: FaultConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.status_codes, vec!["429"]);
        match config.recover_type {
            RecoverType::Scheduled { delay } => {
                assert_eq!(delay.value, 5);
                assert_eq!(delay.unit, DurationUnit::Minutes);
            }
            _ => panic!("expected scheduled"),
        }
    }

    #[test]
    fn test_extract_timestamp_serialize() {
        let config = FaultConfig {
            status_codes: vec!["402".to_string()],
            recover_type: RecoverType::Extract {
                config: ExtractConfig {
                    source: ExtractSource::Body,
                    source_path: "$.error.reset_time".to_string(),
                    regex_pattern: "(.+)".to_string(),
                    kind: ExtractKind::Timestamp,
                    on_extract_failed: Some(OnExtractFailed::FallbackManual),
                },
            },
        };
        let json = serde_json::to_string(&config).unwrap();
        // 验证包含关键字段
        assert!(json.contains(r#""type":"extract""#));
        assert!(json.contains(r#""source":"body""#));
        assert!(json.contains(r#""kind":{"type":"timestamp"}"#));
        assert!(json.contains(r#""on_extract_failed":{"type":"fallback_manual"}"#));
    }

    #[test]
    fn test_extract_duration_deserialize() {
        let json = r#"{
            "status_codes": ["429"],
            "type": "extract",
            "config": {
                "source": "header",
                "source_path": "Retry-After",
                "regex_pattern": "\\d+",
                "kind": { "type": "duration", "unit": "seconds" },
                "on_extract_failed": { "type": "fallback_scheduled", "delay": { "value": 30, "unit": "minutes" } }
            }
        }"#;
        let config: FaultConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.status_codes, vec!["429"]);
        match config.recover_type {
            RecoverType::Extract { config: extract } => {
                assert_eq!(extract.source, ExtractSource::Header);
                assert_eq!(extract.source_path, "Retry-After");
                assert!(matches!(extract.kind, ExtractKind::Duration { unit: DurationUnit::Seconds }));
                match extract.on_extract_failed.unwrap() {
                    OnExtractFailed::FallbackScheduled { delay } => {
                        assert_eq!(delay.value, 30);
                        assert_eq!(delay.unit, DurationUnit::Minutes);
                    }
                    _ => panic!("expected fallback_scheduled"),
                }
            }
            _ => panic!("expected extract"),
        }
    }

    #[test]
    fn test_roundtrip_all_variants() {
        let cases = vec![
            r#"{"status_codes":["429"],"type":"manual"}"#,
            r#"{"status_codes":["429"],"type":"scheduled","delay":{"value":30,"unit":"seconds"}}"#,
            r#"{"status_codes":["402"],"type":"scheduled","delay":{"value":1,"unit":"days"}}"#,
        ];
        for json_str in cases {
            let config: FaultConfig = serde_json::from_str(json_str).unwrap();
            let output = serde_json::to_string(&config).unwrap();
            let roundtrip: FaultConfig = serde_json::from_str(&output).unwrap();
            assert_eq!(config, roundtrip, "roundtrip failed for: {}", json_str);
        }
    }
}

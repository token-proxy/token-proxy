//! 迁移：日志表架构重构 — 合并 log_metadata + log_token_usage → log_requests
//!
//! **背景**：
//! 旧的 log_metadata（按月分区/可过期）和 log_token_usage（永久）存在 12 个维度列冗余，
//! 且所有查询需要 LEFT JOIN。新架构按"数据重量"分表：标量字段合一为 log_requests（永久），
//! 大体积 JSON/TEXT 留在 log_contents（按月分区/可清理）。
//!
//! **变更**：
//! - **新建** `log_requests`：合并两表所有字段，新增 `model_normalized NOT NULL`
//! - **迁移**：从 log_metadata + log_token_usage 复制存量数据并计算 model_normalized
//! - **删除** `log_token_usage`（无分区，直接 DROP）
//! - **删除** `log_metadata CASCADE`（自动删除所有分区子表）
//! - `log_contents` 不变

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

use chrono::Datelike;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260628_000005_log_requests"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── 1. 新建 log_requests 表 ──
        db.execute_unprepared(
            r#"
            CREATE TABLE log_requests (
                id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                timestamp                   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                session_id                  VARCHAR(255) NOT NULL DEFAULT '',
                user_id                     UUID,
                access_point_id             UUID,
                provider_id                 UUID,
                account_id                  UUID,
                model_original              VARCHAR(255),
                model_mapped                VARCHAR(255),
                model_normalized            VARCHAR(255) NOT NULL,
                status_code                 SMALLINT,
                duration_ms                 INTEGER,
                error_message               TEXT,
                is_interrupted              BOOLEAN NOT NULL DEFAULT FALSE,
                has_error                   BOOLEAN NOT NULL DEFAULT FALSE,
                api_type                    VARCHAR(50) NOT NULL DEFAULT 'anthropic',
                client_type                 VARCHAR(32) NOT NULL DEFAULT 'unknown',
                client_user_agent           TEXT,
                client_version              VARCHAR(255),
                conversation_source         VARCHAR(32) NOT NULL DEFAULT 'unknown',
                agent_id                    VARCHAR(255),
                agent_type                  VARCHAR(64),
                input_tokens                INTEGER NOT NULL DEFAULT 0,
                output_tokens               INTEGER NOT NULL DEFAULT 0,
                cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_input_tokens     INTEGER NOT NULL DEFAULT 0,
                thinking_tokens             INTEGER NOT NULL DEFAULT 0,
                total_tokens                INTEGER NOT NULL DEFAULT 0,
                raw_usage                   JSONB,
                server_tool_usage           JSONB,
                cache_creation              JSONB,
                created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .await?;

        // ── 2. 存量数据迁移 ──
        db.execute_unprepared(
            r#"
            INSERT INTO log_requests (
                id, timestamp, session_id, user_id, access_point_id,
                provider_id, account_id,
                model_original, model_mapped, model_normalized,
                status_code, duration_ms, error_message,
                is_interrupted, has_error,
                api_type, client_type,
                client_user_agent, client_version,
                conversation_source, agent_id, agent_type,
                input_tokens, output_tokens,
                cache_creation_input_tokens, cache_read_input_tokens,
                thinking_tokens, total_tokens,
                raw_usage, server_tool_usage, cache_creation,
                created_at
            )
            SELECT
                lm.id,
                lm.timestamp,
                lm.session_id,
                lm.user_id,
                lm.access_point_id,
                lm.provider_id,
                lm.account_id,
                lm.model_original,
                lm.model_mapped,
                LOWER(
                    REPLACE(
                        REPLACE(
                            TRIM(COALESCE(lm.model_mapped, lm.model_original, 'unknown')),
                            ' ',
                            '-'
                        ),
                        '_',
                        '-'
                    )
                ) AS model_normalized,
                lm.status_code,
                lm.duration_ms,
                lm.error_message,
                lm.is_interrupted,
                lm.has_error,
                lm.api_type,
                COALESCE(lm.client_type, 'unknown'),
                lm.client_user_agent,
                lm.client_version,
                lm.conversation_source,
                lm.agent_id,
                ltu.agent_type,
                COALESCE(ltu.input_tokens, 0),
                COALESCE(ltu.output_tokens, 0),
                COALESCE(ltu.cache_creation_input_tokens, 0),
                COALESCE(ltu.cache_read_input_tokens, 0),
                COALESCE(ltu.thinking_tokens, 0),
                COALESCE(ltu.total_tokens, 0),
                ltu.raw_usage,
                ltu.server_tool_usage,
                ltu.cache_creation,
                COALESCE(ltu.created_at, lm.timestamp)
            FROM log_metadata lm
            LEFT JOIN log_token_usage ltu ON ltu.log_id = lm.id
            "#,
        )
        .await?;

        // ── 3. 删除 log_token_usage（无分区，无 FK 约束）──
        db.execute_unprepared("DROP TABLE IF EXISTS log_token_usage")
            .await?;

        // ── 4. 删除 log_metadata CASCADE（自动删除所有分区子表）──
        // 已验证：整个项目中三张日志表之间无任何 FOREIGN KEY 声明，
        // CASCADE 只作用于分区子表，不波及 log_contents 及其他表。
        db.execute_unprepared("DROP TABLE IF EXISTS log_metadata CASCADE")
            .await?;

        // ── 5. 索引 ──
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_log_requests_user_id ON log_requests (user_id)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_log_requests_timestamp ON log_requests (timestamp)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_log_requests_session_id ON log_requests (session_id)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_log_requests_model_normalized ON log_requests (model_normalized)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_log_requests_client_type ON log_requests (client_type)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_log_requests_access_point_id ON log_requests (access_point_id)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 重新创建 log_metadata 分区表及其当前月份分区
        // 注意：这里仅重建结构，无法精确还原分区边界。
        // 实际回滚在生产中极少使用，此处提供基本结构重建。
        let now = chrono::Utc::now();
        let (y, m) = (now.year(), now.month());
        let (next_y, next_m) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        let partition_name = format!("log_metadata_{:04}_{:02}", y, m);
        let from_str = format!("{:04}-{:02}-01", y, m);
        let to_str = format!("{:04}-{:02}-01", next_y, next_m);

        db.execute_unprepared(&format!(
            r#"
            CREATE TABLE log_metadata (
                id                     UUID NOT NULL,
                timestamp              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                session_id             VARCHAR(255) NOT NULL DEFAULT '',
                user_id                UUID,
                access_point_id        UUID,
                provider_id            UUID,
                account_id             UUID,
                model_original         VARCHAR(255),
                model_mapped           VARCHAR(255),
                status_code            SMALLINT,
                duration_ms            INTEGER,
                error_message          TEXT,
                client_user_agent      TEXT,
                conversation_source    VARCHAR(32) NOT NULL DEFAULT 'unknown',
                agent_id               VARCHAR(255),
                has_error              BOOLEAN NOT NULL DEFAULT FALSE,
                raw_content_available  BOOLEAN NOT NULL DEFAULT TRUE,
                is_interrupted         BOOLEAN NOT NULL DEFAULT FALSE,
                client_version         VARCHAR(255),
                api_type               VARCHAR(50) NOT NULL DEFAULT 'anthropic',
                client_type            VARCHAR(32) NOT NULL DEFAULT 'unknown',
                PRIMARY KEY (id, timestamp)
            ) PARTITION BY RANGE (timestamp);

            CREATE TABLE IF NOT EXISTS {partition_name}
                PARTITION OF log_metadata
                FOR VALUES FROM ('{from_str}') TO ('{to_str}');
            "#
        ))
        .await?;

        // 重建 log_token_usage 表
        db.execute_unprepared(
            r#"
            CREATE TABLE log_token_usage (
                id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                log_id                      UUID NOT NULL UNIQUE,
                session_id                  VARCHAR(255) NOT NULL,
                timestamp                   TIMESTAMPTZ NOT NULL,
                user_id                     UUID,
                access_point_id             UUID,
                provider_id                 UUID,
                account_id                  UUID,
                model_original              VARCHAR(255),
                model_mapped                VARCHAR(255),
                conversation_source         VARCHAR(32),
                agent_id                    VARCHAR(255),
                agent_type                  VARCHAR(64),
                input_tokens                INTEGER NOT NULL DEFAULT 0,
                output_tokens               INTEGER NOT NULL DEFAULT 0,
                cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_input_tokens     INTEGER NOT NULL DEFAULT 0,
                thinking_tokens             INTEGER NOT NULL DEFAULT 0,
                total_tokens                INTEGER NOT NULL DEFAULT 0,
                raw_usage                   JSONB,
                server_tool_usage           JSONB,
                cache_creation              JSONB,
                client_type                 VARCHAR(32) NOT NULL DEFAULT 'unknown',
                created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .await?;

        // 将数据从 log_requests 迁回旧表
        db.execute_unprepared(
            r#"
            INSERT INTO log_metadata (
                id, timestamp, session_id, user_id, access_point_id,
                provider_id, account_id, model_original, model_mapped,
                status_code, duration_ms, error_message,
                client_user_agent, conversation_source, agent_id,
                has_error, raw_content_available, is_interrupted,
                client_version, api_type, client_type
            )
            SELECT
                id, timestamp, session_id, user_id, access_point_id,
                provider_id, account_id, model_original, model_mapped,
                status_code, duration_ms, error_message,
                client_user_agent, conversation_source, agent_id,
                has_error, TRUE, is_interrupted,
                client_version, api_type, client_type
            FROM log_requests
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            INSERT INTO log_token_usage (
                log_id, session_id, timestamp, user_id, access_point_id,
                provider_id, account_id, model_original, model_mapped,
                conversation_source, agent_id,
                input_tokens, output_tokens,
                cache_creation_input_tokens, cache_read_input_tokens,
                thinking_tokens, total_tokens,
                raw_usage, server_tool_usage, cache_creation,
                client_type, created_at
            )
            SELECT
                id, session_id, timestamp, user_id, access_point_id,
                provider_id, account_id, model_original, model_mapped,
                conversation_source, agent_id,
                input_tokens, output_tokens,
                cache_creation_input_tokens, cache_read_input_tokens,
                thinking_tokens, total_tokens,
                raw_usage, server_tool_usage, cache_creation,
                client_type, created_at
            FROM log_requests
            WHERE total_tokens > 0
            "#,
        )
        .await?;

        db.execute_unprepared("DROP TABLE IF EXISTS log_requests")
            .await?;

        Ok(())
    }
}

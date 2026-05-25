use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. providers 表
        manager
            .create_table(
                Table::create()
                    .table(Providers::Table)
                    .if_not_exists()
                    .col(pk_uuid(Providers::Id).default(Expr::cust("gen_random_uuid()")))
                    .col(string(Providers::Name).not_null())
                    .col(string_null(Providers::OpenaiBaseUrl))
                    .col(string_null(Providers::AnthropicBaseUrl))
                    .col(json(Providers::Models).default(Expr::cust("'[]'::json")))
                    .col(string_null(Providers::DefaultModel))
                    .col(string(Providers::Status).default("enabled"))
                    .col(
                        timestamp_with_time_zone(Providers::CreatedAt).default(Expr::cust("NOW()")),
                    )
                    .col(
                        timestamp_with_time_zone(Providers::UpdatedAt).default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. accounts 表
        manager
            .create_table(
                Table::create()
                    .table(Accounts::Table)
                    .if_not_exists()
                    .col(pk_uuid(Accounts::Id).default(Expr::cust("gen_random_uuid()")))
                    .col(uuid(Accounts::ProviderId).not_null())
                    .col(string(Accounts::Name).default(""))
                    .col(binary(Accounts::ApiKeyEncrypted).not_null())
                    .col(string_len(Accounts::ApiKeySuffix, 6).not_null())
                    .col(string(Accounts::Status).default("enabled"))
                    .col(timestamp_with_time_zone(Accounts::CreatedAt).default(Expr::cust("NOW()")))
                    .col(timestamp_with_time_zone(Accounts::UpdatedAt).default(Expr::cust("NOW()")))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Accounts::Table, Accounts::ProviderId)
                            .to(Providers::Table, Providers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 4. users 表
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(pk_uuid(Users::Id).default(Expr::cust("gen_random_uuid()")))
                    .col(string_uniq(Users::Username).not_null())
                    .col(string(Users::DisplayName).not_null())
                    .col(string(Users::PasswordHash).not_null())
                    .col(string(Users::Status).default("enabled"))
                    .col(timestamp_with_time_zone(Users::CreatedAt).default(Expr::cust("NOW()")))
                    .col(timestamp_with_time_zone(Users::UpdatedAt).default(Expr::cust("NOW()")))
                    .to_owned(),
            )
            .await?;

        // 5. access_points 表
        manager
            .create_table(
                Table::create()
                    .table(AccessPoints::Table)
                    .if_not_exists()
                    .col(pk_uuid(AccessPoints::Id).default(Expr::cust("gen_random_uuid()")))
                    .col(string(AccessPoints::Name).not_null())
                    .col(string(AccessPoints::ApiType).default("anthropic"))
                    .col(string_uniq(AccessPoints::ShortCode).not_null())
                    .col(uuid(AccessPoints::ProviderId).not_null())
                    .col(uuid(AccessPoints::AccountId).not_null())
                    .col(json(AccessPoints::ModelMappings).default(Expr::cust("'[]'::json")))
                    .col(string(AccessPoints::Status).default("enabled"))
                    .col(uuid(AccessPoints::CreatedBy).not_null())
                    .col(
                        timestamp_with_time_zone(AccessPoints::CreatedAt)
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        timestamp_with_time_zone(AccessPoints::UpdatedAt)
                            .default(Expr::cust("NOW()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(AccessPoints::Table, AccessPoints::ProviderId)
                            .to(Providers::Table, Providers::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(AccessPoints::Table, AccessPoints::AccountId)
                            .to(Accounts::Table, Accounts::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(AccessPoints::Table, AccessPoints::CreatedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        // 6. refresh_tokens 表
        manager
            .create_table(
                Table::create()
                    .table(RefreshTokens::Table)
                    .if_not_exists()
                    .col(pk_uuid(RefreshTokens::Id).default(Expr::cust("gen_random_uuid()")))
                    .col(uuid(RefreshTokens::UserId).not_null())
                    .col(string(RefreshTokens::TokenHash).not_null())
                    .col(timestamp_with_time_zone(RefreshTokens::ExpiresAt).not_null())
                    .col(boolean(RefreshTokens::Revoked).default(false))
                    .col(
                        timestamp_with_time_zone(RefreshTokens::CreatedAt)
                            .default(Expr::cust("NOW()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RefreshTokens::Table, RefreshTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 7. user_api_keys 表
        manager
            .create_table(
                Table::create()
                    .table(UserApiKeys::Table)
                    .if_not_exists()
                    .col(pk_uuid(UserApiKeys::Id).default(Expr::cust("gen_random_uuid()")))
                    .col(uuid(UserApiKeys::UserId).not_null())
                    .col(string_len(UserApiKeys::KeyHash, 64).unique_key().not_null())
                    .col(string_len(UserApiKeys::KeyPrefix, 32).not_null())
                    .col(string_len(UserApiKeys::Description, 255).not_null().default(""))
                    .col(timestamp_with_time_zone_null(UserApiKeys::LastUsedAt))
                    .col(string_len(UserApiKeys::Status, 20).not_null().default("enabled"))
                    .col(timestamp_with_time_zone(UserApiKeys::CreatedAt).not_null().default(Expr::cust("NOW()")))
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserApiKeys::Table, UserApiKeys::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_api_keys_user_id")
                    .table(UserApiKeys::Table)
                    .col(UserApiKeys::UserId)
                    .to_owned(),
            )
            .await?;

        // 8. log_metadata 分区表（主表 + 种子分区）
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS log_metadata (
                    id                         UUID NOT NULL,
                    timestamp                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    session_id                 VARCHAR(255) NOT NULL DEFAULT '',
                    user_id                    UUID,
                    access_point_id            UUID,
                    provider_id                UUID,
                    account_id                 UUID,
                    model_original             VARCHAR(255),
                    model_mapped               VARCHAR(255),
                    status_code                SMALLINT,
                    duration_ms                INTEGER,
                    error_message              TEXT,
                    request_index              INTEGER NOT NULL DEFAULT 0,
                    client_session_id          VARCHAR(255),
                    client_app                 VARCHAR(64),
                    client_user_agent          TEXT,
                    conversation_source        VARCHAR(32) NOT NULL DEFAULT 'unknown',
                    agent_id                   VARCHAR(255),
                    agent_type                 VARCHAR(64),
                    parent_agent_tool_use_id   VARCHAR(255),
                    request_kind               VARCHAR(64),
                    primary_tool_name          VARCHAR(128),
                    message_preview            TEXT,
                    message_full               TEXT,
                    response_preview           TEXT,
                    has_thinking               BOOLEAN NOT NULL DEFAULT FALSE,
                    has_tool_use               BOOLEAN NOT NULL DEFAULT FALSE,
                    has_error                  BOOLEAN NOT NULL DEFAULT FALSE,
                    raw_content_available      BOOLEAN NOT NULL DEFAULT TRUE,
                    parser_version             VARCHAR(20),
                    client_name                VARCHAR(100),
                    client_version             VARCHAR(50),
                    client_channel             VARCHAR(50),
                    client_platform            VARCHAR(50),
                    api_type                   VARCHAR(50) NOT NULL DEFAULT 'anthropic',
                    PRIMARY KEY (id, timestamp)
                ) PARTITION BY RANGE (timestamp);

                -- 创建当月分区（种子分区，后续由应用层 PartitionManager 管理）
                CREATE TABLE IF NOT EXISTS log_metadata_2026_05
                    PARTITION OF log_metadata
                    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
                "#,
            )
            .await?;

        // 8. log_contents 表
        manager
            .create_table(
                Table::create()
                    .table(LogContents::Table)
                    .if_not_exists()
                    .col(uuid(LogContents::LogId).not_null())
                    .col(json(LogContents::RequestHeaders))
                    .col(json(LogContents::RequestBody))
                    .col(text(LogContents::ResponseBody))
                    .primary_key(Index::create().col(LogContents::LogId))
                    .to_owned(),
            )
            .await?;

        // 9. log_conversation_events 和 log_token_usage 表
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS log_conversation_events (
                    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    log_id              UUID NOT NULL,
                    session_id          VARCHAR(255) NOT NULL,
                    timestamp           TIMESTAMPTZ NOT NULL,
                    request_index       INTEGER NOT NULL,
                    event_index         INTEGER NOT NULL,
                    parent_event_id     UUID,
                    parent_tool_use_id  VARCHAR(255),
                    source              VARCHAR(32) NOT NULL,
                    role                VARCHAR(32) NOT NULL,
                    event_type          VARCHAR(64) NOT NULL,
                    agent_id            VARCHAR(255),
                    agent_type          VARCHAR(64),
                    tool_use_id         VARCHAR(255),
                    tool_name           VARCHAR(128),
                    title               TEXT,
                    content             TEXT,
                    content_preview     TEXT,
                    thinking_content    TEXT,
                    hidden_content      JSONB,
                    display_payload     JSONB,
                    confidence          SMALLINT NOT NULL DEFAULT 100,
                    content_type        VARCHAR(50),
                    signature           TEXT,
                    tool_result_content TEXT,
                    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );

                CREATE TABLE IF NOT EXISTS log_token_usage (
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
                    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                "#,
            )
            .await?;

        // 10. audit_logs 表
        manager
            .create_table(
                Table::create()
                    .table(AuditLogs::Table)
                    .if_not_exists()
                    .col(pk_uuid(AuditLogs::Id).default(Expr::cust("gen_random_uuid()")))
                    .col(uuid_null(AuditLogs::UserId))
                    .col(string(AuditLogs::Action).not_null())
                    .col(string(AuditLogs::EntityType).not_null())
                    .col(uuid_null(AuditLogs::EntityId))
                    .col(json(AuditLogs::Details))
                    .col(
                        timestamp_with_time_zone(AuditLogs::Timestamp).default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        // 10. 索引
        manager
            .create_index(
                Index::create()
                    .name("idx_accounts_provider_id")
                    .table(Accounts::Table)
                    .col(Accounts::ProviderId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_access_points_short_code")
                    .table(AccessPoints::Table)
                    .col(AccessPoints::ShortCode)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_access_points_created_by")
                    .table(AccessPoints::Table)
                    .col(AccessPoints::CreatedBy)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_refresh_tokens_user_id")
                    .table(RefreshTokens::Table)
                    .col(RefreshTokens::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_log_metadata_timestamp")
                    .table(LogMetadata::Table)
                    .col(LogMetadata::Timestamp)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_log_metadata_session_id")
                    .table(LogMetadata::Table)
                    .col(LogMetadata::SessionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_log_metadata_user_id")
                    .table(LogMetadata::Table)
                    .col(LogMetadata::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_log_metadata_access_point_id")
                    .table(LogMetadata::Table)
                    .col(LogMetadata::AccessPointId)
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_log_metadata_client_session_id
                    ON log_metadata (client_session_id);
                CREATE INDEX IF NOT EXISTS idx_log_metadata_agent_id
                    ON log_metadata (session_id, agent_id);
                CREATE INDEX IF NOT EXISTS idx_log_metadata_session_request
                    ON log_metadata (session_id, request_index);
                CREATE INDEX IF NOT EXISTS idx_log_metadata_source
                    ON log_metadata (conversation_source);
                CREATE INDEX IF NOT EXISTS idx_log_events_session_sort
                    ON log_conversation_events (session_id, request_index, event_index);
                CREATE INDEX IF NOT EXISTS idx_log_events_log_id
                    ON log_conversation_events (log_id);
                CREATE INDEX IF NOT EXISTS idx_log_events_agent
                    ON log_conversation_events (session_id, agent_id);
                CREATE INDEX IF NOT EXISTS idx_log_events_parent
                    ON log_conversation_events (parent_event_id);
                CREATE INDEX IF NOT EXISTS idx_log_token_usage_session
                    ON log_token_usage (session_id);
                CREATE INDEX IF NOT EXISTS idx_log_token_usage_user_time
                    ON log_token_usage (user_id, timestamp DESC);
                CREATE INDEX IF NOT EXISTS idx_log_token_usage_model_time
                    ON log_token_usage (model_mapped, timestamp DESC);
                CREATE INDEX IF NOT EXISTS idx_log_token_usage_agent
                    ON log_token_usage (session_id, agent_id);
                "#,
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_audit_logs_timestamp")
                    .table(AuditLogs::Table)
                    .col(AuditLogs::Timestamp)
                    .to_owned(),
            )
            .await?;

        // 11. 物化视图
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE MATERIALIZED VIEW IF NOT EXISTS daily_request_stats AS
                SELECT
                    date_trunc('day', timestamp) AS day,
                    user_id,
                    access_point_id,
                    provider_id,
                    account_id,
                    model_original,
                    COUNT(*)                                   AS request_count,
                    AVG(duration_ms)::INTEGER                  AS avg_duration_ms,
                    COUNT(*) FILTER (WHERE status_code >= 400) AS error_count
                FROM log_metadata
                WHERE timestamp >= NOW() - INTERVAL '365 days'
                GROUP BY 1, 2, 3, 4, 5, 6
                WITH DATA;

                CREATE UNIQUE INDEX IF NOT EXISTS idx_daily_stats_unique
                    ON daily_request_stats (day, user_id, access_point_id, provider_id, account_id, model_original);
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP MATERIALIZED VIEW IF EXISTS daily_request_stats CASCADE")
            .await?;

        manager
            .drop_table(Table::drop().table(AuditLogs::Table).to_owned())
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS log_token_usage CASCADE; DROP TABLE IF EXISTS log_conversation_events CASCADE;",
            )
            .await?;

        manager
            .drop_table(Table::drop().table(LogContents::Table).to_owned())
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS log_metadata CASCADE")
            .await?;

        manager
            .drop_table(Table::drop().table(RefreshTokens::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AccessPoints::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Accounts::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Providers::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Providers {
    Table,
    Id,
    Name,
    OpenaiBaseUrl,
    AnthropicBaseUrl,
    Models,
    DefaultModel,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Accounts {
    Table,
    Id,
    ProviderId,
    Name,
    ApiKeyEncrypted,
    ApiKeySuffix,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    DisplayName,
    PasswordHash,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AccessPoints {
    Table,
    Id,
    Name,
    ApiType,
    ShortCode,
    ProviderId,
    AccountId,
    ModelMappings,
    Status,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum RefreshTokens {
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
    Revoked,
    CreatedAt,
}

#[derive(DeriveIden)]
#[allow(dead_code)]
enum LogMetadata {
    Table,
    Id,
    Timestamp,
    SessionId,
    UserId,
    AccessPointId,
    ProviderId,
    AccountId,
    ModelOriginal,
    ModelMapped,
    StatusCode,
    DurationMs,
    ErrorMessage,
    RequestIndex,
    ClientSessionId,
    ClientApp,
    ClientUserAgent,
    ConversationSource,
    AgentId,
    AgentType,
    ParentAgentToolUseId,
    RequestKind,
    PrimaryToolName,
    MessagePreview,
    MessageFull,
    ResponsePreview,
    HasThinking,
    HasToolUse,
    HasError,
    RawContentAvailable,
}

#[derive(DeriveIden)]
enum LogContents {
    Table,
    LogId,
    RequestHeaders,
    RequestBody,
    ResponseBody,
}

#[derive(DeriveIden)]
enum AuditLogs {
    Table,
    Id,
    UserId,
    Action,
    EntityType,
    EntityId,
    Details,
    Timestamp,
}

#[derive(DeriveIden)]
enum UserApiKeys {
    Table,
    Id,
    UserId,
    KeyHash,
    KeyPrefix,
    Description,
    LastUsedAt,
    Status,
    CreatedAt,
}

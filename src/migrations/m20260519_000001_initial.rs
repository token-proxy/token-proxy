use chrono::{Datelike, Utc};
use sea_orm_migration::{prelude::*, schema::*};

fn add_months(year: i32, month: u32, n: i32) -> (i32, u32) {
    let total_months = (year * 12 + month as i32 - 1) + n;
    let y = total_months.div_euclid(12);
    let m = (total_months.rem_euclid(12) + 1) as u32;
    (y, m)
}

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
                    .col(string_null(AccessPoints::DefaultModel))
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
                    .col(
                        string_len(UserApiKeys::Description, 255)
                            .not_null()
                            .default(""),
                    )
                    .col(timestamp_with_time_zone_null(UserApiKeys::LastUsedAt))
                    .col(
                        string_len(UserApiKeys::Status, 20)
                            .not_null()
                            .default("enabled"),
                    )
                    .col(
                        timestamp_with_time_zone(UserApiKeys::CreatedAt)
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
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
        let now = Utc::now().naive_utc().date();
        let (seed_y, seed_m) = (now.year(), now.month());
        let (next_y, next_m) = add_months(seed_y, seed_m, 1);
        let partition_name = format!("log_metadata_{:04}_{:02}", seed_y, seed_m);
        let from_str = format!("{:04}-{:02}-01", seed_y, seed_m);
        let to_str = format!("{:04}-{:02}-01", next_y, next_m);

        let sql = format!(
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
                client_app                 VARCHAR(64),
                client_user_agent          TEXT,
                conversation_source        VARCHAR(32) NOT NULL DEFAULT 'unknown',
                agent_id                   VARCHAR(255),
                has_error                  BOOLEAN NOT NULL DEFAULT FALSE,
                raw_content_available      BOOLEAN NOT NULL DEFAULT TRUE,
                is_interrupted             BOOLEAN NOT NULL DEFAULT FALSE,
                client_name                VARCHAR(100),
                client_version             VARCHAR(50),
                client_channel             VARCHAR(50),
                client_platform            VARCHAR(50),
                api_type                   VARCHAR(50) NOT NULL DEFAULT 'anthropic',
                PRIMARY KEY (id, timestamp)
            ) PARTITION BY RANGE (timestamp);

            CREATE TABLE IF NOT EXISTS {partition_name}
                PARTITION OF log_metadata
                FOR VALUES FROM ('{from_str}') TO ('{to_str}');
            "#
        );

        manager.get_connection().execute_unprepared(&sql).await?;

        // 8. log_contents 分区表（主表 + 种子分区）
        let lc_partition_name = format!("log_contents_{:04}_{:02}", seed_y, seed_m);

        let lc_sql = format!(
            r#"
            CREATE TABLE log_contents (
                log_id           UUID NOT NULL,
                timestamp        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                request_headers  JSON,
                request_body     JSON,
                response_body    TEXT,
                response_headers JSON,
                PRIMARY KEY (log_id, timestamp)
            ) PARTITION BY RANGE (timestamp);

            CREATE TABLE {lc_partition_name}
                PARTITION OF log_contents
                FOR VALUES FROM ('{from_str}') TO ('{to_str}');
            "#
        );

        manager.get_connection().execute_unprepared(&lc_sql).await?;

        // 9. log_token_usage 表
        manager
            .get_connection()
            .execute_unprepared(
                r#"
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
                    .col(json_null(AuditLogs::Details))
                    .col(
                        timestamp_with_time_zone(AuditLogs::Timestamp).default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        // 11. system_settings 表（单行表，id 恒为 1）
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS system_settings (
                    id                      SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
                    log_retention_months    SMALLINT NOT NULL DEFAULT 12
                        CHECK (log_retention_months BETWEEN 1 AND 36),
                    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                "#,
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
                CREATE INDEX IF NOT EXISTS idx_log_metadata_agent_id
                    ON log_metadata (session_id, agent_id);
                CREATE INDEX IF NOT EXISTS idx_log_metadata_session_request
                    ON log_metadata (session_id, request_index);
                CREATE INDEX IF NOT EXISTS idx_log_metadata_source
                    ON log_metadata (conversation_source);
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS system_settings")
            .await?;

        manager
            .drop_table(Table::drop().table(AuditLogs::Table).to_owned())
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS log_token_usage CASCADE;")
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
    DefaultModel,
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
    ClientApp,
    ClientUserAgent,
    ConversationSource,
    AgentId,
    HasError,
    RawContentAvailable,
}

#[derive(DeriveIden)]
#[allow(dead_code)]
enum LogContents {
    Table,
    LogId,
    Timestamp,
    RequestHeaders,
    RequestBody,
    ResponseBody,
    ResponseHeaders,
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

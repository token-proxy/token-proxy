use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // a) 创建 access_point_accounts 表
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS access_point_accounts (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    access_point_id UUID NOT NULL REFERENCES access_points(id) ON DELETE CASCADE,
                    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
                    weight INT NOT NULL DEFAULT 1,
                    priority INT NOT NULL DEFAULT 0,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    UNIQUE(access_point_id, account_id)
                );
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_apa_access_point_id
                    ON access_point_accounts(access_point_id);
                "#,
            )
            .await?;

        // b) 创建 session_affinity 表
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS session_affinity (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    access_point_id UUID NOT NULL REFERENCES access_points(id) ON DELETE CASCADE,
                    session_id VARCHAR(255) NOT NULL,
                    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    UNIQUE(access_point_id, session_id)
                );
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_sa_access_session
                    ON session_affinity(access_point_id, session_id);
                "#,
            )
            .await?;

        // b2) 已有数据库补加 updated_at 列（新表已在 CREATE TABLE 中包含）
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE session_affinity
                    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
                "#,
            )
            .await?;

        // c) 修改 access_points 表：删除 provider_id, account_id, model_mappings, default_model
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE access_points
                    DROP COLUMN IF EXISTS provider_id,
                    DROP COLUMN IF EXISTS account_id,
                    DROP COLUMN IF EXISTS model_mappings,
                    DROP COLUMN IF EXISTS default_model;
                "#,
            )
            .await?;

        // c2) access_points 表：新增 routing_strategy, model_routing_grid
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE access_points
                    ADD COLUMN IF NOT EXISTS routing_strategy VARCHAR(20) NOT NULL DEFAULT 'priority',
                    ADD COLUMN IF NOT EXISTS model_routing_grid JSONB NOT NULL DEFAULT '{"provider_ids":[],"rows":[]}'::jsonb;
                "#,
            )
            .await?;

        // d) accounts 表：新增 disabled_reason, available_at
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE accounts
                    ADD COLUMN IF NOT EXISTS disabled_reason VARCHAR(50),
                    ADD COLUMN IF NOT EXISTS available_at TIMESTAMPTZ;
                "#,
            )
            .await?;

        // e) providers 表：删除 default_model
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE providers
                    DROP COLUMN IF EXISTS default_model;
                "#,
            )
            .await?;

        // e2) providers 表：新增 rate_limit_config, balance_exhausted_config
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE providers
                    ADD COLUMN IF NOT EXISTS rate_limit_config JSONB,
                    ADD COLUMN IF NOT EXISTS balance_exhausted_config JSONB;
                "#,
            )
            .await?;

        // f) audit_logs 表：重命名 user_id → operator_id
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE audit_logs
                    RENAME COLUMN user_id TO operator_id;
                "#,
            )
            .await?;

        // f2) audit_logs 表：新增 operator_type
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE audit_logs
                    ADD COLUMN IF NOT EXISTS operator_type VARCHAR(20) NOT NULL DEFAULT 'user';
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逆向操作：反向执行上述 DDL

        // f) audit_logs：删除 operator_type，回退列名
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE audit_logs
                    DROP COLUMN IF EXISTS operator_type;
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE audit_logs
                    RENAME COLUMN operator_id TO user_id;
                "#,
            )
            .await?;

        // e) providers：删除新增列，恢复 default_model
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE providers
                    DROP COLUMN IF EXISTS rate_limit_config,
                    DROP COLUMN IF EXISTS balance_exhausted_config;
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE providers
                    ADD COLUMN IF NOT EXISTS default_model VARCHAR(255);
                "#,
            )
            .await?;

        // d) accounts：删除新增列
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE accounts
                    DROP COLUMN IF EXISTS disabled_reason,
                    DROP COLUMN IF EXISTS available_at;
                "#,
            )
            .await?;

        // c) access_points：删除新增列，恢复旧列
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE access_points
                    DROP COLUMN IF EXISTS routing_strategy,
                    DROP COLUMN IF EXISTS model_routing_grid;
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE access_points
                    ADD COLUMN IF NOT EXISTS provider_id UUID REFERENCES providers(id) ON DELETE RESTRICT,
                    ADD COLUMN IF NOT EXISTS account_id UUID REFERENCES accounts(id) ON DELETE RESTRICT,
                    ADD COLUMN IF NOT EXISTS model_mappings JSON NOT NULL DEFAULT '[]'::json,
                    ADD COLUMN IF NOT EXISTS default_model VARCHAR(255);
                "#,
            )
            .await?;

        // b) 删除 session_affinity 表
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS session_affinity CASCADE")
            .await?;

        // a) 删除 access_point_accounts 表
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS access_point_accounts CASCADE")
            .await?;

        Ok(())
    }
}

//! 迁移：客户端类型归一化 — 新增 client_type 列并删除冗余解析字段
//!
//! **新增**：
//! - `log_metadata.client_type VARCHAR(32)` + 索引
//! - `log_token_usage.client_type VARCHAR(32)` + 索引
//!
//! **删除**（自研 UA 解析器产生的冗余字段）：
//! - `client_app`、`client_name`、`client_channel`、`client_platform`
//!
//! **扩宽**：
//! - `client_version VARCHAR(50)` → `VARCHAR(255)`，防止非标准 UA 版本号溢出

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── 新增 client_type 列 ──
        db.execute_unprepared(
            r#"
            ALTER TABLE log_metadata
                ADD COLUMN IF NOT EXISTS client_type VARCHAR(32) NOT NULL DEFAULT 'unknown';
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            CREATE INDEX IF NOT EXISTS idx_log_metadata_client_type
                ON log_metadata (client_type);
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            ALTER TABLE log_token_usage
                ADD COLUMN IF NOT EXISTS client_type VARCHAR(32) NOT NULL DEFAULT 'unknown';
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            CREATE INDEX IF NOT EXISTS idx_log_token_usage_client_type
                ON log_token_usage (client_type);
            "#,
        )
        .await?;

        // ── 删除冗余解析字段 + 扩宽 client_version ──
        db.execute_unprepared(
            r#"
            ALTER TABLE log_metadata
                DROP COLUMN IF EXISTS client_app,
                DROP COLUMN IF EXISTS client_name,
                DROP COLUMN IF EXISTS client_channel,
                DROP COLUMN IF EXISTS client_platform;

            ALTER TABLE log_metadata
                ALTER COLUMN client_version TYPE VARCHAR(255);
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 还原冗余字段 + 缩回 client_version
        db.execute_unprepared(
            r#"
            ALTER TABLE log_metadata
                ADD COLUMN IF NOT EXISTS client_app      VARCHAR(64),
                ADD COLUMN IF NOT EXISTS client_name      VARCHAR(100),
                ADD COLUMN IF NOT EXISTS client_channel   VARCHAR(50),
                ADD COLUMN IF NOT EXISTS client_platform  VARCHAR(50);

            UPDATE log_metadata
                SET client_version = LEFT(client_version, 50)
                WHERE client_version IS NOT NULL;

            ALTER TABLE log_metadata
                ALTER COLUMN client_version TYPE VARCHAR(50);
            "#,
        )
        .await?;

        // 删除 client_type 索引 + 列
        db.execute_unprepared("DROP INDEX IF EXISTS idx_log_metadata_client_type")
            .await?;
        db.execute_unprepared("DROP INDEX IF EXISTS idx_log_token_usage_client_type")
            .await?;
        db.execute_unprepared("ALTER TABLE log_metadata DROP COLUMN IF EXISTS client_type")
            .await?;
        db.execute_unprepared("ALTER TABLE log_token_usage DROP COLUMN IF EXISTS client_type")
            .await?;

        Ok(())
    }
}

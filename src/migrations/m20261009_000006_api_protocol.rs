//! 迁移：请求级协议列 — 在 log_requests 表新增 api_protocol 列
//!
//! **背景**：
//! `log_requests.api_type` 记录的是**接入点类型**（anthropic / openai），粒度较粗。
//! OpenAI 接入点下同时存在 Chat Completions 与 Responses 两条线协议，
//! 二者的请求体形状、流式事件族、usage 字段族都不同，日志需要能区分。
//!
//! **新增**：
//! - `log_requests.api_protocol VARCHAR(32) NOT NULL DEFAULT 'anthropic'`
//!   记录本次请求实际使用的协议：`anthropic` / `openai` / `openai_response`
//!
//! **存量数据回填**：按历史 `api_type` 推导——`anthropic` 接入点的请求只能是
//! Anthropic 协议；`openai` 接入点在旧版本中无法区分两条线协议，
//! 统一回填 `openai`（保守取值，不猜测历史请求的真实端点）。
//!
//! 列名与取值刻意不复用 `api_type`，保持「接入点类型」与「请求协议」两个概念正交。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261009_000006_api_protocol"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
            ALTER TABLE log_requests
                ADD COLUMN IF NOT EXISTS api_protocol VARCHAR(32) NOT NULL DEFAULT 'anthropic';

            UPDATE log_requests
               SET api_protocol = 'openai'
             WHERE api_type = 'openai'
               AND api_protocol = 'anthropic';

            CREATE INDEX IF NOT EXISTS idx_log_requests_api_protocol
                ON log_requests (api_protocol);
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            DROP INDEX IF EXISTS idx_log_requests_api_protocol;
            ALTER TABLE log_requests DROP COLUMN IF EXISTS api_protocol;
            "#,
        )
        .await?;
        Ok(())
    }
}

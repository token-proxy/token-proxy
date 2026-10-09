//! 迁移：上游兼容策略 — 在 providers 表新增 normalize_legacy_roles 列
//!
//! **背景**：
//! 部分第三方中转 / 自建网关的上游服务端 SDK 落后于 OpenAI 规范，对较新的消息
//! role（如 `developer`）直接反序列化失败并返回 400。这是上游能力落差，不是代理缺陷。
//!
//! **新增**：
//! - `providers.normalize_legacy_roles BOOLEAN NOT NULL DEFAULT FALSE`
//!
//! 默认 `FALSE`（完全透明）：不改变任何请求语义，保持代理的透明转发契约。
//! 仅当某服务商的上游确实无法处理较新 role 时，才由运维显式开启，
//! 届时代理会把 `developer` 降级为语义等价的 `system`。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261009_000007_provider_compat"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Providers::Table)
                    .add_column(
                        ColumnDef::new(Providers::NormalizeLegacyRoles)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Providers::Table)
                    .drop_column(Providers::NormalizeLegacyRoles)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Providers {
    Table,
    NormalizeLegacyRoles,
}

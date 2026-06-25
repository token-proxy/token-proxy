//! 迁移：日志存储上限 — 在 system_settings 表新增 log_storage_cap_gb 列
//!
//! **新增**：
//! - `system_settings.log_storage_cap_gb SMALLINT`，可为空，None 表示不限制

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260626_000004_storage_cap"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(SystemSettings::Table)
                    .add_column(
                        ColumnDef::new(SystemSettings::LogStorageCapGb)
                            .small_integer()
                            .null(),
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
                    .table(SystemSettings::Table)
                    .drop_column(SystemSettings::LogStorageCapGb)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum SystemSettings {
    Table,
    LogStorageCapGb,
}

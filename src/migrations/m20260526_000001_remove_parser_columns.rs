use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删除 log_conversation_events 表及其索引
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS log_conversation_events CASCADE",
            )
            .await?;

        // 删除 log_metadata 中解析器相关的列
        manager
            .alter_table(
                Table::alter()
                    .table(LogMetadata::Table)
                    .drop_column(LogMetadata::AgentType)
                    .drop_column(LogMetadata::ParentAgentToolUseId)
                    .drop_column(LogMetadata::RequestKind)
                    .drop_column(LogMetadata::PrimaryToolName)
                    .drop_column(LogMetadata::MessagePreview)
                    .drop_column(LogMetadata::MessageFull)
                    .drop_column(LogMetadata::ResponsePreview)
                    .drop_column(LogMetadata::HasThinking)
                    .drop_column(LogMetadata::HasToolUse)
                    .drop_column(LogMetadata::ParserVersion)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 恢复 log_metadata 中被删除的列
        manager
            .alter_table(
                Table::alter()
                    .table(LogMetadata::Table)
                    .add_column(string_null(LogMetadata::AgentType))
                    .add_column(string_null(LogMetadata::ParentAgentToolUseId))
                    .add_column(string_null(LogMetadata::RequestKind))
                    .add_column(string_null(LogMetadata::PrimaryToolName))
                    .add_column(string_null(LogMetadata::MessagePreview))
                    .add_column(string_null(LogMetadata::MessageFull))
                    .add_column(string_null(LogMetadata::ResponsePreview))
                    .add_column(boolean(LogMetadata::HasThinking).default(false))
                    .add_column(boolean(LogMetadata::HasToolUse).default(false))
                    .add_column(string_null(LogMetadata::ParserVersion))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum LogMetadata {
    Table,
    AgentType,
    ParentAgentToolUseId,
    RequestKind,
    PrimaryToolName,
    MessagePreview,
    MessageFull,
    ResponsePreview,
    HasThinking,
    HasToolUse,
    ParserVersion,
}

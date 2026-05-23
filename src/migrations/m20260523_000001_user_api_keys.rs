use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserApiKeys::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
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
mod m20260101_000001_initial;
mod m20260526_000001_remove_parser_columns;

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260101_000001_initial::Migration),
            Box::new(m20260526_000001_remove_parser_columns::Migration),
        ]
    }
}
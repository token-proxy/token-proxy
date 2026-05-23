mod m20260101_000001_initial;
mod m20260523_000001_user_api_keys;

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260101_000001_initial::Migration),
            Box::new(m20260523_000001_user_api_keys::Migration),
        ]
    }
}
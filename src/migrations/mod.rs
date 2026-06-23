//! 数据库迁移定义
//!
//! 使用 `sea-orm-migration` 框架管理数据库 Schema 变更。
//! 迁移按时间顺序编号，每个文件对应一次原子变更。

mod m20260519_000001_initial;
mod m20260618_000002_account_pool;
mod m20260623_000003_client_type;

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260519_000001_initial::Migration),
            Box::new(m20260618_000002_account_pool::Migration),
            Box::new(m20260623_000003_client_type::Migration),
        ]
    }
}

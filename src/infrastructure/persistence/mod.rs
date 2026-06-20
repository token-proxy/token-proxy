//! 持久化层基础设施（基础设施层）
//!
//! 包含 PostgreSQL 分区管理和所有 Repository 的 SeaORM 实现。

pub mod partition_manager;
pub mod repositories;

pub use partition_manager::PartitionManager;

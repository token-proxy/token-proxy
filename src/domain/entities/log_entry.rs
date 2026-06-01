/// LogEntry 等日志相关实体的 re-export
///
/// 实际定义位于 log_metadata、log_content、log_token_usage 模块中，
/// 因为 SeaORM 的 DeriveEntityModel 每个文件只能定义一个 Model。
pub use crate::domain::entities::log_content::Model as LogContent;
pub use crate::domain::entities::log_metadata::Model as LogEntry;
pub use crate::domain::entities::log_token_usage::Model as LogTokenUsage;

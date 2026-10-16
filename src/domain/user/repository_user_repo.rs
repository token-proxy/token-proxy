//! 用户仓储接口 — domain/user/
//!
//! 定义 `UserRepository` trait，提供用户的持久化契约。

use crate::domain::user::User;
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

/// 用户关联数据量（删除守卫的依据）
///
/// 用户在系统里留下的是**审计线索**：他创建的接入点、他调用产生的请求日志、
/// 以他为操作人的审计记录。这些数据一旦删除就无法追溯，因此有任一关联数据时
/// 不允许物理删除，只能禁用（见 `violation_message`）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UserDependents {
    /// 该用户创建的接入点数量（`access_points.created_by`，外键为 ON DELETE RESTRICT）
    pub access_points: i64,
    /// 该用户的代理请求日志数量（`log_requests.user_id`，无外键，删除会留下悬空引用）
    pub logs: i64,
    /// 以该用户为操作人的审计记录数量（`audit_logs.operator_id`）
    pub audits: i64,
}

impl UserDependents {
    /// 是否存在任何关联数据
    pub fn has_any(&self) -> bool {
        self.access_points > 0 || self.logs > 0 || self.audits > 0
    }

    /// 拒绝删除的说明；无关联数据时返回 `None`（可以删除）
    pub fn violation_message(&self) -> Option<String> {
        if !self.has_any() {
            return None;
        }
        let mut parts = Vec::new();
        if self.access_points > 0 {
            parts.push(format!("创建的接入点 {} 个", self.access_points));
        }
        if self.logs > 0 {
            parts.push(format!("请求日志 {} 条", self.logs));
        }
        if self.audits > 0 {
            parts.push(format!("审计记录 {} 条", self.audits));
        }
        Some(format!(
            "该用户存在关联数据（{}），不能删除；请改为禁用（禁用后其登录、API key 与 token 立即失效）",
            parts.join("、")
        ))
    }
}

/// 用户仓储接口
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// 根据 ID 查找用户
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError>;

    /// 根据用户名查找用户
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;

    /// 查找所有用户
    async fn find_all(&self) -> Result<Vec<User>, AppError>;

    /// 保存用户（创建或更新）
    async fn save(&self, user: &User) -> Result<User, AppError>;

    /// 根据 ID 删除用户
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;

    /// 检查用户名是否已存在
    async fn exists_by_username(&self, username: &str) -> Result<bool, AppError>;

    /// 统计用户的关联数据量（物理删除前的守卫）
    async fn count_dependents(&self, id: Uuid) -> Result<UserDependents, AppError>;

    /// 统计启用状态用户数量（保留最后一个可用管理员的守卫）
    async fn count_enabled(&self) -> Result<i64, AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_dependents_allows_delete() {
        let d = UserDependents::default();
        assert!(!d.has_any());
        assert!(d.violation_message().is_none());
    }

    #[test]
    fn test_each_kind_alone_blocks_delete() {
        for d in [
            UserDependents {
                access_points: 1,
                ..Default::default()
            },
            UserDependents {
                logs: 1,
                ..Default::default()
            },
            UserDependents {
                audits: 1,
                ..Default::default()
            },
        ] {
            assert!(d.has_any());
            assert!(d.violation_message().is_some());
        }
    }

    #[test]
    fn test_violation_message_lists_all_counts_and_suggests_disable() {
        let d = UserDependents {
            access_points: 2,
            logs: 100,
            audits: 3,
        };
        let msg = d.violation_message().unwrap();
        assert!(msg.contains("接入点 2 个"));
        assert!(msg.contains("请求日志 100 条"));
        assert!(msg.contains("审计记录 3 条"));
        assert!(msg.contains("禁用"));
    }
}

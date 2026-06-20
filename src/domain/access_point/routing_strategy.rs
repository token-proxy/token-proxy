//! 路由策略值对象 — domain/access_point/
//!
//! 定义 `RoutingStrategy` 枚举（Priority / Weighted），
//! 封装账户池排序逻辑作为领域行为，而不是放在应用层编排中。

use rand::distributions::Distribution;
use sea_orm::prelude::StringLen;
use sea_orm::DeriveActiveEnum;
use sea_orm::EnumIter;
use serde::{Deserialize, Serialize};

use super::access_point_account::AccessPointAccount;
use crate::shared::error::AppError;

/// 路由策略：Priority（按优先级升序）或 Weighted（加权随机 + 降级回退）
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum RoutingStrategy {
    /// 按 priority 升序，值越小优先级越高
    #[default]
    #[sea_orm(string_value = "priority")]
    Priority,
    /// 按 weight 加权随机选择一个账户，其余按 priority 升序作为降级回退
    #[sea_orm(string_value = "weighted")]
    Weighted,
}

impl RoutingStrategy {
    /// 返回枚举的字符串表示
    pub fn as_str(&self) -> &str {
        match self {
            RoutingStrategy::Priority => "priority",
            RoutingStrategy::Weighted => "weighted",
        }
    }

    /// 按路由策略排序账户列表
    ///
    /// - Priority：按 priority 升序（值越小优先级越高）
    /// - Weighted：按 weight 加权随机选择一个账户置于首位，其余按 priority 升序作为降级回退
    pub fn sort_accounts(&self, accounts: &mut [AccessPointAccount]) {
        match self {
            RoutingStrategy::Priority => accounts.sort_by_key(|a| a.priority),
            RoutingStrategy::Weighted => {
                if accounts.len() <= 1 {
                    return;
                }
                // 1. 使用 WeightedIndex 按权重随机选择一个账户
                let weights: Vec<u32> = accounts
                    .iter()
                    .map(|a| if a.weight > 0 { a.weight as u32 } else { 0 })
                    .collect();
                if let Ok(dist) = rand::distributions::WeightedIndex::new(&weights) {
                    let selected = dist.sample(&mut rand::thread_rng());
                    accounts.swap(0, selected);
                }
                // 2. 其余账户按 priority 升序排列作为降级回退
                accounts[1..].sort_by_key(|a| a.priority);
            }
        }
    }
}

impl std::str::FromStr for RoutingStrategy {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "priority" => Ok(RoutingStrategy::Priority),
            "weighted" => Ok(RoutingStrategy::Weighted),
            _ => Err(AppError::Validation(format!(
                "无效的路由策略: {}",
                s
            ))),
        }
    }
}

impl TryFrom<&str> for RoutingStrategy {
    type Error = AppError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

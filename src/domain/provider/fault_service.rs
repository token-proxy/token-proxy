//! 故障检测领域服务 — domain/provider/
//!
//! 依次检查 Provider 的两个故障配置（rate_limit / balance_exhausted），
//! 返回第一个命中的故障原因和预计恢复时间，并执行账户禁用操作。

use axum::http::HeaderMap;
use chrono::{DateTime, FixedOffset};

use super::fault_config::FaultConfig;
use super::{Account, DisabledReason};

/// 故障检测结果
pub enum FaultOutcome {
    /// 未命中任何故障配置
    NoFault,
    /// 命中故障配置，携带原因和预计恢复时间
    Fault {
        reason: DisabledReason,
        available_at: Option<DateTime<FixedOffset>>,
    },
}

/// 故障检测领域服务
///
/// 无状态的领域服务，连接 `FaultConfig` 值对象与 `Account` 实体。
/// 在应用层由 `ProxyPipeline` 调用。
pub struct FaultService;

impl FaultService {
    /// 依次检查 rate_limit_config 和 balance_exhausted_config，
    /// 返回第一个命中的故障原因和预计恢复时间
    pub fn detect(
        status: u16,
        rate_limit_config: Option<&FaultConfig>,
        balance_exhausted_config: Option<&FaultConfig>,
        resp_headers: &HeaderMap,
        resp_body: &[u8],
    ) -> FaultOutcome {
        if let Some(config) = rate_limit_config {
            if config.matches_status(status) {
                return FaultOutcome::Fault {
                    reason: DisabledReason::RateLimited,
                    available_at: config
                        .recover_type
                        .calculate_available_at(resp_headers, resp_body),
                };
            }
        }
        if let Some(config) = balance_exhausted_config {
            if config.matches_status(status) {
                return FaultOutcome::Fault {
                    reason: DisabledReason::BalanceExhausted,
                    available_at: config
                        .recover_type
                        .calculate_available_at(resp_headers, resp_body),
                };
            }
        }
        FaultOutcome::NoFault
    }

    /// 对账户执行禁用操作，委托给 `Account::disable_for()`
    pub fn disable_account(
        account: &mut Account,
        reason: DisabledReason,
        available_at: Option<DateTime<FixedOffset>>,
    ) {
        account.disable_for(reason, available_at);
    }
}

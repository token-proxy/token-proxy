//! 候选账号选择器 — application/proxy/
//!
//! 在 `ProxyPipeline::execute` 的重试循环中提供一个迭代器接口，
//! 每次 `next` 完成"加载 Account → 跳过不可用 → 加载 Provider → 解密 API Key"四步，
//! 把过去散落在循环体内的 4a-4c 步骤封装为一阶组件。
//!
//! 为什么是异步迭代器风格而不是 `Iterator` trait：每次 `next` 涉及多次 await
//! （Repository 查询、加密服务异步解密），Rust 当前的 `Iterator` 不支持 async。
//! 用 `async fn next(&mut self)` 是当下最简单的等价表达。

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::access_point::AccessPointEx;
use crate::domain::provider::repository::{AccountRepository, ProviderRepository};
use crate::domain::provider::{Account, Provider};
use crate::domain::shared::EncryptionService;
use crate::shared::error::AppError;

/// 一个可发起上游请求的候选
///
/// 由 `AccountSelector::next` 构造并交给上游转发流程使用。
/// 持有完整账号信息（用于故障禁用）、Provider（用于 base_url 与故障配置）和明文 API key。
pub struct AccountCandidate {
    pub account: Account,
    pub account_id: Uuid,
    pub provider: Provider,
    pub upstream_key: String,
}

/// 候选账号选择器（异步迭代器风格）
pub struct AccountSelector<'a> {
    accounts:
        std::slice::Iter<'a, crate::domain::access_point::access_point_account::AccessPointAccount>,
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    encryption_service: Arc<dyn EncryptionService>,
}

impl<'a> AccountSelector<'a> {
    pub fn new(
        access_point: &'a AccessPointEx,
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        encryption_service: Arc<dyn EncryptionService>,
    ) -> Self {
        AccountSelector {
            accounts: access_point.accounts.iter(),
            account_repo,
            provider_repo,
            encryption_service,
        }
    }

    /// 取下一个可用的候选账号
    ///
    /// 返回 `Ok(None)` 表示候选池已耗尽，
    /// `Ok(Some(_))` 是一个完整的可发起请求的候选，
    /// `Err(_)` 是基础设施错误（数据库查询失败、解密失败等），应当中止整个请求处理。
    ///
    /// 内部自动跳过 `account.is_available() == false` 的账号（自动禁用或手动禁用的）。
    pub async fn next(&mut self) -> Result<Option<AccountCandidate>, AppError> {
        for entry in self.accounts.by_ref() {
            // 加载账号
            let account = self
                .account_repo
                .find_by_id(entry.account_id)
                .await?
                .ok_or_else(|| AppError::Internal("账户池中关联的 Account 未找到".to_string()))?;

            // 跳过不可用账号（自动禁用、手动禁用等）
            if !account.is_available() {
                continue;
            }

            // 加载 Provider
            let provider = self
                .provider_repo
                .find_by_id(account.provider_id)
                .await?
                .ok_or_else(|| AppError::Internal("后端关联的 Provider 未找到".to_string()))?;

            // 解密 API key
            let encrypted_key = self
                .account_repo
                .get_encrypted_api_key(entry.account_id)
                .await?;
            let decrypted = self
                .encryption_service
                .decrypt(&encrypted_key)
                .await
                .map_err(|e| AppError::Encryption(e.to_string()))?;
            let upstream_key = String::from_utf8(decrypted)
                .map_err(|_| AppError::Internal("API Key 解码失败".to_string()))?;

            return Ok(Some(AccountCandidate {
                account,
                account_id: entry.account_id,
                provider,
                upstream_key,
            }));
        }
        Ok(None)
    }
}

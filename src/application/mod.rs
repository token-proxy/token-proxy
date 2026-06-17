use std::sync::Arc;

pub mod access_point;
pub mod auth;
pub mod log;
pub mod provider;
pub mod proxy;
pub mod system;
pub mod user;

use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::domain::log::AuditLogRepository;
use crate::domain::log::LogRepository;
use crate::domain::log::LogTokenUsageRepository;
use crate::domain::system::SystemSettingsRepository;
use crate::infrastructure::auth::JwtService;
use crate::infrastructure::http_client::ProxyClient;

use access_point::AccessPointService;
use auth::AuthService;
use log::LogService;
use provider::AccountService;
use provider::ProviderService;
use proxy::ProxyPipeline;
use system::SettingsService;
use user::UserApiKeyService;
use user::UserService;

/// 应用全局共享状态
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<DatabaseConnection>,
    pub provider_service: Arc<ProviderService>,
    pub account_service: Arc<AccountService>,
    pub user_service: Arc<UserService>,
    pub user_api_key_service: Arc<UserApiKeyService>,
    pub access_point_service: Arc<AccessPointService>,
    pub auth_service: Arc<AuthService>,
    pub proxy_pipeline: Arc<ProxyPipeline>,
    pub log_service: Arc<LogService>,
    pub log_repo: Arc<dyn LogRepository>,
    pub log_token_usage_repo: Arc<dyn LogTokenUsageRepository>,
    pub audit_log_repo: Arc<dyn AuditLogRepository>,
    pub system_settings_repo: Arc<dyn SystemSettingsRepository>,
    pub settings_service: Arc<SettingsService>,
    pub jwt_service: Arc<JwtService>,
    pub proxy_client: Arc<ProxyClient>,
}

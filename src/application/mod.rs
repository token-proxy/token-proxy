use std::sync::Arc;

pub mod dto;
pub mod services;

use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::domain::log::AuditLogRepository;
use crate::domain::log::LogRepository;
use crate::domain::log::LogTokenUsageRepository;
use crate::infrastructure::auth::jwt::JwtService;
use crate::infrastructure::http_client::proxy_client::ProxyClient;

use services::access_point_service::AccessPointService;
use services::account_service::AccountService;
use services::auth_service::AuthService;
use services::log_service::LogService;
use services::provider_service::ProviderService;
use services::proxy_pipeline::ProxyPipeline;
use services::user_api_key_service::UserApiKeyService;
use services::user_service::UserService;

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
    pub jwt_service: Arc<JwtService>,
    pub proxy_client: Arc<ProxyClient>,
}

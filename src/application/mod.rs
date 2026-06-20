//! 应用层 — 用例编排层
//!
//! 编排领域对象的协作流程，不包含业务判断。
//! 子模块对应各个聚合：access_point、auth、log、provider、proxy、system、user。
//! 本层同时定义 AppState（应用全局共享状态）和 DTO。

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
use crate::domain::access_point::SessionAffinityRepository;
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
    pub settings_service: Arc<SettingsService>,
    pub jwt_service: Arc<JwtService>,
    pub proxy_client: Arc<ProxyClient>,
    pub session_affinity_repo: Arc<dyn SessionAffinityRepository>,
}

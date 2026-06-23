//! 应用层 — 用例编排层
//!
//! 编排领域对象的协作流程，不包含业务判断。
//! 子模块对应各个聚合：access_point、auth、log、provider、proxy、system、user。
//! 本层同时定义 AppState（应用全局共享状态）和 DTO。

use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI64;
use std::sync::Arc;

pub mod access_point;
pub mod auth;
pub mod dashboard;
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
use dashboard::DashboardService;
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
    pub dashboard_service: Arc<DashboardService>,
    pub settings_service: Arc<SettingsService>,
    pub jwt_service: Arc<JwtService>,
    pub proxy_client: Arc<ProxyClient>,
    pub session_affinity_repo: Arc<dyn SessionAffinityRepository>,
    /// 是否正在优雅关闭（由信号处理任务置位，健康检查端点查询）
    pub shutting_down: Arc<AtomicBool>,
    /// 飞行中的异步数据库写入计数（代理日志、会话粘滞）
    ///
    /// 每次 fire-and-forget 的 `tokio::spawn` 入队前 +1，任务尾 -1。
    /// 主线程在 axum 排空连接后轮询此计数归零，确保所有写入落库再退出。
    pub in_flight_writes: Arc<AtomicI64>,
}

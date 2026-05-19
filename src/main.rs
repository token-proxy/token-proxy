use std::sync::Arc;

use axum::Router;
use sea_orm::Database;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use token_proxy::application::services::access_point_service::AccessPointService;
use token_proxy::application::services::account_service::AccountService;
use token_proxy::application::services::auth_service::AuthService;
use token_proxy::application::services::log_service::LogService;
use token_proxy::application::services::provider_service::ProviderService;
use token_proxy::application::services::proxy_service::ProxyService;
use token_proxy::application::services::user_service::UserService;
use token_proxy::application::AppState;
use token_proxy::config::Config;
use token_proxy::domain::repositories::access_point_repository::AccessPointRepository;
use token_proxy::domain::repositories::account_repository::AccountRepository;
use token_proxy::domain::repositories::log_repository::LogRepository;
use token_proxy::domain::repositories::provider_repository::ProviderRepository;
use token_proxy::domain::repositories::refresh_token_repository::RefreshTokenRepository;
use token_proxy::domain::repositories::user_repository::UserRepository;
use token_proxy::domain::services::encryption_service::EncryptionService;
use token_proxy::infrastructure::auth::jwt::JwtService;
use token_proxy::infrastructure::encryption::aes256_gcm::Aes256GcmEncryptionService;
use token_proxy::infrastructure::http_client::proxy_client::ProxyClient;
use token_proxy::infrastructure::persistence::repositories::access_point_repository::SeaOrmAccessPointRepository;
use token_proxy::infrastructure::persistence::repositories::account_repository::SeaOrmAccountRepository;
use token_proxy::infrastructure::persistence::repositories::log_repository::SeaOrmLogRepository;
use token_proxy::infrastructure::persistence::repositories::provider_repository::SeaOrmProviderRepository;
use token_proxy::infrastructure::persistence::repositories::refresh_token_repository::SeaOrmRefreshTokenRepository;
use token_proxy::infrastructure::persistence::repositories::user_repository::SeaOrmUserRepository;
use token_proxy::presentation::routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载 .env（可选）
    dotenvy::dotenv().ok();

    // 初始化 tracing 日志
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(env_filter)
        .init();

    tracing::info!("token-proxy 服务启动中...");

    // 加载配置
    let config = Config::from_env()?;

    // 连接数据库
    let db = Arc::new(Database::connect(&config.database_url).await?);
    tracing::info!("数据库连接成功");

    // ─── 创建 Infrastructure 组件 ───

    let encryption_service: Arc<dyn EncryptionService> =
        Arc::new(Aes256GcmEncryptionService::new(config.encryption_key));

    let jwt_service = Arc::new(JwtService::new(
        config.jwt_secret.clone(),
        config.jwt_access_expiry.as_secs() as i64,
        config.jwt_refresh_expiry.as_secs() as i64,
    ));

    let proxy_client = Arc::new(ProxyClient::new());

    // ─── 创建 Repositories ───

    let provider_repo: Arc<dyn ProviderRepository> =
        Arc::new(SeaOrmProviderRepository::new(db.clone()));

    let account_repo: Arc<dyn AccountRepository> =
        Arc::new(SeaOrmAccountRepository::new(db.clone()));

    let user_repo: Arc<dyn UserRepository> =
        Arc::new(SeaOrmUserRepository::new(db.clone()));

    let access_point_repo: Arc<dyn AccessPointRepository> =
        Arc::new(SeaOrmAccessPointRepository::new(db.clone()));

    let refresh_token_repo: Arc<dyn RefreshTokenRepository> =
        Arc::new(SeaOrmRefreshTokenRepository::new(db.clone()));

    let log_repo: Arc<dyn LogRepository> =
        Arc::new(SeaOrmLogRepository::new(db.clone()));

    // ─── 创建 Application Services ───

    let provider_service = Arc::new(ProviderService::new(
        provider_repo.clone(),
        account_repo.clone(),
    ));

    let account_service = Arc::new(AccountService::new(
        account_repo.clone(),
        provider_repo.clone(),
        encryption_service.clone(),
    ));

    let user_service = Arc::new(UserService::new(user_repo.clone()));

    let access_point_service = Arc::new(AccessPointService::new(
        access_point_repo.clone(),
        provider_repo.clone(),
        account_repo.clone(),
    ));

    let auth_service = Arc::new(AuthService::new(
        user_repo.clone(),
        refresh_token_repo.clone(),
        jwt_service.clone(),
    ));

    let proxy_service = Arc::new(ProxyService::new(
        access_point_repo.clone(),
        provider_repo.clone(),
        account_repo.clone(),
        encryption_service.clone(),
    ));

    let log_service = Arc::new(LogService::new(log_repo.clone()));

    // ─── 构建 AppState ───

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        provider_service,
        account_service,
        user_service,
        access_point_service,
        auth_service,
        proxy_service,
        log_service,
        jwt_service,
        proxy_client,
    };

    // ─── 构建 Router ───

    let app = Router::new()
        .merge(routes::build(state))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // ─── 启动 ───

    let addr = format!("0.0.0.0:{}", config.server_port);
    tracing::info!("服务监听地址: {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
use std::sync::Arc;

use axum::Router;
use sea_orm::Database;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use token_proxy::application::access_point::AccessPointService;
use token_proxy::application::auth::AuthService;
use token_proxy::application::log::LogService;
use token_proxy::application::provider::AccountService;
use token_proxy::application::provider::ProviderService;
use token_proxy::application::proxy::ProxyPipeline;
use token_proxy::application::user::UserApiKeyService;
use token_proxy::application::user::UserService;
use token_proxy::application::AppState;
use token_proxy::config::Config;
use token_proxy::domain::access_point::repository::AccessPointRepository;
use token_proxy::domain::log::AuditLogRepository;
use token_proxy::domain::log::LogRepository;
use token_proxy::domain::log::LogTokenUsageRepository;
use token_proxy::domain::provider::repository::AccountRepository;
use token_proxy::domain::provider::repository::ProviderRepository;
use token_proxy::domain::shared::EncryptionService;
use token_proxy::domain::user::RefreshTokenRepository;
use token_proxy::domain::user::UserApiKeyRepository;
use token_proxy::domain::user::UserRepository;
use token_proxy::infrastructure::auth::JwtService;
use token_proxy::infrastructure::encryption::Aes256GcmEncryptionService;
use token_proxy::infrastructure::http_client::ProxyClient;
use token_proxy::infrastructure::persistence::repositories::SeaOrmAccessPointRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmAccountRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmAuditLogRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmLogRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmLogTokenUsageRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmProviderRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmRefreshTokenRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmUserApiKeyRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmUserRepository;
use token_proxy::infrastructure::persistence::PartitionManager;
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

    // ─── 迁移命令行模式 ───
    // 用法: cargo make migrate <up|down|fresh|status>
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "migrate" {
        let subcommand = args.get(2).map(|s| s.as_str()).unwrap_or("up");

        let config = Config::from_env()?;
        let db = Database::connect(&config.database_url).await?;
        tracing::info!("数据库连接成功");

        use sea_orm_migration::MigratorTrait;
        match subcommand {
            "up" => {
                token_proxy::migrations::Migrator::up(&db, None).await?;
                println!("迁移完成 (up)");
            }
            "down" => {
                let steps = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
                token_proxy::migrations::Migrator::down(&db, Some(steps)).await?;
                println!("迁移完成 (down) - 回滚 {} 步", steps);
            }
            "fresh" => {
                token_proxy::migrations::Migrator::fresh(&db).await?;
                println!("迁移完成 (fresh)");
            }
            "status" => {
                token_proxy::migrations::Migrator::status(&db).await?;
            }
            other => {
                eprintln!("未知迁移命令: {}", other);
                eprintln!("用法: cargo make migrate -- <up|down|fresh|status> [steps]");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    tracing::info!("token-proxy 服务启动中...");

    // 加载配置
    let config = Config::from_env()?;

    // 连接数据库
    let db = Arc::new(Database::connect(&config.database_url).await?);
    tracing::info!("数据库连接成功");

    // 执行数据库迁移
    use sea_orm_migration::MigratorTrait;
    token_proxy::migrations::Migrator::up(&*db, None).await?;
    tracing::info!("数据库迁移完成");

    // ─── 分区管理器初始化 ───

    let partition_manager = Arc::new(PartitionManager::new(
        db.clone(),
        config.partition_premake_months,
        config.partition_retention_months,
    ));

    // 启动时同步执行分区维护
    match partition_manager.run_maintenance().await {
        Ok(result) => {
            if !result.created.is_empty() {
                tracing::info!("创建分区: {:?}", result.created);
            }
            if !result.dropped.is_empty() {
                tracing::info!("清理分区: {:?}", result.dropped);
            }
            tracing::info!("分区初始化完成");
        }
        Err(e) => {
            tracing::error!("分区初始化失败: {}", e);
        }
    }

    // 启动后台定时分区维护任务
    let pm = partition_manager.clone();
    let check_interval = std::time::Duration::from_secs(config.partition_check_interval_secs);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(check_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            match pm.run_maintenance_with_lock().await {
                Ok(result) => {
                    if !result.created.is_empty() {
                        tracing::info!("创建分区: {:?}", result.created);
                    }
                    if !result.dropped.is_empty() {
                        tracing::info!("清理分区: {:?}", result.dropped);
                    }
                }
                Err(e) => {
                    tracing::error!("分区维护失败: {}", e);
                }
            }
        }
    });

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

    let user_repo: Arc<dyn UserRepository> = Arc::new(SeaOrmUserRepository::new(db.clone()));

    let access_point_repo: Arc<dyn AccessPointRepository> =
        Arc::new(SeaOrmAccessPointRepository::new(db.clone()));

    let refresh_token_repo: Arc<dyn RefreshTokenRepository> =
        Arc::new(SeaOrmRefreshTokenRepository::new(db.clone()));

    // 启动后台定时任务：清理过期的 refresh_token
    let token_repo_cleanup = refresh_token_repo.clone();
    let token_cleanup_interval =
        std::time::Duration::from_secs(config.partition_check_interval_secs);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(token_cleanup_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // 首个 tick 立即触发, 跳过它避免启动瞬间执行
        interval.tick().await;
        loop {
            interval.tick().await;
            match token_repo_cleanup.delete_expired().await {
                Ok(n) if n > 0 => {
                    tracing::info!("清理过期 refresh_token: {} 条", n);
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("清理过期 refresh_token 失败: {}", e);
                }
            }
        }
    });

    let log_repo: Arc<dyn LogRepository> = Arc::new(SeaOrmLogRepository::new(db.clone()));

    let log_token_usage_repo: Arc<dyn LogTokenUsageRepository> =
        Arc::new(SeaOrmLogTokenUsageRepository::new(db.clone()));

    let audit_log_repo: Arc<dyn AuditLogRepository> =
        Arc::new(SeaOrmAuditLogRepository::new(db.clone()));

    let user_api_key_repo: Arc<dyn UserApiKeyRepository> =
        Arc::new(SeaOrmUserApiKeyRepository::new(db.clone()));

    // ─── 创建 Application Services ───

    let provider_service = Arc::new(ProviderService::new(
        provider_repo.clone(),
        account_repo.clone(),
        audit_log_repo.clone(),
        encryption_service.clone(),
    ));

    let account_service = Arc::new(AccountService::new(
        account_repo.clone(),
        provider_repo.clone(),
        encryption_service.clone(),
    ));

    let user_service = Arc::new(UserService::new(user_repo.clone(), audit_log_repo.clone()));

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

    let log_service = Arc::new(LogService::new(
        log_repo.clone(),
        log_token_usage_repo.clone(),
        user_repo.clone(),
        access_point_repo.clone(),
    ));

    let proxy_pipeline = Arc::new(ProxyPipeline::new(
        access_point_repo.clone(),
        encryption_service.clone(),
        proxy_client.clone(),
        log_service.clone(),
    ));

    let user_api_key_service = Arc::new(UserApiKeyService::new(
        user_api_key_repo.clone(),
        audit_log_repo.clone(),
    ));

    // ─── 首次启动：创建默认 admin 用户 ───

    let users = user_repo
        .find_all()
        .await
        .map_err(|e| format!("查询用户失败: {}", e))?;

    if users.is_empty() {
        let password = generate_random_password(12);
        let password_hash = token_proxy::infrastructure::auth::password::hash_password(&password)
            .map_err(|e| format!("密码哈希失败: {}", e))?;

        let admin = token_proxy::domain::user::User::new(
            "admin".to_string(),
            "管理员".to_string(),
            password_hash,
        );
        user_repo
            .save(&admin)
            .await
            .map_err(|e| format!("创建默认管理员失败: {}", e))?;

        println!();
        println!("========================================");
        println!("  默认管理员账号已创建");
        println!("  账号: admin");
        println!("  密码: {}", password);
        println!("  请登录后立即修改密码");
        println!("========================================");
        println!();
    }

    // ─── 构建 AppState ───

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        provider_service,
        account_service,
        user_service,
        user_api_key_service,
        access_point_service,
        auth_service,
        proxy_pipeline,
        log_service,
        log_repo,
        log_token_usage_repo,
        audit_log_repo,
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

fn generate_random_password(len: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

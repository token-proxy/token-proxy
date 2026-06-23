use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::Router;
use sea_orm::Database;
use tokio::signal;
use tokio::sync::broadcast;
use tokio::sync::watch;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use token_proxy::application::access_point::AccessPointService;
use token_proxy::application::auth::AuthService;
use token_proxy::application::dashboard::DashboardService;
use token_proxy::application::log::dto::NewLogEvent;
use token_proxy::application::log::LogService;
use token_proxy::application::provider::AccountService;
use token_proxy::application::provider::ProviderService;
use token_proxy::application::proxy::ProxyPipeline;
use token_proxy::application::system::SettingsService;
use token_proxy::application::user::UserApiKeyService;
use token_proxy::application::user::UserService;
use token_proxy::application::AppState;
use token_proxy::config::Config;
use token_proxy::domain::access_point::repository::AccessPointRepository;
use token_proxy::domain::access_point::SessionAffinityRepository;
use token_proxy::domain::log::AuditLogRepository;
use token_proxy::domain::log::LogRepository;
use token_proxy::domain::log::LogTokenUsageRepository;
use token_proxy::domain::provider::repository::AccountRepository;
use token_proxy::domain::provider::repository::ProviderRepository;
use token_proxy::domain::shared::EncryptionService;
use token_proxy::domain::system::SystemSettingsRepository;
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
use token_proxy::infrastructure::persistence::repositories::SeaOrmSessionAffinityRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmSystemSettingsRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmUserApiKeyRepository;
use token_proxy::infrastructure::persistence::repositories::SeaOrmUserRepository;
use token_proxy::infrastructure::persistence::PartitionManager;
use token_proxy::presentation::routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // jsonwebtoken 10.x 不再自动选择加密后端，必须在任何 JWT 操作之前显式安装
    // 否则 CryptoProvider::from_crate_features() 会 panic
    jsonwebtoken::crypto::aws_lc::DEFAULT_PROVIDER
        .install_default()
        .ok();

    // 加载 .env（可选）
    dotenvy::dotenv().ok();

    // 加载配置（须在 tracing 初始化之前，以便 LOG_LEVEL 生效）
    let config = Config::from_env()?;

    // 初始化 tracing 日志 — 由 LOG_LEVEL 环境变量控制
    let env_filter = EnvFilter::new(&config.log_level);
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(env_filter)
        .init();

    // ─── 迁移命令行模式 ───
    // 用法: cargo make migrate <up|down|fresh|status>
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "migrate" {
        let subcommand = args.get(2).map(|s| s.as_str()).unwrap_or("up");

        let db = Database::connect(&config.database_url).await?;
        tracing::info!("迁移模式数据库连接成功");

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

    tracing::info!("token-proxy 服务正在启动");

    // 连接数据库
    let db = Arc::new(Database::connect(&config.database_url).await?);
    tracing::info!("数据库连接成功");

    // 执行数据库迁移
    use sea_orm_migration::MigratorTrait;
    token_proxy::migrations::Migrator::up(&*db, None).await?;
    tracing::info!("数据库迁移完成");

    // ─── 优雅关闭信号协调 ───
    //
    // `shutdown_tx`：广播关闭信号到 axum 和所有后台任务
    // `shutting_down`：供健康检查端点查询当前状态
    // `in_flight_writes`：追踪所有 fire-and-forget 的数据库写入任务（代理日志、会话粘滞），
    //                    主线程在 axum 排空连接后轮询此计数归零，确保数据落库再退出
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let shutting_down = Arc::new(AtomicBool::new(false));
    let in_flight_writes = Arc::new(AtomicI64::new(0));

    // 注册 SIGTERM 监听（K8s 滚动更新发送的标准信号）
    {
        let tx = shutdown_tx.clone();
        let flag = shutting_down.clone();
        tokio::spawn(async move {
            let mut sig = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("无法注册 SIGTERM 信号处理");
            sig.recv().await;
            tracing::info!("收到 SIGTERM 信号，开始优雅关闭");
            flag.store(true, Ordering::Release);
            let _ = tx.send(true);
        });
    }

    // 注册 SIGINT 监听（本地开发 Ctrl+C）
    {
        let tx = shutdown_tx.clone();
        let flag = shutting_down.clone();
        tokio::spawn(async move {
            let mut sig = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .expect("无法注册 SIGINT 信号处理");
            sig.recv().await;
            tracing::info!("收到 SIGINT 信号，开始优雅关闭");
            flag.store(true, Ordering::Release);
            let _ = tx.send(true);
        });
    }

    // ─── 分区管理器初始化 ───

    let partition_manager = Arc::new(PartitionManager::new(
        db.clone(),
        config.partition_premake_months,
    ));

    // 启动时同步执行分区维护（使用环境变量默认保留月数）
    let initial_retention = config.partition_retention_months;
    match partition_manager.run_maintenance(initial_retention).await {
        Ok(result) => {
            if !result.created.is_empty() {
                tracing::info!(created = ?result.created, "启动时创建分区");
            }
            if !result.dropped.is_empty() {
                tracing::info!(dropped = ?result.dropped, "启动时清理过期分区");
            }
            tracing::info!("分区初始化完成");
        }
        Err(e) => {
            tracing::error!(error = %e, "分区初始化失败");
        }
    }

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
    {
        let mut task_shutdown_rx = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(token_cleanup_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            // 首个 tick 立即触发，跳过它避免启动瞬间执行
            interval.tick().await;
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        match token_repo_cleanup.delete_expired().await {
                            Ok(n) if n > 0 => {
                                tracing::info!(count = %n, "清理过期 refresh_token");
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(error = %e, "清理过期 refresh_token 失败");
                            }
                        }
                    }
                    _ = task_shutdown_rx.changed() => {
                        tracing::info!("后台任务退出：refresh_token 清理");
                        break;
                    }
                }
            }
        });
    }

    let log_repo: Arc<dyn LogRepository> = Arc::new(SeaOrmLogRepository::new(db.clone()));

    let log_token_usage_repo: Arc<dyn LogTokenUsageRepository> =
        Arc::new(SeaOrmLogTokenUsageRepository::new(db.clone()));

    let audit_log_repo: Arc<dyn AuditLogRepository> =
        Arc::new(SeaOrmAuditLogRepository::new(db.clone()));

    let system_settings_repo: Arc<dyn SystemSettingsRepository> =
        Arc::new(SeaOrmSystemSettingsRepository::new(db.clone()));

    let user_api_key_repo: Arc<dyn UserApiKeyRepository> =
        Arc::new(SeaOrmUserApiKeyRepository::new(db.clone()));

    let session_affinity_repo: Arc<dyn SessionAffinityRepository> =
        Arc::new(SeaOrmSessionAffinityRepository::new(db.clone()));

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
        access_point_repo.clone(),
        encryption_service.clone(),
        audit_log_repo.clone(),
    ));

    let user_service = Arc::new(UserService::new(user_repo.clone(), audit_log_repo.clone()));

    let access_point_service = Arc::new(AccessPointService::new(access_point_repo.clone()));

    let auth_service = Arc::new(AuthService::new(
        user_repo.clone(),
        refresh_token_repo.clone(),
        jwt_service.clone(),
    ));

    // ─── 日志事件广播 channel（SSE 实时推送）───
    let (log_event_tx, _log_event_rx) = broadcast::channel::<NewLogEvent>(256);

    let log_service = Arc::new(LogService::new(
        log_repo.clone(),
        log_token_usage_repo.clone(),
        user_repo.clone(),
        access_point_repo.clone(),
        log_event_tx.clone(),
    ));

    let dashboard_service = Arc::new(DashboardService::new(log_repo.clone()));

    let proxy_pipeline = Arc::new(ProxyPipeline::new(
        access_point_repo.clone(),
        provider_repo.clone(),
        account_repo.clone(),
        encryption_service.clone(),
        proxy_client.clone(),
        log_service.clone(),
        session_affinity_repo.clone(),
        token_proxy::application::proxy::TrackedSpawner::new(in_flight_writes.clone()),
        shutting_down.clone(),
    ));

    let user_api_key_service = Arc::new(UserApiKeyService::new(
        user_api_key_repo.clone(),
        audit_log_repo.clone(),
    ));

    let settings_service = Arc::new(SettingsService::new(
        system_settings_repo.clone(),
        audit_log_repo.clone(),
    ));

    // 启动后台定时任务：清理过期的 session_affinity
    let sa_repo_cleanup = session_affinity_repo.clone();
    let sa_cleanup_interval = std::time::Duration::from_secs(config.partition_check_interval_secs);
    {
        let mut task_shutdown_rx = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(sa_cleanup_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval.tick().await;
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        match sa_repo_cleanup
                            .delete_stale(chrono::Duration::days(7))
                            .await
                        {
                            Ok(n) if n > 0 => {
                                tracing::info!(count = %n, "清理过期 session_affinity");
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(error = %e, "清理 session_affinity 失败");
                            }
                        }
                    }
                    _ = task_shutdown_rx.changed() => {
                        tracing::info!("后台任务退出：session_affinity 清理");
                        break;
                    }
                }
            }
        });
    }

    // 启动后台定时分区维护任务
    let pm = partition_manager.clone();
    let ss_repo = system_settings_repo.clone();
    let check_interval = std::time::Duration::from_secs(config.partition_check_interval_secs);
    {
        let mut task_shutdown_rx = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let retention = ss_repo
                            .get()
                            .await
                            .map(|s| s.log_retention_months)
                            .unwrap_or(12);
                        match pm.run_maintenance_with_lock(retention).await {
                            Ok(result) => {
                                if !result.created.is_empty() {
                                    tracing::info!(created = ?result.created, "分区维护创建分区");
                                }
                                if !result.dropped.is_empty() {
                                    tracing::info!(dropped = ?result.dropped, "分区维护清理过期分区");
                                }
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "分区维护失败");
                            }
                        }
                    }
                    _ = task_shutdown_rx.changed() => {
                        tracing::info!("后台任务退出：分区维护");
                        break;
                    }
                }
            }
        });
    }

    // 启动后台定时任务：自动恢复已到恢复时间的账号
    let ar_account_service = account_service.clone();
    let recover_interval = std::time::Duration::from_secs(config.partition_check_interval_secs);
    {
        let mut task_shutdown_rx = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(recover_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        match ar_account_service.recover_expired().await {
                            Ok(0) => {}
                            Ok(n) => tracing::info!(recovered = n, "账号自动恢复完成"),
                            Err(e) => tracing::error!(error = %e, "账号自动恢复失败"),
                        }
                    }
                    _ = task_shutdown_rx.changed() => {
                        tracing::info!("后台任务退出：账号自动恢复");
                        break;
                    }
                }
            }
        });
    }

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
        dashboard_service,
        settings_service,
        jwt_service,
        proxy_client,
        session_affinity_repo,
        shutting_down: shutting_down.clone(),
        in_flight_writes: in_flight_writes.clone(),
        log_event_tx: log_event_tx.clone(),
        shutdown_rx: shutdown_rx.clone(),
    };

    // ─── 构建 Router ───

    // ─── HTTP 请求日志 ───
    // TraceLayer 仅为每个请求记录 method、uri、status、latency 和 request_id。
    // 不记录请求/响应 header 和 body，避免泄露 Authorization、API key 等敏感信息。
    let trace_layer =
        TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
            let request_id = Uuid::new_v4().to_string();
            info_span!(
                "http_request",
                http.method = %request.method(),
                http.uri = %request.uri(),
                request_id = %request_id,
            )
        });

    let app = Router::new()
        .merge(routes::build(state))
        .layer(CorsLayer::permissive())
        .layer(trace_layer);

    // ─── 启动 ───

    let addr = format!("0.0.0.0:{}", config.server_port);
    tracing::info!(address = %addr, "服务开始监听");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // ─── 启动 axum 并接入优雅关闭 ───
    //
    // `with_graceful_shutdown` 会在 shutdown 信号到达时停止接受新连接，
    // 并等待所有现有 HTTP handler 自然返回（包括 SSE 流自然结束）。
    {
        let mut axum_shutdown_rx = shutdown_rx.clone();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                // watch::Receiver::changed() 在 sender 发送新值后立即 resolve
                while !*axum_shutdown_rx.borrow_and_update() {
                    if axum_shutdown_rx.changed().await.is_err() {
                        break;
                    }
                }
                tracing::info!("已停止接受新连接，等待现有请求完成...");
            })
            .await?;
    }

    tracing::info!("所有 HTTP 连接已关闭");

    // ─── 等待异步数据库写入落库 ───
    //
    // axum 的 graceful shutdown 只保证 handler 返回，
    // 但 handler 内部通过 `tokio::spawn` 提交的代理日志、会话粘滞写入仍在飞行中。
    // 通过 `in_flight_writes` 计数器轮询，确保全部落库再退出进程。
    let poll_interval = std::time::Duration::from_millis(100);
    loop {
        let remaining = in_flight_writes.load(Ordering::Acquire);
        if remaining == 0 {
            break;
        }
        tracing::info!(remaining = %remaining, "等待异步数据库写入完成...");
        tokio::time::sleep(poll_interval).await;
    }

    tracing::info!("服务已完全关闭");

    Ok(())
}

fn generate_random_password(len: usize) -> String {
    use rand::RngExt;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

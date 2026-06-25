# Token Proxy 架构文档

此工程使用 Rust + axum 框架，在后端采用领域驱动设计四层架构（Domain / Application / Infrastructure / Presentation），前端使用
React + TypeScript + Vite + Semi Design 构建 SPA，数据库使用 PostgreSQL 17，log_metadata 表通过 PostgreSQL
原生分区语法（PARTITION BY RANGE）按月分区，由应用层 PartitionManager 自动管理分区生命周期。接入点采用账户池架构，通过
access_point_accounts 多对多关联实现多账号故障转移和负载分发，模型路由从线性列表升级为二维路由网格（source_model x
provider_id）。

## 目录结构总览

```
├── src/                    # 后端 Rust 核心代码 (~170 个 .rs 文件)
├── src-dashboard/          # 前端管理面板 SPA (~73 个 .ts/.tsx 源文件)
├── public/                 # 前端静态资源 (favicon, icons)
├── index.html              # 前端 HTML 入口 (Vite)
├── vite.config.ts          # Vite 构建配置
├── tsconfig.json           # TypeScript 根配置
├── tsconfig.app.json       # TypeScript 应用配置
├── tsconfig.node.json      # TypeScript Node 配置
├── eslint.config.js        # ESLint 配置
├── .prettierrc             # Prettier 前端格式化配置
├── .prettierignore         # Prettier 忽略规则
├── cliff.toml              # git-cliff 变更日志生成配置
├── package.json            # 前端依赖声明
├── migration/              # 数据库迁移 (独立 workspace crate)
├── target/                 # Rust 构建产物
├── node_modules/           # 前端依赖 (root 级, 供 cargo-make 使用)
├── .claude/                # Claude Code 项目配置 (含 release 发布技能)
├── .github/                # GitHub CI 工作流和 Dependabot 配置
├── Cargo.toml              # Rust workspace 定义 (v0.0.0, license Apache-2.0)
├── Cargo.lock              # Rust 依赖锁文件
├── rust-toolchain.toml     # Rust 工具链固定 (1.96, clippy + rustfmt)
├── Makefile.toml           # cargo-make 任务编排
├── Dockerfile              # 多阶段 Docker 构建 (Node 22 → Rust 1.89 → Alpine 3.21)
├── .dockerignore           # Docker 构建上下文优化
└── product.md              # 产品需求文档
```

## 后端架构详解 (src/)

后端按照**领域驱动设计 (DDD) 四层架构**组织，遵循依赖反转原则。

```
src/
├── domain/                 # 领域层 (按聚合组织)
│   ├── access_point/       # AccessPoint 聚合 (核心编排, 含账户池 + 路由网格)
│   ├── provider/           # Provider 聚合 (配置持有者 + 密钥管理, 含限流/故障配置)
│   ├── proxy/              # Proxy 聚合 (代理转发领域决策: UpstreamOutcome / RetryDecision)
│   ├── user/               # User 聚合 (认证)
│   ├── log/                # Log 聚合 (只读事件数据, 含 operator_type、client_type、AuditAction 和 AuditEntityType 枚举)
│   ├── system/             # System 聚合 (系统设置)
│   └── shared/             # 跨聚合共享 (Status, ApiKey, AccessPointType + 协议方法, ClientType, EncryptionService, InboundRequest, UpstreamRequest, protocols/)
├── application/            # 应用层 (用例编排, 依赖注入, 按聚合组织)
│   ├── access_point/       # AccessPoint 聚合用例 (含账户池 + 路由网格 DTO)
│   ├── auth/               # 跨聚合认证用例
│   ├── dashboard/          # Dashboard 个人用量报告 (KPI / sparkline / heatmap / top models / top access points / quality, 按 user_id 过滤 + 浏览器时区白名单校验)
│   ├── log/                # Log 聚合用例
│   ├── provider/           # Provider 聚合用例
│   ├── proxy/              # 跨聚合代理转发用例 (含账号重试循环)
│   ├── system/             # System 聚合用例
│   ├── user/               # User 聚合用例
│   └── mod.rs              # AppState 全局共享状态
├── infrastructure/         # 基础设施层
│   ├── persistence/        # Repository 实现 (12 个) + 分区管理
│   ├── parsers/            # Claude Code 请求头、SSE、消息摘要和 词元用量解析
│   ├── encryption/         # AES-256-GCM 加密服务
│   ├── auth/               # JWT 认证 + argon2 密码哈希
│   └── http_client/        # reqwest 代理转发客户端 (含账号重试支持)
├── presentation/           # 展示层
│   ├── routes/             # 10 组 axum 路由处理器 + dashboard 聚合查询路由
│   └── middleware/         # JWT 认证中间件 + 用户 API key 认证中间件
├── shared/                 # 共享模块
│   ├── error.rs            # AppError (9 种错误变体)
│   └── types.rs            # PaginatedResult, PaginationParams, Timestamp
├── config.rs               # 环境变量配置加载
├── main.rs                 # 启动入口 (依赖组装 + broadcast channel 创建 + 路由构建 + 分区初始化 + 后台定时分区维护 + 过期令牌清理任务 + 后台 RateLimited 账号恢复任务)
└── lib.rs                  # Crate 根模块 (模块导出)
```

### 领域层 (domain/)

领域层是整个架构的核心，**使用 SeaORM 宏实现实体定义**
（DeriveEntityModel、DeriveActiveEnum、DeriveValueType、FromJsonQueryResult），依赖 sea-orm、serde、uuid、chrono、async-trait。

```
domain/
├── access_point/           # AccessPoint 聚合 (核心编排, 含账户池 + 路由网格)
│   ├── access_point.rs     # SeaORM Model + AccessPointEx 聚合根 + base_url/resolve_model/validate_usable/remove_provider_from_routing/sync_routing_providers 行为方法
│   ├── access_point_account.rs # AccessPointAccount 值对象 (account_id, weight, priority)
│   ├── model_mapping.rs    # 保留向后兼容的旧模型映射类型
│   ├── model_routing_grid.rs # ModelRoutingGrid 值对象 (JSONB: provider_ids + rows[source_model + HashMap&lt;provider_id, target_model&gt;]); 含 resolve_model/remove_provider_column/sync_providers
│   ├── repository.rs       # AccessPointRepository trait (含 find_accounts_by_access_point/save_accounts)
│   ├── routing_strategy.rs # RoutingStrategy 枚举 (Priority/Weighted)
│   ├── short_code.rs       # 接入点短码 (生成/校验)
│   └── mod.rs              # 模块导出
├── provider/               # Provider 聚合 (配置持有者 + 密钥管理, 含限流/故障配置)
│   ├── provider.rs         # SeaORM Model + base_url_for + rate_limit_config/balance_exhausted_config 字段
│   ├── account.rs          # Account SeaORM Model + DisabledReason 枚举 (Manual/RateLimited/BalanceExhausted/Fault) + available_at 字段 + is_available/is_auto_disabled 方法
│   ├── model_list.rs       # ModelList 值对象
│   ├── repository.rs       # ProviderRepository + AccountRepository traits
│   └── mod.rs
├── user/                   # User 聚合 (认证)
│   ├── user.rs             # User SeaORM Model
│   ├── refresh_token.rs    # RefreshToken (is_expired, is_valid)
│   ├── user_api_key.rs     # UserApiKey (SHA-256 哈希存储)
│   ├── repository_refresh_token.rs # RefreshTokenRepository trait
│   ├── repository_user_api_key.rs  # UserApiKeyRepository trait
│   ├── repository_user_repo.rs     # UserRepository trait
│   └── mod.rs
├── log/                    # Log 聚合 (只读事件数据, 含 client_type 列)
│   ├── metadata.rs         # LogMetadata SeaORM Model (含 client_type 列)
│   ├── content.rs          # LogContent SeaORM Model
│   ├── token_usage.rs      # LogTokenUsage SeaORM Model (含 client_type 列)
│   ├── audit_log.rs        # AuditLog SeaORM Model (operator_id/operator_type 替代 user_id)
│   ├── audit_action.rs     # AuditAction 枚举 (20 variants: Create/Update/Delete/Enable/Disable/Recover/AutoRecover/CreateApiKey/RevokeApiKey/UpdateApiKeyDescription/ChangePassword/UpdateProfile/UpdateSettings/Login/LoginFailed/Logout/RefreshRejected/DiscoverModels)
│   ├── audit_entity_type.rs # AuditEntityType 枚举 (8 variants: AccessPoint/Account/Provider/User/UserApiKey/SystemSettings/AuthSession/RefreshToken)
│   ├── dashboard_query.rs  # Dashboard 读模型 (DashboardWindow / KpiAggregate / SparklineBucket / HeatmapCell / TopModelRow / TopAccessPointRow / QualityMetrics, 全部按 user_id 过滤; LEFT JOIN 容忍引用对象删除)
│   ├── repository_audit_log.rs # AuditLogRepository trait
│   ├── repository_log.rs   # LogRepository trait (含个人视角聚合方法: aggregate_kpi / aggregate_sparkline / aggregate_heatmap / top_models / top_access_points / quality_metrics, 首参均为 user_id: Uuid)
│   ├── repository_token_usage.rs # LogTokenUsageRepository trait
│   └── mod.rs
├── system/                 # System 聚合 (系统设置)
│   ├── system_settings.rs  # SystemSettings SeaORM Model
│   └── repository.rs       # SystemSettingsRepository trait
├── proxy/                  # Proxy 聚合 (代理转发领域决策, 无 SeaORM 实体)
│   ├── upstream_outcome.rs # UpstreamOutcome enum (Success/ClientError/Fault/ServerError) + classify 函数
│   ├── retry_decision.rs   # RetryDecision enum (Return(Response) / Continue(AppError))
│   └── mod.rs
├── shared/                 # 跨聚合共享
│   ├── status.rs           # 启用/禁用状态枚举
│   ├── api_key.rs          # API Key (掩码展示)
│   ├── api_type.rs         # AccessPointType 枚举 (Anthropic/OpenAi) + 5 个协议适配方法 (parse_inbound/extract_session_id/inject_api_key/replace_model_in_body/is_sse_response)
│   ├── client_type.rs      # ClientType 枚举 (ClaudeCode/Codex/Other/Unknown), 与 AccessPointType 正交
│   ├── encryption.rs       # EncryptionService trait (encrypt/decrypt)
│   ├── inbound_request.rs  # InboundRequest struct (入站请求纯数据, 含 client_type 字段)
│   ├── upstream_request.rs # UpstreamRequest struct (上游请求纯数据, 无方法)
│   ├── protocols/          # 协议适配实现 (每协议一文件, 由 AccessPointType 方法 match 分发)
│   │   ├── anthropic.rs    # Anthropic 协议实现 (pub(in crate::domain::shared) fn)
│   │   ├── openai.rs       # OpenAI 协议实现 (Chat Completions + Responses API 双端点)
│   │   └── mod.rs
│   └── mod.rs
└── mod.rs                  # 领域层入口 (声明所有聚合模块 + shared)
```

**领域实体即 ORM 实体**：domain 层直接使用 SeaORM `DeriveEntityModel` 宏定义实体，消除了基础设施层的重复实体和 200+ 行
TryFrom/From 手工映射代码。领域实体附加的行为方法（new、resolve_model、is_expired 等）直接定义在 Model 上。

**聚合根模式**：AccessPoint 的 `AccessPointEx`（自定义 struct，含 access_point + accounts）是代理管道的聚合根。Repository 的
`find_by_short_code` 返回 `AccessPointEx`。ProxyPipeline 仅与该聚合根交互，不再直接引用 Provider/Account
类型。与旧架构不同，AccessPoint 不再包含 provider_id 和 account_id 外键列，接入点通过 `access_point_accounts` 关联表与多个
Account 建立多对多关联，模型路由通过 `model_routing_grid` JSONB 列按二维网格（source_model x provider_id）进行目标模型匹配。

**聚合边界**：

| 聚合根                      | 包含子实体 / 值对象                                                                 | 跨聚合引用                                                                          |
| --------------------------- | ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| AccessPoint (AccessPointEx) | ShortCode, AccessPointAccount(s) (账户池), ModelRoutingGrid, RoutingStrategy        | access_point_accounts → Account (多对多), model_routing_grid → Provider (UUID 引用) |
| Provider                    | Account (含 DisabledReason), ModelList, rate_limit_config, balance_exhausted_config | -                                                                                   |
| Proxy (无聚合根)            | UpstreamOutcome, RetryDecision (纯领域决策枚举, 由 ProxyPipeline 调用)              | 通过 classify 调用 FaultService 进行故障归类                                        |
| User                        | RefreshToken, UserApiKey                                                            | -                                                                                   |
| LogMetadata                 | LogContent, LogTokenUsage                                                           | user_id, access_point_id, provider_id, account_id (Uuid)                            |
| SessionAffinity             | -                                                                                   | access_point_id, account_id (UUID)                                                  |

### 应用层 (application/)

应用层负责用例编排，通过构造函数注入 domain 层的 Repository trait，**不直接依赖 SeaORM**。
应用层按聚合组织目录结构，与领域层聚合命名对齐。跨聚合的编排服务（auth、proxy）独立归置。

```
application/
├── access_point/           # AccessPoint 聚合用例
│   ├── mod.rs              # 模块导出
│   ├── access_point_service.rs # 接入点管理用例 (含 routing_strategy + model_routing_grid + accounts + 审计日志写入)
│   └── dto/                # 接入点增改查 DTO (含 account_dto, model_routing_grid_dto)
│       ├── mod.rs
│       ├── access_point_response.rs
│       ├── account_dto.rs          # AccountDto (account_id, weight, priority)
│       ├── create_access_point_request.rs
│       ├── model_mapping_dto.rs
│       ├── model_routing_grid_dto.rs # ModelRoutingGridDto + ModelRoutingRowDto
│       └── update_access_point_request.rs
├── auth/                   # 跨聚合认证用例
│   ├── mod.rs
│   ├── claims.rs
│   ├── auth_service.rs     # 认证用例 (登录/刷新/登出, Refresh Token Rotation, 审计日志写入)
│   └── dto/                # Login/Refresh/TokenPair DTO
├── dashboard/              # Dashboard 个人用量报告用例 (仅依赖 LogRepository, 全部 SQL 含 user_id 过滤, 无跨聚合 Repository 依赖)
│   ├── mod.rs
│   ├── dashboard_service.rs # 编排服务 (get_kpi + get_heatmap + get_top_models + get_top_access_points + get_quality_metrics; compute_trend 处理 5 种 TrendBadge 边界)
│   ├── time_window.rs      # 时间范围解析纯逻辑 (today/last7/last30/custom)
│   ├── timezone.rs         # 时区白名单校验 (chrono_tz::Tz::from_str), 拦截 PostgreSQL AT TIME ZONE 字面量拼接的 SQL 注入风险
│   └── dto/                # 11 个 DTO: TimeRangeQuery / TimezoneQuery / KpiResponse (含 5 项词元 input/output/cache_creation/cache_read/thinking) / KpiTrendItem / TrendBadge / SparklineSeries / HeatmapResponse / HeatmapCellDto / TopModelItem / TopAccessPointItem / QualityResponse
├── log/                    # Log 聚合用例
│   ├── mod.rs
│   ├── log_service.rs      # 日志写入/查询用例 (metadata、content、events、词元用量); 集成 broadcast::Sender 广播新日志事件
│   └── dto/                # 日志 DTO (含 proxy_log_input —— LogService::record_proxy_log 的一次性入参契约; NewLogEvent —— SSE 广播事件)
├── provider/               # Provider 聚合用例
│   ├── mod.rs
│   ├── provider_service.rs # 提供商管理用例 (含 rate_limit/balance_exhausted 配置 + 审计日志写入)
│   ├── account_service.rs  # 账号管理用例 (含加密解密、禁用/恢复 + 审计日志写入)
│   └── dto/                # Provider/Account 增改查 DTO
├── proxy/                  # 跨聚合代理转发用例 (调度骨架 + 编排子组件)
│   ├── mod.rs
│   ├── proxy_pipeline.rs       # 核心代理转发管道 (60 行 execute 调度骨架 + try_one_account 子方法; 0 步关闭短路 + 协议解析含 ClientType 识别 + 排序粘滞 + 候选迭代)
│   ├── proxy_call_record.rs    # 代理调用记录器 (start → attach_response → append/set_body → finish; Drop 兜底 SSE 中断标记 is_interrupted)
│   ├── tracked_spawner.rs      # 后台写入调度器 (fetch_add + try_current 守卫 + spawn + fetch_sub; 归一代理日志和会话粘滞两处 spawn 模板)
│   ├── account_selector.rs     # 候选账号异步迭代器 (AccountSelector + AccountCandidate; 封装加载 Account → 跳过不可用 → 加载 Provider → 解密 API Key 四步)
│   ├── upstream_dispatcher.rs  # 上游转发执行器 (UpstreamDispatcher; ProxyClient::forward + 120s 非流式响应体读取超时 + copy_passthrough_headers)
│   └── response_builder.rs     # axum 响应构造 (build_streaming_response / build_buffered_response + hop-by-hop 头过滤)
├── system/                 # System 聚合用例
│   ├── mod.rs
│   ├── settings_service.rs # 系统设置管理 (含审计日志写入)
│   └── dto/
├── user/                   # User 聚合用例
│   ├── mod.rs
│   ├── user_service.rs     # 用户管理用例 (含密码哈希 + profile 更新 + 密码修改 + 审计日志写入)
│   ├── api_key_service.rs  # 用户 API key 管理 (生成/列表/撤销, SHA-256 哈希 + 审计日志写入)
│   └── dto/                # User 增改查 DTO
└── mod.rs                  # AppState 定义 (所有 Service 的引用容器)
```

**目录组织原则**：

- 每个聚合目录内聚 service 和 dto，通过 `super::dto::` 相对路径引用同目录 DTO
- 外部引用使用绝对路径 `crate::application::<聚合>::dto::*`
- auth/、proxy/、dashboard/ 和 system/ 是跨聚合编排服务，不归属于单一聚合
- dashboard/ 虽属跨聚合视图，但仅依赖 `LogRepository`——事实表为 `log_metadata` / `log_token_usage`
  ，users/accounts/providers 仅作展示数据 LEFT JOIN 进同一聚合 SQL，不构成跨聚合读模型
- `ProxyCallRecord` 置于 `application/proxy/` 而非基础设施层——它直接接受领域聚合（`InboundRequest` / `UpstreamRequest` /
  `AccessPointEx`），层级归属应与其依赖一致

**代理日志架构**：`ProxyCallRecord` 是应用层的日志记录器，贯穿一次代理转发的完整生命周期，API 形态对应业务时序：

```
start(请求侧已知, 启动计时)
  └─→ attach_response(上游响应头到达后)
        └─→ append_body / set_body (SSE 逐段或非流式一次性)
              └─→ finish() (正常完成 → spawn 异步落库)
                    └─→ Drop 兜底 (未 finish 则标记 is_interrupted=true 并落库)
```

构造时 `start` 从 `InboundRequest`、`UpstreamRequest`、`AccessPointEx` 提取请求侧字段（model_original / model_mapped /
api_type / session_id / user_id / provider_id / account_id 等），上游响应头到达后通过 `attach_response` 登记状态码和响应头，SSE
流通过 `append_body` 逐段累积、非流式通过 `set_body` 一次性写入。`finish` 计算 `duration_ms` 并构造 `ProxyLogInput`（
`application/log/dto/proxy_log_input.rs`，仅作 LogService 一次性入参契约，不再是双重职责 DTO）后通过 `TrackedSpawner` 异步交给
`LogService::record_proxy_log` 落库。

中断检测由 `ProxyCallRecord::Drop` 隐式处理——`async_stream` 闭包被 drop 时（如客户端断开 SSE 连接）触发
`is_interrupted = true` 标记并 spawn 落库，是 SSE 中断场景下唯一可靠的落库机制。

**日志实时推送**：`LogService::record_proxy_log` 在每次日志写入成功后，通过 `tokio::sync::broadcast::Sender<NewLogEvent>`
广播 `NewLogEvent`（含 `log_id` 和 `short_code`）。前端 `useLogEvents` hook 通过 `GET /api/logs/events` SSE 端点（JWT 认证，
`text/event-stream`）接收事件，触发页面全量刷新。broadcast channel 容量 256，满时丢弃最旧事件以避免背压，前端通过全量刷新作为兜底。SSE
端点响应优雅关闭信号（`shutdown_rx`），与主进程优雅关闭联动。JWT 令牌通过 URL query 参数传递（EventSource API 不支持自定义
header）。选择 SSE 而非 WebSocket：仅需单向推送（后端到前端），axum 原生支持，EventSource 浏览器 API 自动重连。

**后台写入调度**：所有 fire-and-forget 写入（代理日志 + 会话粘滞）统一通过 `TrackedSpawner::spawn(operation, future)`
入队，封装了 `fetch_add → Handle::try_current 守卫 → tokio::spawn → fetch_sub` 模板。`Handle::try_current` 守卫避免运行时关闭后
spawn 触发 panic；`in_flight_writes` 计数器供主进程优雅关闭时轮询归零。

**AppState** 是全局共享状态，通过 axum 的 `with_state()` 注入到所有路由处理器，包含 Config、数据库连接、所有 Service 引用、JWT
服务、代理客户端、AuditLogRepository，以及 SSE 相关基础设施（`log_event_tx: broadcast::Sender<NewLogEvent>`、
`shutdown_rx: watch::Receiver<bool>`）。

### 基础设施层 (infrastructure/)

基础设施层实现 domain 层定义的接口，处理所有外部依赖。

```
infrastructure/
├── persistence/            # SeaORM 数据持久化
│   ├── partition_manager.rs # PartitionManager: 应用层分区自动管理
│   └── repositories/       # Repository 实现 (8 个, 含 refresh token 过期清理 delete_expired)
│       ├── provider_repository.rs        # SeaOrmProviderRepository
│       ├── account_repository.rs         # SeaOrmAccountRepository
│       ├── user_repository.rs            # SeaOrmUserRepository
│       ├── access_point_repository.rs    # SeaOrmAccessPointRepository (含 账户池 CRUD 事务)
│       ├── access_point_account_repository.rs         # access_point_accounts 表 SeaORM 实体定义
│       ├── session_affinity_repository.rs # session_affinity 表 SeaORM 实体定义
│       ├── refresh_token_repository.rs   # SeaOrmRefreshTokenRepository
│       ├── log_repository.rs             # SeaOrmLogRepository
│       ├── log_token_usage_repository.rs # SeaOrmLogTokenUsageRepository
│       ├── audit_log_repository.rs       # SeaOrmAuditLogRepository
│       ├── user_api_key_repository.rs    # SeaOrmUserApiKeyRepository
│       └── system_settings_repository.rs # SeaOrmSystemSettingsRepository
├── encryption/             # 加密实现
│   └── aes256_gcm_encryption_service.rs # Aes256GcmEncryptionService
├── auth/                   # 认证实现
│   ├── jwt_service.rs      # JwtService (jsonwebtoken 10 + aws-lc-rs 加密后端, 含 refresh_expiry_secs 访问器)
│   ├── password.rs         # argon2 密码哈希
│   └── claims.rs           # JWT Claims 定义
├── parsers/                # 响应体解析器
│   ├── claude_code_context.rs # Claude Code 请求头解析 (session_id/agent_id)
│   ├── client_info.rs      # User-Agent 客户端信息解析
│   ├── parsed_token_usage.rs # 词元用量提取 (支持 Anthropic SSE + OpenAI Chat/Responses 词元格式)
│   └── mod.rs
└── http_client/            # HTTP 客户端
    ├── proxy_client.rs     # ProxyClient (reqwest 连接池 + rustls TLS, 纯 HTTP 执行器)
    └── mod.rs              # 模块导出 (仅 ProxyClient; 上游请求构造已上移到 AccessPointEx::build_upstream_request, 日志记录器已上移到 application/proxy/proxy_call_record.rs)
```

所有 Repository 实现以 `SeaOrm` 为前缀（如 `SeaOrmProviderRepository`），并在 `main.rs` 中通过 `Arc<dyn Trait>` 向上转型后注入应用层。

### 展示层 (presentation/)

展示层处理 HTTP 请求/响应，使用 axum 框架。

```
presentation/
├── routes/                 # 路由处理器
│   ├── mod.rs              # 路由聚合 + 健康检查
│   ├── auth_routes.rs      # POST /api/auth/login, /api/auth/refresh
│   ├── provider_routes.rs  # CRUD /api/providers
│   ├── account_routes.rs   # CRUD /api/providers/:id/accounts (嵌套)
│   ├── user_routes.rs      # CRUD /api/users
│   ├── me_routes.rs        # GET/PUT /api/users/me/* (个人 profile/密码/API key)
│   ├── access_point_routes.rs # CRUD /api/access-points (含 accounts/model_routing_grid)
│   ├── proxy_routes.rs     # POST /ap/{short_code}/v1/messages (强制 API key 认证)
│   ├── log_routes.rs       # GET /api/logs, /api/logs/events (SSE, JWT 保护), /api/logs/sessions, /api/logs/sessions/:id
│   ├── dashboard_routes.rs # GET /api/getting-started/{kpi,heatmap,top-models,top-access-points,quality} (JWT 保护, handler 通过 CurrentUser extractor 提取 user_id 注入 Service)
│   ├── settings_routes.rs  # GET/PUT /api/settings
│   └── frontend.rs         # 前端静态资源服务
└── middleware/             # 中间件
    ├── jwt_auth.rs         # JWT 认证中间件 + CurrentUser extractor
    └── user_api_key_auth.rs # 用户 API key 认证 (SHA-256, Authorization 头)
```

**路由认证策略**：

| 路径                     | 认证要求                                                               |
| ------------------------ | ---------------------------------------------------------------------- |
| `/api/auth/*`            | 公开 (登录/刷新)                                                       |
| `/ap/*`                  | Bearer 用户 API key 认证 (SHA-256, Authorization 头)                   |
| `/api/health`            | 公开                                                                   |
| `/api/providers/*`       | JWT 认证                                                               |
| `/api/accounts/*`        | JWT 认证                                                               |
| `/api/users/*`           | JWT 认证                                                               |
| `/api/users/me/*`        | JWT 认证 (当前用户个人设置)                                            |
| `/api/access-points/*`   | JWT 认证                                                               |
| `/api/logs/*`            | JWT 认证 (其中 `/api/logs/events` SSE 端点通过 URL query 参数传递 JWT) |
| `/api/getting-started/*` | JWT 认证                                                               |

### 共享模块 (shared/)

```
shared/
├── error.rs    # AppError 枚举 (9 种变体 + IntoResponse 实现)
└── types.rs    # PaginatedResult<T>, PaginationParams, Timestamp
```

**AppError 错误类型**：

| 变体         | HTTP 状态码 | 说明                       |
| ------------ | ----------- | -------------------------- |
| Validation   | 400         | 请求参数校验失败           |
| NotFound     | 404         | 资源未找到                 |
| Conflict     | 409         | 资源冲突（如重名）         |
| Unauthorized | 401         | 未认证或令牌无效           |
| Forbidden    | 403         | 无操作权限                 |
| Encryption   | 500         | 加密/解密错误 (不暴露详情) |
| Database     | 500         | 数据库错误 (不暴露详情)    |
| Upstream     | 502         | 上游 LLM 服务错误          |
| Internal     | 500         | 内部服务器错误             |

### 配置加载 (config.rs)

从环境变量加载运行时配置，所有必填变量在启动时验证：

| 变量                          | 类型   | 说明                    | 默认值   |
| ----------------------------- | ------ | ----------------------- | -------- |
| DATABASE_URL                  | String | PostgreSQL 连接串       | **必填** |
| JWT_SECRET                    | String | JWT 签名密钥            | **必填** |
| ENCRYPTION_KEY                | String | 64 位十六进制 (32 字节) | **必填** |
| SERVER_PORT                   | u16    | 监听端口                | 3000     |
| LOG_LEVEL                     | String | 日志级别                | info     |
| PARTITION_CHECK_INTERVAL_SECS | u64    | 分区检查间隔（秒）      | 3600     |
| PARTITION_PREMAKE_MONTHS      | u32    | 提前创建未来分区数      | 3        |
| PARTITION_RETENTION_MONTHS    | u32    | 分区保留月数            | 12       |

## 前端架构详解 (src-dashboard/)

前端是单页应用 (SPA)，构建产物嵌入 Rust 二进制，生产环境与后端同源部署。

```
├── src-dashboard/                  # 前端管理面板源代码
│   ├── main.tsx                    # React 入口
│   ├── App.tsx                     # 路由定义 (react-router-dom v7)
│   ├── App.css                     # 应用全局样式
│   ├── index.css                   # 基础样式重置
│   ├── styles.css                  # 额外样式
│   ├── api.ts                      # API 通信层 (fetch 封装)
│   ├── assets/                     # 静态资源
│   ├── components/                 # 组件 (按功能分组子目录, 通过 @components 路径别名引用)
│   │   ├── common/                 # 通用 UI 组件 (8 个, 跨领域复用)
│   │   │   ├── CollapsibleCard.tsx      # 可折叠卡片 (header 区域可点击折叠/展开)
│   │   │   ├── CodeHighlight.tsx        # 代码高亮组件
│   │   │   ├── ConnectionIndicator.tsx  # SSE 连接状态指示器 (绿/黄/红三色圆点)
│   │   │   ├── CopyableIdText.tsx       # 可复制 ID 文本 (等宽字体 + 点击复制)
│   │   │   ├── ExpandableContentBlock.tsx # 可展开/收起的长内容块
│   │   │   ├── MarkdownRender.tsx       # Markdown 渲染组件
│   │   │   ├── StatusToggle.tsx         # 状态切换开关 (跨领域复用)
│   │   │   └── ThemeToggle.tsx          # 主题切换 (light/dark/system)
│   │   ├── access-point/           # 接入点管理组件 (3 个) + 工具函数
│   │   │   ├── AccessPointDrawer.tsx    # 接入点创建/编辑表单 (含 api_type、Provider 选择并显示默认模型, 自动预填 __unmatched__ -> __default_model__ 映射)
│   │   │   ├── AccessPointTable.tsx     # 接入点列表表格
│   │   │   ├── ModelMappingEditor.tsx   # 模型映射编辑器
│   │   │   └── modelMappingUtils.ts     # 模型映射工具函数、类型和常量 (从 ModelMappingEditor.tsx 分离)
│   │   ├── provider/               # Provider 管理组件 (1 个)
│   │   │   └── AccountManager.tsx       # Account 表格 + 添加/编辑 SideSheet (从 ProviderManagement 提取)
│   │   ├── log/                    # 日志相关组件 (5 个)
│   │   │   ├── LogFilterBar.tsx         # 日志过滤栏
│   │   │   ├── RequestLogTable.tsx      # 请求日志表格 (列定义 + Table 渲染)
│   │   │   ├── RawResponseView.tsx      # 原始响应查看组件
│   │   │   ├── TokenCell.tsx            # 词元列渲染 (6 类词元字段)
│   │   │   └── log-detail/              # 日志详情卡片组 (强内聚子组, 含 request-content/ 和 response-content/ 子目录)
│   │   │       ├── BasicInfoCard.tsx        # 基础信息卡片
│   │   │       ├── TokenUsageCard.tsx       # 词元用量卡片
│   │   │       ├── tokenUsage.ts            # 词元用量计算工具函数 (从 TokenUsageCard.tsx 分离)
│   │   │       ├── HeadersCard.tsx          # 请求头卡片
│   │   │       ├── RequestContentCard.tsx   # 请求内容卡片 (委托 request-content/ 子组件)
│   │   │       ├── ResponseContentCard.tsx  # 响应内容卡片 (委托 response-content/ 子组件)
│   │   │       ├── request-content/        # 请求内容子组件 (11 个文件)
│   │   │       │   ├── AccordionSection.tsx
│   │   │       │   ├── ContextManagementSection.tsx
│   │   │       │   ├── MessageBlock.tsx
│   │   │       │   ├── MessagesSection.tsx
│   │   │       │   ├── MetadataSection.tsx
│   │   │       │   ├── RequestConfigSection.tsx
│   │   │       │   ├── SectionHeading.tsx
│   │   │       │   ├── SystemPromptSection.tsx
│   │   │       │   ├── ToolDetail.tsx
│   │   │       │   ├── ToolsSection.tsx
│   │   │       │   └── utils.ts
│   │   │       └── response-content/       # 响应内容子组件 (3 个)
│   │   │           ├── TextBlockCard.tsx
│   │   │           ├── ThinkingBlockCard.tsx
│   │   │           └── ToolUseBlockCard.tsx
│   │   ├── session/                # 会话查看组件 (8 个)
│   │   │   ├── SessionListView.tsx      # 会话列表视图 (过滤栏 + 分页表格; 集成 SSE 自动刷新)
│   │   │   ├── SessionDetailView.tsx    # 会话详情视图 (轮次导航 + 轮次卡片列表; 集成 SSE 增量刷新 + beforeRefresh prop)
│   │   │   ├── TurnCard.tsx             # 轮次卡片组件 (请求/响应/工具调用等消息块)
│   │   │   ├── TurnNavigator.tsx        # 轮次导航条组件 (编号/状态/摘要)
│   │   │   └── RawContentModal.tsx      # 原始内容查看弹窗
│   │   ├── dashboard/              # 仪表盘组件 (个人用量视角; 基于 Recharts)
│   │   │   ├── Sparkline.tsx             # Recharts 极简单色折线
│   │   │   ├── ComparisonArrow.tsx       # 5 种 TrendBadge 渲染 (empty/new/down=-100/up/flat/down)
│   │   │   ├── StackedBar.tsx            # 纯 CSS Flex 横向堆叠条
│   │   │   ├── KpiCard.tsx               # 含 sparkline 的 KPI 卡 (个人总请求数 / 词元 5 项 / 错误率等)
│   │   │   ├── CacheHitCard.tsx          # 纯比率 KPI 卡
│   │   │   ├── TimeRangeSelector.tsx     # RadioGroup + Popover DatePicker
│   │   │   ├── Heatmap.tsx               # GitHub 风格 53×7 方格矩阵 (固定近 1 年窗口, 分位数色阶, 按浏览器时区分桶)
│   │   │   ├── TopModelsRanking.tsx      # Top Models 排行卡片 (按模型聚合个人请求数 + 词元)
│   │   │   ├── TopAccessPointsRanking.tsx # Top Access Points 排行卡片 (按个人使用的接入点聚合)
│   │   │   └── QualityCard.tsx           # 错误率 / 平均延迟 / 重试比例等质量指标卡
│   │   └── user/                   # 用户管理组件 (1 个)
│   │       └── ApiKeyManager.tsx        # API Key 表格 + 创建/编辑/吊销 Modal (从 ProfilePage 提取)
│   ├── hooks/                      # 自定义 hooks
│   │   ├── useFetch.ts             # 通用数据获取 hook (useFetch<T>(fetcher, deps) → { data, loading, error, refetch }; loading 初始 true, setState 仅在异步回调中执行; refetch 时 setLoading(true))
│   │   ├── useTheme.ts             # 主题管理 (ThemeProvider + useTheme, 三种模式)
│   │   ├── useAccessPoints.ts      # 接入点数据管理 (Provider/Account 加载; 创建/编辑时过滤 target_model 不在 Provider.models + Provider.default_model + DEFAULT_MODEL 哨兵的映射; 删除/切换状态/复制 URL)
│   │   └── useLogEvents.ts         # SSE 事件监听 hook (EventSource 连接管理, 接入 JWT 令牌, 连接状态: connected/disconnected/error; 收到事件后触发全量刷新)
│   ├── layouts/
│   │   └── AdminLayout.tsx         # 管理界面布局 (Semi Design Navigation)
│   ├── pages/
│   │   ├── LoginPage.tsx           # POST /api/auth/login
│   │   ├── GettingStartedPage.tsx       # 个人用量报告 (CSS Grid 布局, 5 个 useFetch 并行加载 KPI/Heatmap/Top Models/Top Access Points/Quality; 浏览器时区通过 Intl.DateTimeFormat 获取并作为 query 参数传后端; 热力图 deps 与 timeRange 解耦, 仅依赖 refreshKey; 不再订阅 SSE; 配套 GettingStartedPage.css 响应式断点 1280/768)
│   │   ├── ProviderManagement.tsx  # CRUD /api/providers (表格 default_model 列使用 Tag 渲染; 编辑面板模型列表 TagInput + 下方独立 default_model Select; models 为空时禁用选择; TagInput 移除模型联动清空 default_model; 保存时若 default_model 不在 models 中则自动清空)
│   │   ├── AccessPointManagement.tsx # CRUD /api/access-points (Provider 切换时, 创建态下若有 default_model 则自动生成 __unmatched__(prefix) → __default_model__ 哨兵映射; 保存委托 useAccessPoints hook 过滤无效映射)
│   │   ├── UserManagement.tsx      # CRUD /api/users
│   │   ├── ProfilePage.tsx         # 个人设置 (profile/密码/API key 管理)
│   │   ├── SessionLogPage.tsx      # 会话日志路由壳 (根据 URL 中 sessionId 参数切换列表/详情视图: 无 sessionId 渲染 SessionListView, 有 sessionId 渲染 SessionDetailView; 集成 useLogEvents 自动刷新)
│   │   ├── RequestLogPage.tsx      # GET /api/logs (数据加载 + 过滤 + 委托 RequestLogTable 渲染表格; 集成 useLogEvents 自动刷新)
│   │   ├── LogDetailPage.tsx       # GET /api/logs/:id (单条日志详情, 含请求/响应内容展示)
│   │   └── SettingsPage.tsx        # 设置页面
│   ├── types/                      # TypeScript 类型定义 (accessPoint.ts, dashboard.ts, log.ts)
│   │   ├── accessPoint.ts          # 接入点相关类型定义
│   │   ├── dashboard.ts            # 仪表盘个人用量类型 (镜像后端 DTO: TimeRangeQuery / TimezoneQuery / KpiResponse / KpiTrendItem / TrendBadge / SparklineSeries / HeatmapResponse / HeatmapCell / TopModelItem / TopAccessPointItem / QualityResponse)
│   │   └── log.ts                  # 日志相关类型: TokenUsage / SessionContentItem / ConversationTurn(轮次-块-摘要三级结构: 含 TurnBlock 消息块联合类型、TurnTokenSummary 用量汇总、ConversationTurn 轮次容器)
│   └── utils/                      # 工具函数 (parseLogs.ts, parseOpenAI.ts, format.ts, query.ts)
├── index.html                      # HTML 入口 (Vite)
├── vite.config.ts                  # Vite 构建配置
├── tsconfig.json                   # TypeScript 根配置
├── tsconfig.app.json               # TypeScript 应用配置
├── tsconfig.node.json              # TypeScript Node 配置
├── eslint.config.js                # ESLint 配置
└── package.json                    # 依赖声明
```

### 前端路由结构

```
/login                  → LoginPage
/                       → AdminLayout (重定向至 /dashboard)
  /dashboard            → GettingStartedPage
  /providers/*          → ProviderManagement
  /access-points        → AccessPointManagement
  /sessions             → SessionLogPage
  /sessions/:sessionId  → SessionLogPage (单会话详情)
  /logs                 → RequestLogPage
  /logs/:id             → LogDetailPage (单条日志详情)
  /users                → UserManagement
  /settings             → SettingsPage
  /profile              → ProfilePage (个人设置)
```

### API 通信层

`api.ts` 封装了基于 fetch 的 HTTP 客户端，自动附加 JWT `Authorization` 头。采用「双层防御」策略处理令牌过期：请求前检查
Access Token 是否接近过期，必要时通过 Refresh Token 静默刷新；若刷新失败或 401 响应仍到达，则清除所有本地令牌并跳转登录页。模块级
`refreshing` Promise 实现并发刷新去重，避免 Refresh Token Rotation 模式下多请求互相吊销。提供 `get`、`post`、`put`、`delete`
四个方法。

### 主题系统

前端支持 light / dark / system 三种主题模式，通过 `useTheme.ts` hook 管理。系统主题自动跟随 `prefers-color-scheme` 媒体查询。
`ThemeProvider` 在根组件包装，通过 `document.body` 的 `theme-mode` 属性控制 Semi Design 暗色模式切换。`ThemeToggle`
组件位于管理面板侧边栏和登录页。

### 组件工具函数分离模式

组件文件中只保留组件导出和 JSX 渲染逻辑，纯函数、类型定义和常量分离到同级的 `xxxUtils.ts` 或 `xxx.ts` 文件中。此模式保持组件文件聚焦于
UI 渲染，同时提升纯函数的可测试性。

当前已应用此模式的组件：

- `ModelMappingEditor.tsx` → `modelMappingUtils.ts`（工具函数、类型和常量）
- `TokenUsageCard.tsx` → `tokenUsage.ts`（词元用量计算纯函数）

### 派生状态模式

通过 `useMemo` 从 props 或现有状态派生数据，替代 `useState + useEffect` 模式。消除 `useEffect` 中的 `setState` 调用和 ref
渲染期写入，确保状态在渲染阶段同步更新，减少不必要的重渲染。

- `AdminLayout`: `selectedKeys` 从 `useState` 改为 `useMemo`（基于 `location.pathname`）
- `AccessPointDrawer`: `rowSelectedProviders` 从 `useState` 改为 `useMemo`（基于 `formData.accounts`）

### 通用数据获取 Hook

`useFetch<T>(fetcher, deps)` 封装 fetch-on-mount 模式，统一管理数据获取生命周期（加载/成功/错误）和状态清理。`loading` 初始为
`true`，`setState` 仅在异步回调中执行，避免竞态条件和内存泄漏。API 接口：`useFetch<T>(fetcher, deps)` →
`{ data, loading, error, refetch }`。已替代 11 个文件中的手动 `useState + useCallback + useEffect` 模式。

## 数据库架构详解 (src/migrations/)

迁移使用 `sea-orm-migration`，迁移文件位于 `src/migrations/` 目录下。

```
src/migrations/
├── mod.rs
├── m20260519_000001_initial.rs              # 初始 Schema (含所有基础表)
├── m20260618_000002_account_pool.rs          # 接入点账户池重构: 创建 access_point_accounts + session_affinity 表, access_points 列变更, accounts 列新增, providers 列变更, audit_logs 列变更
└── m20260623_000003_client_type.rs           # ClientType 支持: log_metadata + log_token_usage 添加 client_type VARCHAR(32) 列
```

### 数据库表

| 表                    | 说明                            | 关键字段                                                                                                       |
| --------------------- | ------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| providers             | LLM 提供商                      | name, openai_base_url, anthropic_base_url, models, rate_limit_config (JSONB), balance_exhausted_config (JSONB) |
| accounts              | API 账号                        | encrypted_key, key_tail (末 6 位), provider_id (FK), disabled_reason, available_at                             |
| users                 | 管理员用户                      | username, password_hash                                                                                        |
| access_points         | 接入点                          | short_code (唯一), api_type, routing_strategy, model_routing_grid (JSONB)                                      |
| access_point_accounts | 接入点账号 (多对多)             | access_point_id (FK), account_id (FK), weight, priority                                                        |
| session_affinity      | 会话粘滞                        | access_point_id (FK), session_id, account_id (FK)                                                              |
| refresh_tokens        | JWT 刷新令牌                    | user_id (FK), token_hash, expires_at, revoked; 过期记录由 tokio 后台任务每小时物理清理                         |
| log_metadata          | 代理日志元数据 (按月分区)       | session_id, model_original, model_mapped, status_code, duration_ms, client_type                                |
| log_contents          | 代理日志内容 (按月分区)         | log_id, timestamp, request_headers, request_body, response_body                                                |
| log_token_usage       | 词元用量详情 (永久保留)         | log_id, timestamp, input_tokens, output_tokens, cache_creation, cache_read, usage_type, client_type            |
| audit_logs            | 操作审计日志                    | operator_id, operator_type, action, entity_type, entity_id, details                                            |
| user_api_keys         | 用户 API key (SHA-256 哈希存储) | user_id (FK), key_hash (唯一), key_prefix, description, last_used_at, status, created_at                       |
| system_settings       | 系统设置                        | key, value                                                                                                     |

**分区策略**: `log_metadata` 和 `log_contents` 表按月 `RANGE (timestamp)` 分区，由应用层 `PartitionManager`
自动管理（创建 / 清理），通过 `pg_try_advisory_xact_lock` 保证多副本安全。`log_token_usage` 不做分区，永久保留用于分析。

## 代理转发流程

```
POST /ap/{short_code}/v1/messages  (Authorization: Bearer <user_api_key>)
    │
    ├── 用户 API key 认证 (presentation/middleware/user_api_key_auth): SHA-256 hex → 查找 UserApiKey → 验证 enabled, 更新 last_used_at
    │
    └── ProxyPipeline::execute (application/proxy/proxy_pipeline.rs, 60 行调度骨架):
         │
         ├── 0. 优雅关闭短路: shutting_down.load() 为 true → 立即返回 AppError::Upstream("服务正在关闭")
         │
         ├── 1. 加载并准备聚合根: find_by_short_code → AccessPointEx → validate_usable / has_available_accounts
         │
         ├── 2. 协议解析: access_point.api_type.parse_inbound(headers, body) → InboundRequest
         │     access_point.api_type.extract_session_id(&inbound) → Option<String>
         │     ClientType::from_user_agent(user_agent, &path) → client_type (客户端类型识别)
         │     (协议方法 match 分发到 domain/shared/protocols/<name>.rs)
         │
         ├── 3. 排序 + 粘滞: sort_accounts() → 若有 session_id 则 apply_session_affinity
         │
         └── 4. 重试循环 (AccountSelector 异步迭代):
         │
         ├── selector.next() → AccountCandidate { account, account_id, provider, upstream_key }
         │   (内部自动完成: 加载 Account → 跳过 is_available=false → 加载 Provider → 解密 API Key)
         │
         └── try_one_account(候选) → RetryDecision:
              │
              ├── a. AccessPointEx::build_upstream_request(&inbound, &provider, &upstream_key, remainder)
              │       → UpstreamRequest { url, headers, body, mapped_model }
              │       (聚合根编排: URL 拼接 + 模型路由网格查表 + 协议方法 inject_api_key/replace_model_in_body)
              │
              ├── b. ProxyCallRecord::start(请求侧已知, 启动计时)
              │
              ├── c. UpstreamDispatcher::forward(&upstream) → reqwest::Response
              │       record.attach_response(status, &resp_headers)
              │       is_sse = access_point.api_type.is_sse_response(&resp_headers)
              │
              └── d. UpstreamOutcome::classify(provider, status, &resp_headers, resp_body, is_sse):
                      ├── Success / ClientError / ServerError → 透传给客户端 → RetryDecision::Return
                      │     - SSE: build_streaming_response 内 record.append_body 逐段累积 + finish
                      │     - 非 SSE: build_buffered_response 内 record.set_body + finish
                      │     - 成功时通过 TrackedSpawner 异步保存会话粘滞
                      │
                      └── Fault → 禁用当前账号 (FaultService::disable_account + Account::save):
                            - SSE 路径: 流已开始无法切换 → 透传 + RetryDecision::Return(stream)
                            - 非 SSE 路径: record.finish() → RetryDecision::Continue(AppError)

         调度循环 (execute 中):
              ├── RetryDecision::Return(resp) → return Ok(resp)
              └── RetryDecision::Continue(reason) → last_error = Some(reason); 取下一候选

         候选耗尽 → 返回 last_error 或 AppError::Upstream("所有账号不可用")
```

> 类型系统强制完整性：`RetryDecision::Continue(AppError)` 强制重试时必须携带错误原因，无法 `continue` 而不带错误；过去
> `last_error = Some(...); continue;` 配对易遗漏的隐患被永久消除。

### 协议适配层（domain/shared/protocols/）

LLM API 协议的差异点（请求头格式、session 标识 header 名、API key 注入方式、body 字段约定）由 `AccessPointType` 枚举挂载的 5
个协议方法（`parse_inbound` / `extract_session_id` / `inject_api_key` / `replace_model_in_body` / `is_sse_response`
）封装。每个协议方法内部 `match self` 后调用对应协议模块（`protocols/<name>.rs`）中的 `pub(in crate::domain::shared) fn`。

**已支持的协议**：

| 协议      | AccessPointType 变体 | 协议文件               | 端点                                                                        |
| --------- | -------------------- | ---------------------- | --------------------------------------------------------------------------- |
| Anthropic | Anthropic            | protocols/anthropic.rs | `/v1/messages`                                                              |
| OpenAI    | OpenAi               | protocols/openai.rs    | Chat Completions (`/v1/chat/completions`) + Responses API (`/v1/responses`) |

**为什么挂在 AccessPointType 而非新建 ApiProtocol 枚举**：`AccessPointType` 本来就是协议类型的抽象（数据库列约束、
`Provider::base_url_for` 已用它分发），让现有概念长出本该属于它的行为，避免并行枚举的概念膨胀。

**新增协议的工作量**：

1. `AccessPointType` 加新变体 + 数据库列约束 + 前端 Select
2. 新建 `protocols/<name>.rs` 实现 5 个 `pub(in crate::domain::shared) fn`
3. **编译器自动指出所有需要补 match 分支的位置**（5 个协议方法 + `Provider::base_url_for`）
4. `InboundRequest`、`UpstreamRequest`、`ProxyCallRecord`、`ProxyPipeline` 等组件零改动 —— 开放性由架构原生保证

**ClientType 正交概念**：`ClientType` 枚举（`ClaudeCode` / `Codex` / `Other` / `Unknown`）与 `AccessPointType`
正交——前者描述"哪个客户端在调用"，后者描述"上游走什么协议"。ClientType 由 `ProxyPipeline` 在协议解析阶段从 User-Agent
和请求路径中识别，随 `InboundRequest` 流入日志管线，最终记录到 `log_metadata.client_type` 和 `log_token_usage.client_type`
列，供日志查询和后续按客户端分群的分析使用。

### 代理转发领域决策（domain/proxy/）

`UpstreamOutcome` 和 `RetryDecision` 是两个一阶领域概念，让响应分类和重试决策从隐式的 if 树中提升为类型系统中的显式 enum：

- **UpstreamOutcome::classify(provider, status, &resp_headers, resp_body, sse)** 是响应分类的唯一入口，统一调用
  `FaultService::detect` 后归类为 `Success` / `ClientError` / `Fault` / `ServerError`。SSE 错误路径无法预读响应体，约定传
  `resp_body=None`，body-based 故障规则被静默忽略（doc 注释已明确）。
- **RetryDecision** 是 `try_one_account` 的返回类型，`Return(Response)` 终止重试、`Continue(AppError)` 切换下一候选 ——
  类型系统强制重试时必须携带错误原因。

## 审计日志标准化

审计日志（`audit_logs` 表）记录系统中所有管理操作的审计轨迹，覆盖 7 个 Service 的 26 个写操作。审计日志仅后端写入，不暴露 API
路由和前端页面。

### 领域模型

**AuditAction 枚举**（`src/domain/log/audit_action.rs`）统一所有审计操作类型，通过 `Display` trait 序列化为 snake_case
字符串写入 VARCHAR 列：

| Variant                 | 序列化字符串                 | 说明                         |
| ----------------------- | ---------------------------- | ---------------------------- |
| Create                  | `create`                     | 创建实体                     |
| Update                  | `update`                     | 非状态类更新（名称、配置）   |
| Delete                  | `delete`                     | 删除实体                     |
| Enable                  | `enable`                     | 启用实体                     |
| Disable                 | `disable`                    | 禁用实体                     |
| Recover                 | `recover`                    | 手动恢复账号                 |
| AutoRecover             | `auto_recover`               | 系统自动恢复账号（定时任务） |
| CreateApiKey            | `create_api_key`             | 创建 API key                 |
| RevokeApiKey            | `revoke_api_key`             | 吊销 API key                 |
| UpdateApiKeyDescription | `update_api_key_description` | 更新 API key 备注            |
| ChangePassword          | `change_password`            | 修改密码                     |
| UpdateProfile           | `update_profile`             | 更新个人资料                 |
| UpdateSettings          | `update_settings`            | 系统设置变更                 |
| Login                   | `login`                      | 登录成功                     |
| LoginFailed             | `login_failed`               | 登录失败（含失败原因）       |
| Logout                  | `logout`                     | 登出                         |
| RefreshRejected         | `refresh_rejected`           | refresh token 被拒绝         |
| DiscoverModels          | `discover_models`            | 模型自动发现                 |

**AuditEntityType 枚举**（`src/domain/log/audit_entity_type.rs`）定义受操作的实体类型：

| Variant        | 序列化字符串      | 说明                      |
| -------------- | ----------------- | ------------------------- |
| AccessPoint    | `access_point`    | 接入点                    |
| Account        | `account`         | 账号                      |
| Provider       | `provider`        | 服务商                    |
| User           | `user`            | 用户                      |
| UserApiKey     | `user_api_key`    | 用户 API key              |
| SystemSettings | `system_settings` | 系统设置                  |
| AuthSession    | `auth_session`    | 认证会话（登录 / 登出）   |
| RefreshToken   | `refresh_token`   | Refresh Token（刷新被拒） |

### 审计日志覆盖率

7 个 Service 的 26 个写操作全部覆盖审计日志，无遗漏：

| Service            | 写操作数 | 覆盖 | 使用的 AuditAction                                                                           |
| ------------------ | -------- | ---- | -------------------------------------------------------------------------------------------- |
| UserService        | 5        | 5    | Create / Update / Delete / Enable / Disable / UpdateProfile / ChangePassword                 |
| UserApiKeyService  | 4        | 4    | CreateApiKey / RevokeApiKey / UpdateApiKeyDescription (列表操作自身即为审计载体，不单独审计) |
| ProviderService    | 4        | 4    | Create / Update / Delete / Enable / Disable / DiscoverModels                                 |
| AccountService     | 6        | 6    | Create / Update / Delete / Enable / Disable / Recover / AutoRecover                          |
| AccessPointService | 3        | 3    | Create / Update / Delete / Enable / Disable                                                  |
| SettingsService    | 1        | 1    | UpdateSettings                                                                               |
| AuthService        | 3        | 3    | Login / LoginFailed / Logout / RefreshRejected                                               |

### 审计策略

- **写入模式**：统一 fire-and-forget + `tracing::error!`，不阻塞主业务逻辑。各 Service 内聚私有 `write_audit_log()`
  辅助方法封装日志构造和异步写入
- **保留策略**：永久保留，不分区、不自动清理。审计日志总量较小，无需分区管理
- **可见性**：仅后端写入，不暴露 API 路由和前端页面。`AuditLogRepository` 提供 `save`（用于写入）和 `find_all_paginated`
  （预留未来审计查询）两个方法
- **敏感信息防护**：details 字段不记录 API key 明文、密码哈希、JWT token 等敏感信息。写入前各 Service 负责过滤敏感字段
- **operator_id 策略**：管理员操作传入 `CurrentUser` 的 UUID，系统自动操作（如 AutoRecover、DiscoverModels）传 `None`

### 审计日志数据结构

`audit_logs` 表 7 个字段，结构自初始 Schema 确立以来未变：

| 字段          | 类型             | 说明                                     |
| ------------- | ---------------- | ---------------------------------------- |
| id            | UUID (PK)        | 审计日志唯一标识                         |
| operator_id   | UUID (nullable)  | 操作者 ID（系统操作为 NULL）             |
| operator_type | VARCHAR          | 操作者类型（如 `admin` / `system`）      |
| action        | VARCHAR          | 操作类型（AuditAction 序列化字符串）     |
| entity_type   | VARCHAR          | 实体类型（AuditEntityType 序列化字符串） |
| entity_id     | UUID (nullable)  | 被操作实体 ID                            |
| details       | JSONB (nullable) | 操作详情（已过滤敏感字段）               |
| timestamp     | TIMESTAMPTZ      | 操作时间                                 |

## 核心架构原则

1. **Domain 层使用 SeaORM 宏定义实体**: 领域实体通过
   DeriveEntityModel、DeriveActiveEnum、DeriveValueType、FromJsonQueryResult 等 SeaORM 宏定义，与基础设施层共用类型系统。消除
   200+ 行 TryFrom/From 手动映射代码，但 domain 代码理论上可调用 SeaORM query API，需通过 code review 约束
2. **依赖反转**: Repository trait 在 domain 定义，infrastructure 实现；Application 引用 trait 而非具体实现；`main.rs`
   完成依赖组装
3. **领域实体即 ORM 实体**: domain/entities 直接使用 SeaORM DeriveEntityModel
   宏，既是领域模型也是数据库映射。行为方法（new、resolve_model、is_expired 等）直接附加在 Model 上
4. **聚合边界明确**: Provider (根+Account)、User (根+RefreshToken)、AccessPoint (根+跨聚合 UUID 引用)、LogEntry (
   根+LogContent)
5. **错误隔离**: 数据库错误和加密错误详情不暴露给客户端，统一转换为 `500 Internal Server Error`
6. **同源部署**: 前端构建产物嵌入 Rust 二进制，生产环境前后端同源，无需 CORS 配置
7. **依赖最小化原则**: 优先复用现有基础设施（PostgreSQL、tokio）解决问题，引入新中间件需要明确的多个使用场景作为合理性论证
8. **双层防御模式**: 前端请求前体检 + 401 兜底双层保障令牌有效性，适配浏览器后台冻结节流策略，不依赖定时器
9. **依赖倒置在认证场景的体现**: RefreshTokenRepository trait 隔离存储实现，切换 Redis 等存储时无需修改 AuthService
10. **账户池故障转移**: 代理转发采用重试循环模式，按 priority 排序遍历账号列表，跳过不可用
    Account，对可重试状态码（429/402/502/503）自动切换到下一个账号，所有账号均不可用时返回错误
11. **二维模型路由**: 模型匹配从原来的线性列表升级为二维路由网格（source_model x provider_id），每个接入点可针对不同的
    Provider 定义差异化的目标模型映射，支持精确匹配、前缀匹配和 **unmatched** 兜底
12. **Account 自动故障检测**: Account 实体支持 disabled_reason 枚举（manual/rate_limited/balance_exhausted/fault）和
    available_at 自动恢复时间戳，is_available() 方法统一检查 status 和禁用原因
13. **Dashboard 个人用量报告**: Dashboard 自 2026-06-24 起从「全局聚合视角」反转为「个人用量报告」视角——项目没有 role 字段，\* \*所有登录用户（含管理员）只能看到自己的数据\*\*。所有 Dashboard 聚合 SQL 强制 `WHERE user_id = ?` 过滤；`LogRepository` 6
    个聚合方法（`aggregate_kpi` / `aggregate_sparkline` / `aggregate_heatmap` / `top_models` / `top_access_points` /
    `quality_metrics`）首参均为 `user_id: Uuid`；展示层 handler 通过 `CurrentUser(user_id): CurrentUser` extractor 从 JWT
    提取并下传。`DashboardService` 仅依赖 `Arc<dyn LogRepository>`，引用对象（access_points / providers）一律 LEFT JOIN 并以
    `Option<String>` 容忍删除。趋势同比通过 `compute_trend` 纯函数覆盖 5 种 TrendBadge 边界（empty / new / down=-100 /
    up / flat / down）
14. **Dashboard 时区参数 SQL 注入防护**: 热力图按用户**浏览器时区**动态分桶（前端
    `Intl.DateTimeFormat().resolvedOptions().timeZone` 传入），后端使用 PostgreSQL `AT TIME ZONE '<tz>'` 字面量分桶。由于
    `AT TIME ZONE` 不接受参数占位符（必须字面量拼接），引入 `application/dashboard/timezone.rs::validate_timezone` 通过
    `chrono_tz::Tz::from_str` 进行 IANA 时区白名单校验，未通过校验立即返回 `AppError::Validation`，确保拼接到 SQL
    的时区字符串只能是合法 IANA 标识符。时区校验置于 application 层而非 domain 层，`LogRepository` trait 接受已校验的
    `&str`，保持 domain 层不依赖时区库
15. **Dashboard 时间维度解耦**: 热力图视图固定使用「近 1 年」窗口（53×7 = 371 天，与时间范围选择器解耦），由独立的
    `get_heatmap` 端点服务；其余卡片（KPI / Top Models / Top Access Points / Quality）共享
    `?range=today|last7|last30|custom` 时间窗口。前端在 useFetch 依赖数组上明确区分：热力图仅依赖 `refreshKey`，其他 4 个
    fetcher 同时依赖 `timeRange + refreshKey`

## 安全设计

| 层面         | 措施                                                                                                           |
| ------------ | -------------------------------------------------------------------------------------------------------------- |
| API Key 存储 | AES-256-GCM 加密，数据库仅存密文                                                                               |
| API Key 展示 | 仅显示末尾 6 位                                                                                                |
| 密码存储     | argon2id 哈希 (慢哈希算法)                                                                                     |
| 认证令牌     | JWT Access Token (30 分钟) + Refresh Token (7 天)                                                              |
| 令牌吊销     | Refresh Token 原子级别 revoked 标记                                                                            |
| 错误隔离     | 加密/数据库错误不暴露原始详情                                                                                  |
| Header 构造  | 上游请求独立构建，入站 `authorization` 只用于用户 API key 认证，provider 认证由账号 API key 单独生成           |
| 传输安全     | 建议部署时配置 HTTPS 反向代理                                                                                  |
| TLS 实现     | reqwest 0.13 默认使用 rustls（纯 Rust TLS），替代 native-tls (OpenSSL)；JWT 签名使用 aws-lc-rs 加密后端        |
| JWT 自动刷新 | 前端「双层防御」: 请求前体检 + 401 兜底，模块级 Promise 并发去重                                               |
| 过期令牌清理 | tokio 后台任务每小时物理删除过期 refresh_token，不引入 Redis 或 pg_cron; 多副本部署时通过 advisory lock 防冲突 |

## 构建与部署

### 开发模式

```bash
cargo make dev          # 并行启动 Vite HMR + cargo run
```

前端热更新通过 Vite dev server (端口 5173)，API 请求通过 Vite proxy 代理到后端 (端口 3000)。

### 生产构建

```bash
cargo make build        # 顺序: npm build → cargo build --release
cargo make preview      # 构建并运行 release 版本
```

### Docker 部署

Dockerfile 分三阶段构建，`.dockerignore` 排除 `target/`、`node_modules/` 等构建上下文中的无关文件，加速远程构建:

1. **frontend-builder**: Node 22 Alpine — npm ci + npm run build
2. **backend-builder**: Rust 1.96 Alpine — cargo build --release (reqwest 使用 rustls TLS, 无需 OpenSSL 系统库;
   嵌入前端产物)
3. **runtime**: Alpine 3.22 — 仅包含二进制和运行时依赖 (ca-certificates + tzdata + libgcc)

镜像通过 CI 发布到 `ghcr.io/your-org/token-proxy:latest`，直接 `docker run` 即可启动。

## CI/CD

### GitHub Actions

`.github/workflows/ci.yml` 在 PR → main 时触发，包含 3 个并行 job:

| Job                | 步骤                                           | 说明                      |
| ------------------ | ---------------------------------------------- | ------------------------- |
| **check-backend**  | cargo fmt --check + cargo clippy + cargo build | 后端格式、lint 和编译检查 |
| **check-frontend** | npm lint + tsc --noEmit + npm run build        | 前端 lint、类型检查和构建 |
| **test**           | cargo test (PostgreSQL 17 服务容器)            | 数据库集成测试            |

缓存策略: 后端使用 `Swatinem/rust-cache@v2` 加速依赖恢复。

### Dependabot

`.github/dependabot.yml` 配置每周检查 cargo 和 npm 依赖更新，自动提交 PR:

| 生态  | 检查目录 | 频率 |
| ----- | -------- | ---- |
| cargo | `/`      | 每周 |
| npm   | `/`      | 每周 |

### Makefile 任务

| 命令                 | 说明                                   |
| -------------------- | -------------------------------------- |
| `cargo make dev`     | 并行启动前端 Vite HMR + 后端 cargo run |
| `cargo make build`   | 顺序构建前端 + 后端 release            |
| `cargo make check`   | 并行 cargo check + tsc --noEmit        |
| `cargo make preview` | build 并运行 release 二进制            |
| `cargo make fmt`     | cargo fmt                              |
| `cargo make clippy`  | cargo clippy (deny warnings)           |
| `cargo make test`    | cargo test                             |
| `cargo make clean`   | cargo clean                            |

## 工具链

### 代码格式化

| 工具     | 配置                              | 用途                                                                                    |
| -------- | --------------------------------- | --------------------------------------------------------------------------------------- |
| Prettier | `.prettierrc` + `.prettierignore` | TypeScript / CSS / Markdown 格式化 (semi + singleQuote + trailingComma, printWidth 100) |
| rustfmt  | `rust-toolchain.toml` (组件)      | Rust 代码格式化，集成至 Makefile (`cargo make fmt`)                                     |

### Git Hooks

使用 simple-git-hooks + lint-staged 实现提交前自动检查:

| Hook       | 触发         | 检查内容                                                           |
| ---------- | ------------ | ------------------------------------------------------------------ |
| pre-commit | `git commit` | ts/tsx → eslint + prettier; json/css/md → prettier; rs → cargo fmt |

配置声明于 `package.json` 的 `simple-git-hooks` 字段，`lint-staged` 定义文件过滤器。

### 变更日志

`cliff.toml` 配置 git-cliff，基于约定式提交自动生成 CHANGELOG。提交分组规则:

| 提交类型               | CHANGELOG 分组 |
| ---------------------- | -------------- |
| feat                   | Added          |
| fix                    | Fixed          |
| perf, refactor         | Changed        |
| doc                    | Documentation  |
| test, ci, chore, style | Miscellaneous  |

### Rust 工具链

`rust-toolchain.toml` 固定 Rust 1.96，包含 clippy 和 rustfmt 组件。确保所有开发环境和 CI 使用一致的工具链版本。

## 版本管理与发布流程

### 版本号约定

| 分支       | 版本号     | 说明                                   |
| ---------- | ---------- | -------------------------------------- |
| main       | 0.0.0      | 占位版本，仅用于开发集成               |
| release/\* | 语义化版本 | 实际发布版本，如 `0.1.0`、`0.2.0-rc.1` |

- Git tag 不带 `v` 前缀（`0.1.0` 而非 `v0.1.0`）
- CHANGELOG 按发布日期倒序，兼容 Keep a Changelog 格式

### 发布流程

通过 Claude Code `/release <version> [<description>]` 技能执行:

1. 基于 `main` 创建 `release/<major>.<minor>` 分支
2. 运行 `cargo make check` 确保零错误零警告
3. 运行 `cliff-tag <version>` 生成 CHANGELOG 并创建 git tag
4. 手动编辑 CHANGELOG 确认分组合理，补充未覆盖的变更
5. 执行两个提交: CHANGELOG 更新 + 版本号更新
6. 将 CHANGELOG 提交 cherry-pick 回 main 分支（保持 main 可追溯但版本号占位 0.0.0）

## 项目状态

| 维度        | 状态                                                                          |
| ----------- | ----------------------------------------------------------------------------- |
| Phase 1 MVP | 已完成                                                                        |
| 后端        | ~170 个 .rs 文件, cargo check 零错误零警告                                    |
| 前端        | ~73 个 .ts/.tsx 源文件, tsc --noEmit 零错误                                   |
| Schema 迁移 | 3 个迁移文件 (初始表 + 账户池 + client_type)                                  |
| Docker 构建 | 多阶段构建就绪 (含 .dockerignore + HEALTHCHECK)                               |
| 镜像分发    | GitHub Container Registry (`ghcr.io/your-org/token-proxy`)                    |
| CI          | GitHub Actions 3 并行 job (后端检查 + 前端检查 + 集成测试)                    |
| 依赖更新    | Dependabot 每周自动检查 cargo 和 npm                                          |
| 代码格式化  | Prettier (前端) + rustfmt (后端), pre-commit hook 自动执行                    |
| 变更日志    | git-cliff 基于约定式提交生成 CHANGELOG                                        |
| 工具链固定  | rust-toolchain.toml 锁定 Rust 1.96                                            |
| 发布流程    | `/release` Claude Code 技能 + release/\* 分支 + cherry-pick CHANGELOG 回 main |

## 变更记录

| 日期             | 变更说明                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-06-24       | Dashboard 个人用量报告重构: 视角从「全局聚合」反转为「个人用量报告」——项目无 role 字段，所有用户（含管理员）只看自己。`LogRepository` 删除 `top_users` / `top_accounts`，新增 `aggregate_heatmap` / `top_models` / `top_access_points` / `quality_metrics` 共 4 个个人视角聚合方法；`aggregate_kpi` / `aggregate_sparkline` 改造首参为 `user_id: Uuid`；所有聚合 SQL 强制 `WHERE user_id = ?` 过滤。领域读模型扩展：`KpiAggregate` 删除 `active_user_count`、新增 5 项词元（input/output/cache_creation/cache_read/thinking），新增 `HeatmapCell` / `TopModelRow` / `TopAccessPointRow` / `QualityMetrics`，删除 `TopUserRow` / `TopAccountRow`。新增 `application/dashboard/timezone.rs` 用 `chrono_tz::Tz::from_str` 实现 IANA 时区白名单校验，拦截 PostgreSQL `AT TIME ZONE` 字面量拼接的 SQL 注入风险；时区校验置于 application 层，domain trait 接受已校验 `&str`。路由端点变更为 5 个（`/api/getting-started/{kpi,heatmap,top-models,top-access-points,quality}`），handler 注入 `CurrentUser(user_id)` extractor。前端：新增 `Heatmap.tsx`（GitHub 风格 53×7 方格矩阵 + 分位数色阶, 固定近 1 年窗口）/ `TopModelsRanking.tsx` / `TopAccessPointsRanking.tsx` / `QualityCard.tsx`，删除 `TopUsersRanking.tsx` / `TopAccountsRanking.tsx` / `TopClientsRanking.tsx`；GettingStartedPage 改为 5 个 useFetch 并行，热力图 deps 与 timeRange 解耦（仅依赖 refreshKey）；Dashboard 移除 SSE 订阅；浏览器时区通过 `Intl.DateTimeFormat().resolvedOptions().timeZone` 获取并作为 query 参数传后端。新增依赖 `chrono-tz = "0.10"`。新增核心架构原则 13 (个人用量报告) / 14 (时区 SQL 注入防护) / 15 (时间维度解耦)，替换旧的「Dashboard 只读视图」原则 |
| 2026-06-23       | 审计日志标准化: 新增 `AuditAction` 枚举（20 variants, domain/log/audit_action.rs）和 `AuditEntityType` 枚举（8 variants, domain/log/audit_entity_type.rs），通过 Display trait 序列化为 snake_case 字符串写入 VARCHAR 列。7 个 Service 的 26 个写操作全部覆盖审计日志（UserService 5 / UserApiKeyService 4 / ProviderService 4 / AccountService 6 / AccessPointService 3 / SettingsService 1 / AuthService 3），各 Service 内聚私有 `write_audit_log()` 辅助方法，统一 fire-and-forget + tracing::error! 模式。领域层、数据库 schema 无变更（audit_logs 表结构保持 7 字段不变）。审计日志永久保留、不分区、仅后端写入、不暴露 API 和前端页面。operator_id 管理员操作传入 CurrentUser，系统自动操作传 None。details 不记录 API key 明文、密码哈希、JWT token 等敏感信息                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 2026-06-23       | 前端日志实时刷新（SSE 推送）: 新增 `NewLogEvent` DTO（`application/log/dto/new_log_event.rs`），LogService 集成 `broadcast::Sender` 在日志写入后广播事件；展示层新增 `GET /api/logs/events` SSE 端点（JWT 保护，`text/event-stream`，响应 `shutdown_rx` 优雅关闭信号）；AppState 新增 `log_event_tx` 和 `shutdown_rx` 字段；main.rs 创建 `broadcast::channel::<NewLogEvent>(256)`。选择 SSE 而非 WebSocket（仅需单向推送，axum 原生支持，EventSource 自动重连）；broadcast 容量 256 满时丢弃最旧事件，前端全量刷新兜底；JWT 通过 URL query 参数传递（EventSource API 不支持自定义 header）。前端：新增 `useLogEvents.ts` hook（EventSource 连接管理）、`ConnectionIndicator.tsx`（SSE 连接状态绿/黄/红三色圆点指示器）；`useFetch.ts` 改造 refetch 时 `setLoading(true)`；`RequestLogPage.tsx`、`SessionLogPage.tsx` 集成 SSE 自动刷新；`SessionDetailView.tsx` 新增 `beforeRefresh` prop。领域层和基础设施层无变更，数据库 schema 无变更                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 2026-06-23       | OpenAI 协议支持: `AccessPointType` 新增 `OpenAi` 变体 + 新建 `domain/shared/protocols/openai.rs` 实现 Chat Completions + Responses API 双端点协议适配；新增 `ClientType` 枚举（`domain/shared/client_type.rs`，ClaudeCode / Codex / Other / Unknown）与 `AccessPointType` 正交——前者描述调用客户端，后者描述上游协议。`InboundRequest` 新增 `client_type` 字段，`ProxyPipeline` 协议解析阶段从 User-Agent 和请求路径识别 ClientType；`log_metadata` 和 `log_token_usage` 新增 `client_type` 列（迁移 `m20260623_000003_client_type`）；`LogRepository` 新增 `top_clients` 聚合方法；`DashboardService` 新增 `get_top_clients`；`GET /api/getting-started/top-clients` 端点。前端：`parseOpenAI.ts` OpenAI 响应/请求解析器；`AccessPointDrawer` 启用 OpenAI 选项 + MODEL_FAMILIES 按 api_type 动态切换；`RequestContentCard` / `ResponseContentCard` 支持 OpenAI 协议渲染；`buildConversationTurns` 按 api_type 分发；`TopClientsRanking` 排行卡片；`GettingStartedPage` 集成第 4 个 `useFetch`。基础设施：`parsed_token_usage.rs` 扩展支持 OpenAI Chat/Responses 词元格式；`Provider::base_url_for` 补 `OpenAi` 分支                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 2026-06-23       | Dashboard 数据分析重做: 新增 `application/dashboard/` 模块（`dashboard_service.rs` + `time_window.rs` + 12 个 DTO），仅依赖 `LogRepository`；`LogRepository` trait 扩展 4 个聚合方法（`aggregate_kpi` / `aggregate_sparkline` / `top_users` / `top_accounts`），SQL 统一用 `Statement::from_sql_and_values` 配合 `generate_series` 在 SQL 层补齐空桶；新增 `domain/log/dashboard_query.rs` 领域读模型（DashboardWindow / KpiAggregate / SparklineBucket / TopUserRow / TopAccountRow，LEFT JOIN 容忍删除）。新增 3 个 JWT 保护端点：`GET /api/getting-started/kpi` (KPI + 内嵌 sparkline) / `top-users` / `top-accounts`。删除旧 `stats_routes.rs` + `stats/dto/` 目录、`LogService` 4 个统计方法、`LogRepository` 5 个旧方法。前端 `components/dashboard/` 重写为 8 个组件（Sparkline / ComparisonArrow / StackedBar / KpiCard / CacheHitCard / TimeRangeSelector / TopUsersRanking / TopAccountsRanking），删除旧 `StatCard` / `TrendChart`；`GettingStartedPage.tsx` 完全重写为 CSS Grid 布局（1280/768 响应式断点）+ 3 个并行 `useFetch`；新增 `recharts@^3.8.1` 依赖、`formatTokenCompact` 工具函数、`.dashboard-deleted` 全局 CSS 类。新增架构原则 13「Dashboard 只读视图」                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 2026-06-22       | 前端 TypeScript/ESLint 错误修复 (共 20 个): 新增通用数据获取 hook `useFetch.ts`（替代 11 个文件中的手动 `useState + useCallback + useEffect` 模式）；组件工具函数分离模式（`ModelMappingEditor.tsx` → `modelMappingUtils.ts`、`TokenUsageCard.tsx` → `tokenUsage.ts`）；派生状态模式（`AdminLayout` 和 `AccessPointDrawer` 中的 `useState + useEffect` 改为 `useMemo`）；tsconfig 移除 `baseUrl`（TypeScript 7.0 废弃）、paths 改为 `./` 相对路径。前端源文件数从 64 更新为 67                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 2026-06-22       | 前端路由调整: ProfilePage 路由从 /settings/profile 移为 /profile，与系统设置路由平整分离。修改 App.tsx 路由定义和 AdminLayout Header Dropdown 导航路径，侧边栏无需新增导航项                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 2026-06-18       | 接入点账户池重构: 新增 access_point_accounts 表（多对多关联）+ session_affinity 表（会话粘滞）+ ModelRoutingGrid JSONB 替代线性 model_mappings（二维网格匹配）+ RoutingStrategy 枚举（Priority/Weighted）+ Account 新增 DisabledReason 枚举（manual/rate_limited/balance_exhausted/fault）和 available_at + Provider 新增 rate_limit_config/balance_exhausted_config + audit_logs 的 user_id→operator_id + 新增 operator_type + 删除 access_points 的 provider_id/account_id/model_mappings + 删除 providers 的 default_model。新增 3 个领域文件（access_point_account.rs/model_routing_grid.rs/routing_strategy.rs）、2 个 Repository（access_point_account_repository/session_affinity_repository）、2 个 DTO（account_dto/model_routing_grid_dto）、1 个迁移文件（account_pool）。代理转发改为账号重试循环模式，跳过不可用 Account，对 429/402/502/503 自动重试。应用层 dto 目录从单文件扩展为 dto/ 子目录多文件模式                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 2026-06-18（续） | 前端组件目录重组: 将 `src-dashboard/components/` 中 25 个平铺组件按功能分组为 common/、access-point/、provider/、log/、session/、dashboard/、user/ 7 个子目录; log-detail/ 作为 log/ 的内聚子组。删除旧空目录 (charts/、log-viewer/、timeline/)。提取 2 个新组件 (AccountManager、ApiKeyManager)。配置 `@components` Vite 路径别名, 所有组件间 import 统一使用别名格式                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 2026-05-19       | 初始化架构文档，记录 DDD 四层架构、代理转发流程、安全设计和项目状态                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 2026-05-20       | 应用层分区管理替代 pg_partman：新增 PartitionManager，迁移移除 pg_partman 依赖改为原生分区语法 + 种子分区，Config 新增 3 个分区配置项，main.rs 新增分区初始化和后台定时任务                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 2026-05-24       | 调整代理 Header 构造语义：`ProxyClient` 独立构建上游请求，入站 `authorization` 只用于用户 API key 认证，上游 provider 认证由账号 API key 单独生成；同时实现 `decrypt_account_key` 解密逻辑（从 stub 变为完整实现）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 2026-05-24       | Provider 增加 `default_model` 字段（全链路：domain entity、SeaORM entity、DTO、service、migration）；CreateAccessPointRequest 支持 `api_type` 参数（当前有效类型为 Anthropic）；ModelMapping 增加 `MatchType`（exact/prefix）和常量（`UNMATCHED_MODEL_SENTINEL`、Claude 模型族前缀）；实现统一模型匹配逻辑（精确 > 前缀 > `__unmatched__` > Provider.default_model），代理路由使用统一匹配并记录最终 `model_mapped`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 2026-05-24       | 前端新增主题切换系统：`useTheme` hook（light/dark/system 三种模式）、`ThemeToggle` 组件、`ThemeProvider` 包裹根组件；接入点表单新增 `api_type` 选择器和 `ModelMappingEditor`（支持 Anthropic 模型族 Opus/Sonnet/Haiku 快捷添加前缀匹配规则）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 2026-05-24       | 前端 Provider 表格 default_model 列使用 Tag 渲染; Provider 编辑面板 default_model Select 移至模型列表 TagInput 下方, TagInput 移除模型联动清空 default_model; ModelMappingEditor 源模型下拉展示匹配类型说明, 目标模型下拉仅含 Provider 已注册 models 且禁止创建; 保存时过滤 target_model 不在 Provider.models 的映射 (useAccessPoints hook 实现)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 2026-05-24       | 同步架构文档与实际代码：`__unmatched__` 视为模式匹配, 自动生成的未匹配规则使用 prefix; Select 选项用 Semi Tag 前缀显示"精准匹配/模式匹配"; 目标模型 Select 包含 Provider.models + Provider.default_model; 保存过滤也允许 Provider.default_model                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2026-05-24       | 服务端强化匹配类型: 新增 `normalize_match_type` 和 `is_prefix_source_model` 函数, 强制 `__unmatched__` 和 Claude 家族前缀 (claude-opus-/claude-sonnet-/claude-haiku-) 始终视为 `prefix` 匹配; AccessPointService 创建/更新时执行 match_type 标准化; 前端 ModelMappingEditor 对 apiType 做大小写兼容                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 2026-05-29       | 实体合并改造: 将 SeaORM DeriveEntityModel 从 `infrastructure/persistence/entities/` 迁移到 `domain/entities/`，删除基础设施层 entities 目录。domain 层引入 SeaORM 宏依赖，消除 200+ 行 TryFrom/From 手工映射代码。领域实体即 ORM 实体，不再区分                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2026-05-27       | 前端组件架构拆分: 从 RequestLogPage 提取 `RequestLogTable` 组件 (表格列定义 + Table 渲染); 从 SessionLogPage 提取 `SessionListView` (会话列表视图) 和 `SessionDetailView` (会话详情视图); SessionLogPage 瘦身为路由壳, 根据 sessionId 参数切换列表/详情视图; 新增 `/logs/:id` 路由和 `LogDetailPage` 页面; 前端源文件数更新为 45 个                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 2026-05-26       | 认证体系优化: 前端 `api.ts` 采用「双层防御」策略（请求前体检 + 401 兜底），模块级 Promise 并发去重，解决浏览器冻结导致定时器失效问题；`JwtService` 新增 `refresh_expiry_secs` 访问器，修复 AuthService 两处误用 access 寿命写入 refresh_token expires_at 的 bug；新增 tokio 后台任务每小时物理清理过期 refresh_token，明确拒绝引入 Redis 或 pg_cron，遵循依赖最小化原则；新增架构原则 7-9（依赖最小化、双层防御、依赖倒置认证场景体现）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 2026-06-03       | 领域层聚合重构: 将 domain/ 层从按技术类别（entities/value_objects/repositories/services）重组为按聚合边界（access_point/provider/user/log/shared）组织。AccessPoint 引入 AccessPointEx 聚合根（自定义 struct，含 access_point + accounts），Repository 的 `find_by_short_code` 返回已加载 Provider 和 Account 关联的完整聚合。ProxyPipeline 删除 `select_base_url` 和 `decrypt_account_key` 方法，全部操作委托 AccessPointEx 行为方法（base_url、resolve_model、validate_usable、decrypt_upstream_key）。Provider 新增 `base_url_for` 方法。AccessPointType 移至 shared 解决循环依赖。account_id 退化为纯 FK 列（不定义 belongs_to 关系）。Relation 定义保持 DeriveRelation 枚举语法（SeaORM 2.0-rc.38 兼容性）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2026-06-17       | 应用层按聚合重构: 将 application/ 层从按技术类别（services/ + dto/）重组为按聚合组织（access_point/auth/log/provider/proxy/user），与领域层聚合命名对齐。auth/ 和 proxy/ 作为跨聚合编排服务独立归置。删除已废弃的 `domain/shared/api_protocol.rs`，替换为 `RequestSnapshot` 值对象（内聚 to HeaderMap 变换、模型提取、流检测、会话提取行为）。删除已废弃的 `infrastructure/protocols/` 目录。引入目录组织原则（相对/绝对路径规则）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 2026-06-17       | Proxy 防腐层重构: 新增 `log_anti_corruption.rs` 实现 LogContext 防腐层，从 ProcessedRequest 和 AccessPointEx 中一次性提取所有日志参数。pipeline.rs 将日志数据提取、LogEntry 构造、LogTaskContext 组装和 InterruptGuard 创建全部委托给防腐层处理。InterruptGuard 从 7 个独立字段简化为持有 LogContext。遵循 DDD 防腐层模式，保证代理转发逻辑不被日志格式细节侵蚀                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2026-06-17       | 透明代理 + ProxyLogger 日志架构重写: 确立透明代理原则——上游响应原样透传，仅过滤 hop-by-hop 头；传输方式判断由请求体 `stream` 字段改为响应头 `content-type`；日志架构从双路径（InterruptGuard + spawn_log_task）简化为 ProxyLogger 积累器模式（统一 flush，Drop 自动检测中断）；删除 `interrupt_guard.rs` 和 `ProcessedRequest.is_streaming`；`record_proxy_log` 强化三阶段独立错误处理；同步更新基础设施层目录结构（新增 parsers/、http_client/ 拆为 processed_request + proxy_client）为实际文件布局                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 2026-06-20       | 会话详情视图重构: 新增 TurnCard（轮次卡片）、TurnNavigator（轮次导航条）两个组件；删除 ClaudeSessionTimeline（被 TurnCard 替代）；SessionDetailView 从事件流+摘要表格改为轮次导航+轮次卡片列表；types/log.ts 新增 ConversationTurn / TurnBlock / TurnTokenSummary 轮次-块-摘要三级类型；parseLogs.ts 新增 buildConversationTurns() 核心函数                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 2026-06-22       | Rust 依赖批量升级 (12 个依赖): reqwest 0.12→0.13（默认 TLS 从 native-tls 切换为 rustls, 不再需要 OpenSSL 系统库）、jsonwebtoken 9→10 并新增 aws-lc-rs 加密后端、tower-http 0.6→0.7、sea-orm rc.38→rc.41、rand 0.8→0.10 (RngCore→Rng API 迁移)、sha2 0.10→0.11 (finalize 返回类型变更); 新增 hex 0.4 依赖。5 个 .rs 文件 API 迁移, 逻辑零变更                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

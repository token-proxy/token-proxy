# Token Proxy 架构文档

此工程使用 Rust + axum 框架，在后端采用领域驱动设计四层架构（Domain / Application / Infrastructure / Presentation），前端使用 React + TypeScript + Vite + Semi Design 构建 SPA，数据库使用 PostgreSQL 17，log_metadata 表通过 PostgreSQL 原生分区语法（PARTITION BY RANGE）按月分区，由应用层 PartitionManager 自动管理分区生命周期。

## 目录结构总览

```
├── src/                    # 后端 Rust 核心代码 (103 个 .rs 文件)
├── frontend/               # 前端 React SPA (15 个 .ts/.tsx 源文件)
├── migration/              # 数据库迁移 (独立 workspace crate)
├── target/                 # Rust 构建产物
├── node_modules/           # 前端依赖 (root 级, 供 cargo-make 使用)
├── .claude/                # Claude Code 项目配置
├── Cargo.toml              # Rust workspace 定义 (依赖管理)
├── Cargo.lock              # Rust 依赖锁文件
├── Makefile.toml           # cargo-make 任务编排
├── Dockerfile              # 多阶段 Docker 构建 (Node 22 → Rust 1.89 → Alpine 3.21)
├── docker-compose.yml      # PostgreSQL 17 + App 容器编排
└── product.md              # 产品需求文档
```

## 后端架构详解 (src/)

后端按照**领域驱动设计 (DDD) 四层架构**组织，遵循依赖反转原则，domain 层零外部框架依赖。

```
src/
├── domain/                 # 领域层 (零外部框架依赖)
│   ├── entities/           # 7 个业务实体
│   ├── value_objects/      # 5 个值对象
│   ├── repositories/       # 8 个 Repository trait
│   └── services/           # 2 个领域服务
├── application/            # 应用层 (用例编排, 依赖注入)
│   ├── dto/                # 6 组 DTO
│   ├── services/           # 8 个应用服务
│   └── mod.rs              # AppState 全局共享状态
├── infrastructure/         # 基础设施层
│   ├── persistence/        # SeaORM 实体 (10 个) + Repository 实现 (7 个)
│   ├── encryption/         # AES-256-GCM 加密服务
│   ├── auth/               # JWT 认证 + argon2 密码哈希
│   └── http_client/        # reqwest 代理转发客户端
├── presentation/           # 展示层
│   ├── routes/             # 8 组 axum 路由处理器
│   └── middleware/         # JWT 认证中间件
├── shared/                 # 共享模块
│   ├── error.rs            # AppError (9 种错误变体)
│   └── types.rs            # PaginatedResult, PaginationParams, Timestamp
├── config.rs               # 环境变量配置加载
├── main.rs                 # 启动入口 (依赖组装 + 路由构建 + 分区初始化 + 后台定时分区维护)
└── lib.rs                  # Crate 根模块 (模块导出)
```

### 领域层 (domain/)

领域层是整个架构的核心，**不依赖任何外部框架**（axum、SeaORM、reqwest），只使用 Rust 标准库 + serde + uuid + chrono + async-trait。

```
domain/
├── entities/               # 纯业务 struct, 包含领域校验逻辑
│   ├── provider.rs         # LLM 提供商 (OpenAI/Anthropic)
│   ├── account.rs          # API 账号 (跨聚合引用 Provider)
│   ├── user.rs             # 管理员用户
│   ├── access_point.rs     # 接入点 (跨聚合引用 Provider + Account)
│   ├── refresh_token.rs    # JWT 刷新令牌
│   ├── user_api_key.rs     # 用户 API key (SHA-256 哈希存储)
│   └── log_entry.rs        # 日志条目 + 日志内容
├── value_objects/          # 不可变值对象
│   ├── short_code.rs       # 接入点短码 (生成/校验)
│   ├── api_key.rs          # API Key (掩码展示)
│   ├── model_mapping.rs    # 模型映射对 (source → target)
│   ├── status.rs           # 启用/禁用状态枚举
│   └── access_point_type.rs # 接入点类型枚举
├── repositories/           # Repository trait (接口定义)
│   ├── provider_repository.rs
│   ├── account_repository.rs
│   ├── user_repository.rs
│   ├── access_point_repository.rs
│   ├── refresh_token_repository.rs
│   ├── log_repository.rs
│   └── user_api_key_repository.rs
└── services/               # 领域服务
    ├── encryption_service.rs  # 加密服务 trait (encrypt/decrypt)
    └── model_mapping_service.rs # 模型映射纯函数 (含单元测试)
```

**领域实体 ≠ ORM 实体**：domain/entities 是纯业务 Rust struct，infrastructure/persistence/entities 是 SeaORM `DeriveEntityModel`，repository 实现中完成手工映射。

**聚合边界**：

| 聚合根 | 包含子实体 | 跨聚合引用 |
|--------|-----------|-----------|
| Provider | Account | - |
| User | RefreshToken, UserApiKey | - |
| AccessPoint | - | provider_id, account_id (Uuid) |
| LogEntry | LogContent | user_id, access_point_id, provider_id, account_id (Uuid) |

### 应用层 (application/)

应用层负责用例编排，通过构造函数注入 domain 层的 Repository trait，**不直接依赖 SeaORM**。

```
application/
├── dto/                    # 请求/响应数据传输对象
│   ├── provider_dto.rs     # Provider 增改查 DTO
│   ├── account_dto.rs      # Account 增改查 DTO (不含完整 Key)
│   ├── user_dto.rs         # User 增改查 DTO + 个人设置 (profile/密码/API key)
│   ├── access_point_dto.rs # AccessPoint 增改查 DTO
│   ├── auth_dto.rs         # Login/Refresh/TokenPair DTO
│   └── log_dto.rs          # 日志查询 DTO
├── services/               # 应用服务 (注入 Repository traits)
│   ├── provider_service.rs # 提供商管理用例
│   ├── account_service.rs  # 账号管理用例 (含加密解密)
│   ├── user_service.rs     # 用户管理用例 (含密码哈希 + profile 更新 + 密码修改)
│   ├── user_api_key_service.rs # 用户 API key 管理 (生成/列表/撤销, SHA-256 哈希)
│   ├── access_point_service.rs # 接入点管理用例 (含短码生成)
│   ├── auth_service.rs     # 认证用例 (登录/刷新/登出)
│   ├── proxy_service.rs    # 核心代理转发用例 (含 Bearer API key 认证)
│   └── log_service.rs      # 日志查询用例
└── mod.rs                  # AppState 定义 (所有 Service 的引用容器)
```

**AppState** 是全局共享状态，通过 axum 的 `with_state()` 注入到所有路由处理器，包含 Config、数据库连接、所有 Service 引用、JWT 服务和代理客户端。

### 基础设施层 (infrastructure/)

基础设施层实现 domain 层定义的接口，处理所有外部依赖。

```
infrastructure/
├── persistence/            # SeaORM 数据持久化
│   ├── entities/           # ORM 实体 (8 个)
│   │   ├── provider.rs     # 映射 providers 表
│   │   ├── account.rs      # 映射 accounts 表
│   │   ├── user.rs         # 映射 users 表
│   │   ├── access_point.rs # 映射 access_points 表
│   │   ├── refresh_token.rs # 映射 refresh_tokens 表
│   │   ├── log_metadata.rs # 映射 log_metadata 表 (按月分区)
│   │   ├── log_content.rs  # 映射 log_contents 表
│   │   ├── audit_log.rs    # 映射 audit_logs 表
│   │   └── user_api_key.rs # 映射 user_api_keys 表
│   ├── partition_manager.rs # PartitionManager: 应用层分区自动管理
│   └── repositories/       # Repository 实现 (7 个)
│       ├── provider_repository.rs        # SeaOrmProviderRepository
│       ├── account_repository.rs         # SeaOrmAccountRepository
│       ├── user_repository.rs            # SeaOrmUserRepository
│       ├── access_point_repository.rs    # SeaOrmAccessPointRepository
│       ├── refresh_token_repository.rs   # SeaOrmRefreshTokenRepository
│       ├── log_repository.rs             # SeaOrmLogRepository
│       └── user_api_key_repository.rs    # SeaOrmUserApiKeyRepository
├── encryption/             # 加密实现
│   └── aes256_gcm.rs       # Aes256GcmEncryptionService
├── auth/                   # 认证实现
│   ├── jwt.rs              # JwtService (jsonwebtoken)
│   └── password.rs         # argon2 密码哈希
└── http_client/            # HTTP 客户端
    └── proxy_client.rs     # ProxyClient (reqwest 连接池)
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
│   ├── access_point_routes.rs # CRUD /api/access-points
│   ├── proxy_routes.rs     # POST /ap/{short_code}/v1/messages (强制 API key 认证)
│   └── log_routes.rs       # GET /api/logs, /api/logs/sessions, /api/logs/sessions/:id
└── middleware/             # 中间件
    └── jwt_auth.rs         # JWT 认证中间件 + CurrentUser extractor
```

**路由认证策略**：

| 路径 | 认证要求 |
|------|---------|
| `/api/auth/*` | 公开 (登录/刷新) |
| `/ap/*` | Bearer 用户 API key 认证 (SHA-256, Authorization 头) |
| `/api/health` | 公开 |
| `/api/providers/*` | JWT 认证 |
| `/api/accounts/*` | JWT 认证 |
| `/api/users/*` | JWT 认证 |
| `/api/users/me/*` | JWT 认证 (当前用户个人设置) |
| `/api/access-points/*` | JWT 认证 |
| `/api/logs/*` | JWT 认证 |

### 共享模块 (shared/)

```
shared/
├── error.rs    # AppError 枚举 (9 种变体 + IntoResponse 实现)
└── types.rs    # PaginatedResult<T>, PaginationParams, Timestamp
```

**AppError 错误类型**：

| 变体 | HTTP 状态码 | 说明 |
|------|------------|------|
| Validation | 400 | 请求参数校验失败 |
| NotFound | 404 | 资源未找到 |
| Conflict | 409 | 资源冲突（如重名） |
| Unauthorized | 401 | 未认证或令牌无效 |
| Forbidden | 403 | 无操作权限 |
| Encryption | 500 | 加密/解密错误 (不暴露详情) |
| Database | 500 | 数据库错误 (不暴露详情) |
| Upstream | 502 | 上游 LLM 服务错误 |
| Internal | 500 | 内部服务器错误 |

### 配置加载 (config.rs)

从环境变量加载运行时配置，所有必填变量在启动时验证：

| 变量 | 类型 | 说明 | 默认值 |
|------|------|------|--------|
| DATABASE_URL | String | PostgreSQL 连接串 | **必填** |
| JWT_SECRET | String | JWT 签名密钥 | **必填** |
| ENCRYPTION_KEY | String | 64 位十六进制 (32 字节) | **必填** |
| SERVER_PORT | u16 | 监听端口 | 3000 |
| LOG_LEVEL | String | 日志级别 | info |
| PARTITION_CHECK_INTERVAL_SECS | u64 | 分区检查间隔（秒） | 3600 |
| PARTITION_PREMAKE_MONTHS | u32 | 提前创建未来分区数 | 3 |
| PARTITION_RETENTION_MONTHS | u32 | 分区保留月数 | 12 |

## 前端架构详解 (frontend/)

前端是单页应用 (SPA)，构建产物通过 `rust-embed` 嵌入 Rust 二进制，生产环境与后端同源部署。

```
frontend/
├── src/
│   ├── main.tsx                    # React 入口
│   ├── App.tsx                     # 路由定义 (react-router-dom v7)
│   ├── api.ts                      # API 通信层 (fetch 封装)
│   ├── layouts/
│   │   └── AdminLayout.tsx         # 管理界面布局 (Semi Design Navigation)
│   └── pages/
│       ├── LoginPage.tsx           # POST /api/auth/login
│       ├── DashboardPage.tsx       # 仪表盘概览
│       ├── ProviderManagement.tsx  # CRUD /api/providers
│       ├── AccessPointManagement.tsx # CRUD /api/access-points
│       ├── UserManagement.tsx      # CRUD /api/users
│       ├── ProfilePage.tsx         # 个人设置 (profile/密码/API key 管理)
│       ├── SessionLogPage.tsx      # GET /api/logs/sessions
│       ├── RequestLogPage.tsx      # GET /api/logs
│       └── SettingsPage.tsx        # 设置页面
├── tsconfig.json                   # TypeScript 配置
├── vite.config.ts                  # Vite 构建配置
└── package.json                    # 依赖声明
```

### 前端路由结构

```
/login                  → LoginPage
/                       → AdminLayout (重定向至 /dashboard)
  /dashboard            → DashboardPage
  /providers/*          → ProviderManagement
  /access-points        → AccessPointManagement
  /sessions             → SessionLogPage
  /sessions/:sessionId  → SessionLogPage (单会话详情)
  /logs                 → RequestLogPage
  /users                → UserManagement
  /settings             → SettingsPage
	  /settings/profile     → ProfilePage (个人设置)
```

### API 通信层

`api.ts` 封装了基于 fetch 的 HTTP 客户端，自动附加 JWT `Authorization` 头，401 响应时自动清除令牌并跳转登录页。提供 `get`、`post`、`put`、`delete` 四个方法。

## 数据库架构详解 (migration/)

迁移使用独立的 workspace crate (`migration/`)，基于 `sea-orm-migration`。

```
migration/
├── Cargo.toml        # 依赖: sea-orm-migration
└── src/
    ├── lib.rs
    ├── m20260101_000001_initial.rs              # 初始 Schema (8 个表 + 物化视图)
    └── m20260523_000001_user_api_keys.rs        # 用户 API key 表
```

### 数据库表

| 表 | 说明 | 关键字段 |
|----|------|---------|
| providers | LLM 提供商 | name, openai_base_url, anthropic_base_url, models |
| accounts | API 账号 | encrypted_key, key_tail (末 6 位), provider_id (FK) |
| users | 管理员用户 | username, password_hash |
| access_points | 接入点 | short_code (唯一), api_type, provider_id, account_id |
| refresh_tokens | JWT 刷新令牌 | user_id (FK), token_hash, expires_at, revoked |
| log_metadata | 代理日志元数据 (按月分区) | session_id, model_original, model_mapped, status_code, duration_ms |
| log_contents | 代理日志内容 | log_id, request_headers, request_body, response_body |
| audit_logs | 操作审计日志 | user_id, action, target_type, target_id, details |
| user_api_keys | 用户 API key (SHA-256 哈希存储) | user_id (FK), key_hash (唯一), key_prefix, description, last_used_at, status, created_at |

**物化视图**: `daily_request_stats` — 按天聚合统计，含请求量、平均耗时、错误数。

**分区策略**: `log_metadata` 表按月 `RANGE (timestamp)` 分区，由应用层 `PartitionManager` 自动管理（创建 / 清理），通过 `pg_try_advisory_xact_lock` 保证多副本安全。

## 代理转发流程

```
客户端请求 (携带 Authorization: Bearer <user_api_key>)
    │
    ▼
POST /ap/{short_code}/v1/messages
    │
    ▼
0. 提取 Authorization 头 → 计算 SHA-256 hex → 查找 UserApiKey (验证 enabled, 更新 last_used_at)
    │
    ▼
1. 提取 short_code → 查找 AccessPoint (验证 enabled)
    │
    ▼
2. 通过 AccessPoint.provider_id → 查找 Provider (验证 enabled)
    │
    ▼
3. 通过 AccessPoint.account_id → 查找 Account (验证 enabled, 解密 API Key)
    │
    ▼
4. 应用模型映射 (替换 JSON 中 model 字段, 同步 Content-Length)
    │
    ▼
5. 构建新的上游请求: 入站 `authorization` 只用于用户 API key 认证, 不参与上游请求构造; 上游请求使用解密后的账号 API key 设置 `Authorization: Bearer <account_key>`, 仅复制 `x-*` 自定义头、`accept`、`content-type` 等业务头, 并排除入站 `authorization` / `x-api-key`
    │
    ▼
6. 发送到上游 LLM API
    ├── 非流式: 完整响应 → 返回
    └── SSE 流式: 逐块转发 → 逐块返回
    │
    ▼
7. 异步写入日志 (log_metadata + log_contents, 不阻塞响应)
```

## 核心架构原则

1. **Domain 层零外部依赖**: 只使用 Rust 标准库 + serde + uuid + chrono + async-trait，不依赖 axum、SeaORM、reqwest 等框架
2. **依赖反转**: Repository trait 在 domain 定义，infrastructure 实现；Application 引用 trait 而非具体实现；`main.rs` 完成依赖组装
3. **领域实体 ≠ ORM 实体**: domain/entities 是纯业务 struct，infrastructure/persistence/entities 是 SeaORM DeriveEntityModel，repository 中手工映射
4. **聚合边界明确**: Provider (根+Account)、User (根+RefreshToken)、AccessPoint (根+跨聚合 UUID 引用)、LogEntry (根+LogContent)
5. **错误隔离**: 数据库错误和加密错误详情不暴露给客户端，统一转换为 `500 Internal Server Error`
6. **同源部署**: 前端构建产物嵌入 Rust 二进制，生产环境前后端同源，无需 CORS 配置

## 安全设计

| 层面 | 措施 |
|------|------|
| API Key 存储 | AES-256-GCM 加密，数据库仅存密文 |
| API Key 展示 | 仅显示末尾 6 位 |
| 密码存储 | argon2id 哈希 (慢哈希算法) |
| 认证令牌 | JWT Access Token (30 分钟) + Refresh Token (7 天) |
| 令牌吊销 | Refresh Token 原子级别 revoked 标记 |
| 错误隔离 | 加密/数据库错误不暴露原始详情 |
| Header 构造 | 上游请求独立构建，入站 `authorization` 只用于用户 API key 认证，provider 认证由账号 API key 单独生成 |
| 传输安全 | 建议部署时配置 HTTPS 反向代理 |

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

Dockerfile 分三阶段构建:

1. **frontend-builder**: Node 22 Alpine — npm ci + npm run build
2. **backend-builder**: Rust 1.89 Alpine — cargo build --release (嵌入前端产物)
3. **runtime**: Alpine 3.21 — 仅包含二进制和运行时依赖

```bash
docker compose up -d    # 启动 PostgreSQL + App
```

### Makefile 任务

| 命令 | 说明 |
|------|------|
| `cargo make dev` | 并行启动前端 Vite HMR + 后端 cargo run |
| `cargo make build` | 顺序构建前端 + 后端 release |
| `cargo make check` | 并行 cargo check + tsc --noEmit |
| `cargo make preview` | build 并运行 release 二进制 |
| `cargo make fmt` | cargo fmt |
| `cargo make clippy` | cargo clippy (deny warnings) |
| `cargo make test` | cargo test |
| `cargo make clean` | cargo clean |

## 项目状态

| 维度 | 状态 |
|------|------|
| Phase 1 MVP | 已完成 |
| 后端 | 96 个 .rs 文件, cargo check 零错误零警告 |
| 前端 | 15 个 .ts/.tsx 源文件, tsc --noEmit 零错误 |
| Schema 迁移 | 初始迁移就绪 (8 表 + 1 物化视图) |
| Docker 构建 | 多阶段构建就绪 |
| 容器编排 | docker-compose.yml 就绪 |

## 变更记录

| 日期 | 变更说明 |
|------|---------|
| 2026-05-19 | 初始化架构文档，记录 DDD 四层架构、代理转发流程、安全设计和项目状态 |
| 2026-05-20 | 应用层分区管理替代 pg_partman：新增 PartitionManager，迁移移除 pg_partman 依赖改为原生分区语法 + 种子分区，Config 新增 3 个分区配置项，main.rs 新增分区初始化和后台定时任务 |
| 2026-05-24 | 调整代理 Header 构造语义：`ProxyClient` 独立构建上游请求，入站 `authorization` 只用于用户 API key 认证，上游 provider 认证由账号 API key 单独生成；同时实现 `decrypt_account_key` 解密逻辑（从 stub 变为完整实现） |

# Token Proxy 架构文档

此工程使用 Rust + axum 框架，在后端采用领域驱动设计四层架构（Domain / Application / Infrastructure / Presentation），前端使用 React + TypeScript + Vite + Semi Design 构建 SPA，数据库使用 PostgreSQL 17，log_metadata 表通过 PostgreSQL 原生分区语法（PARTITION BY RANGE）按月分区，由应用层 PartitionManager 自动管理分区生命周期。

## 目录结构总览

```
├── src/                    # 后端 Rust 核心代码 (97 个 .rs 文件)
├── src-dashboard/          # 前端管理面板 SPA (45 个 .ts/.tsx 源文件)
├── public/                 # 前端静态资源 (favicon, icons)
├── index.html              # 前端 HTML 入口 (Vite)
├── vite.config.ts          # Vite 构建配置
├── tsconfig.json           # TypeScript 根配置
├── tsconfig.app.json       # TypeScript 应用配置
├── tsconfig.node.json      # TypeScript Node 配置
├── eslint.config.js        # ESLint 配置
├── package.json            # 前端依赖声明
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

后端按照**领域驱动设计 (DDD) 四层架构**组织，遵循依赖反转原则。

```
src/
├── domain/                 # 领域层 (按聚合组织)
│   ├── access_point/       # AccessPoint 聚合 (核心编排)
│   ├── provider/           # Provider 聚合 (配置持有者 + 密钥管理)
│   ├── user/               # User 聚合 (认证)
│   ├── log/                # Log 聚合 (只读事件数据)
│   └── shared/             # 跨聚合共享 (Status, ApiKey, AccessPointType, EncryptionService, ApiProtocol)
├── application/            # 应用层 (用例编排, 依赖注入)
│   ├── dto/                # 6 组 DTO
│   ├── services/           # 8 个应用服务
│   └── mod.rs              # AppState 全局共享状态
├── infrastructure/         # 基础设施层
│   ├── persistence/        # Repository 实现 + 分区管理
│   ├── parsers/            # Claude Code 请求头、SSE、消息摘要和 token usage 解析
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
├── main.rs                 # 启动入口 (依赖组装 + 路由构建 + 分区初始化 + 后台定时分区维护 + 过期令牌清理任务)
└── lib.rs                  # Crate 根模块 (模块导出)
```

### 领域层 (domain/)

领域层是整个架构的核心，**使用 SeaORM 宏实现实体定义**（DeriveEntityModel、DeriveActiveEnum、DeriveValueType、FromJsonQueryResult），依赖 sea-orm、serde、uuid、chrono、async-trait。

```
domain/
├── access_point/           # AccessPoint 聚合 (核心编排)
│   ├── access_point.rs     # SeaORM Model + ModelEx 聚合根 + base_url/resolve_model/validate_usable/decrypt_upstream_key 行为方法
│   ├── short_code.rs       # 接入点短码 (生成/校验)
│   ├── model_mapping.rs    # 模型映射对 (source → target, 支持 exact/prefix 匹配) + 常量 + normalize_match_type
│   ├── repository.rs       # AccessPointRepository trait
│   └── mod.rs              # 模块导出 (Model as AccessPoint, ModelEx as AccessPointEx)
├── provider/               # Provider 聚合 (配置持有者 + 密钥管理)
│   ├── provider.rs         # SeaORM Model + base_url_for 方法
│   ├── account.rs          # Account SeaORM Model
│   ├── model_list.rs       # ModelList 值对象
│   ├── repository.rs       # ProviderRepository + AccountRepository traits
│   └── mod.rs
├── user/                   # User 聚合 (认证)
│   ├── user.rs             # User SeaORM Model
│   ├── refresh_token.rs    # RefreshToken (is_expired, is_valid)
│   ├── user_api_key.rs     # UserApiKey (SHA-256 哈希存储)
│   ├── repository.rs       # UserRepository + RefreshTokenRepository + UserApiKeyRepository traits
│   └── mod.rs
├── log/                    # Log 聚合 (只读事件数据)
│   ├── metadata.rs         # LogEntry SeaORM Model (new_proxy_entry)
│   ├── content.rs          # LogContent SeaORM Model
│   ├── token_usage.rs      # LogTokenUsage SeaORM Model
│   ├── audit_log.rs        # AuditLog SeaORM Model
│   ├── repository.rs       # LogRepository + LogTokenUsageRepository + AuditLogRepository traits
│   └── mod.rs              # re-export (LogEntry/LogContent/LogTokenUsage/AuditLog 别名)
├── shared/                 # 跨聚合共享
│   ├── status.rs           # 启用/禁用状态枚举
│   ├── api_key.rs          # API Key (掩码展示)
│   ├── api_type.rs         # AccessPointType 枚举 (Anthropic)
│   ├── encryption.rs       # EncryptionService trait (encrypt/decrypt)
│   ├── api_protocol.rs     # ApiProtocol trait (协议抽象)
│   └── mod.rs
└── mod.rs                  # 领域层入口 (声明 5 个聚合模块 + shared)
```

**领域实体即 ORM 实体**：domain 层直接使用 SeaORM `DeriveEntityModel` 宏定义实体，消除了基础设施层的重复实体和 200+ 行 TryFrom/From 手工映射代码。领域实体附加的行为方法（new、resolve_model、is_expired 等）直接定义在 Model 上。

**聚合根模式**：AccessPoint 的 `ModelEx` (= `AccessPointEx`) 是代理管道的聚合根，包含已加载的 Provider 和 Account 关联。Repository 的 `find_by_short_code` 返回 `AccessPointEx`。ProxyPipeline 仅与该聚合根交互，不再直接引用 Provider/Account 类型。

**聚合边界**：

| 聚合根 | 包含子实体 | 跨聚合引用 |
|--------|-----------|-----------|
| AccessPoint (ModelEx) | ShortCode, ModelMappingCollection | provider_id → Provider, account_id → Account (均在 ModelEx 中加载) |
| Provider | Account, ModelList | - |
| User | RefreshToken, UserApiKey | - |
| LogEntry | LogContent, LogTokenUsage | user_id, access_point_id, provider_id, account_id (Uuid) |

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
│   ├── access_point_service.rs # 接入点管理用例 (含短码生成 + match_type 标准化: 创建/更新时对 __unmatched__ 和 Claude 家族源模型强制设置为 prefix)
│   ├── auth_service.rs     # 认证用例 (登录/刷新/登出)
│   ├── proxy_service.rs    # 核心代理转发用例 (含 Bearer API key 认证)
│   └── log_service.rs      # 日志写入 / 查询用例 (metadata、content、conversation events、token usage 编排)
└── mod.rs                  # AppState 定义 (所有 Service 的引用容器)
```

**AppState** 是全局共享状态，通过 axum 的 `with_state()` 注入到所有路由处理器，包含 Config、数据库连接、所有 Service 引用、JWT 服务和代理客户端。

### 基础设施层 (infrastructure/)

基础设施层实现 domain 层定义的接口，处理所有外部依赖。

```
infrastructure/
├── persistence/            # SeaORM 数据持久化
│   ├── partition_manager.rs # PartitionManager: 应用层分区自动管理
│   └── repositories/       # Repository 实现 (7 个, 含 refresh token 过期清理 delete_expired)
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
│   ├── jwt.rs              # JwtService (jsonwebtoken, 含 refresh_expiry_secs 访问器供 AuthService 正确计算 refresh_token 过期时间)
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
│   ├── components/                 # 通用组件
│   │   ├── ThemeToggle.tsx          # 主题切换 (light/dark/system)
│   │   ├── AccessPointDrawer.tsx    # 接入点创建/编辑表单 (含 api_type、Provider 选择并显示默认模型; 将 Provider.models + Provider.default_model 传递给 ModelMappingEditor 作为目标模型候选列表)
│   │   ├── AccessPointTable.tsx     # 接入点列表表格
│   │   ├── ModelMappingEditor.tsx   # 模型映射编辑器 (源模型 Select 用 Semi Tag 前缀显示"精准匹配/模式匹配", 预设含 __unmatched__(prefix) 和 Claude 家族(prefix), 支持 allowCreate 自定义; 目标模型 Select 包含 Provider.models + Provider.default_model + 附加的"默认模型"选项 (DEFAULT_MODEL 哨兵值), 禁止创建; 导出 matchTypeForSource 和 UNMATCHED_MODEL 供外部使用)
│   │   ├── StatusToggle.tsx         # 状态切换开关
│   │   ├── StatCard.tsx             # 统计卡片
│   │   ├── TrendChart.tsx           # 趋势图表
│   │   ├── LogFilterBar.tsx         # 日志过滤栏
│   │   ├── SessionInfoHeader.tsx    # 会话信息头部
│   │   ├── LogDetailModal.tsx       # 日志详情弹窗
│   │   ├── RawContentModal.tsx      # 原始内容查看弹窗
│   │   ├── ChatBubbleView.tsx       # 聊天气泡视图
│   │   ├── RequestLogTable.tsx      # 请求日志表格 (列定义 + Table 渲染, 从 RequestLogPage 提取)
│   │   ├── SessionListView.tsx      # 会话列表视图 (过滤栏 + 分页表格, 从 SessionLogPage 提取)
│   │   ├── SessionDetailView.tsx    # 会话详情视图 (事件流 + 事件摘要表格 + 原始内容弹窗, 从 SessionLogPage 提取)
│   │   ├── CopyableIdText.tsx       # 可复制 ID 文本组件 (等宽字体 + 点击复制)
│   │   ├── BasicInfoCard.tsx        # 基础信息卡片
│   │   ├── CodeHighlight.tsx        # 代码高亮组件
│   │   ├── CollapsibleCard.tsx      # 可折叠卡片
│   │   ├── MarkdownRender.tsx       # Markdown 渲染组件
│   │   ├── RawResponseView.tsx      # 原始响应查看组件
│   │   ├── RequestContentCard.tsx   # 请求内容卡片
│   │   ├── RequestHeadersCard.tsx   # 请求头卡片
│   │   ├── ResponseContentCard.tsx  # 响应内容卡片
│   │   ├── TokenUsageCard.tsx       # Token 用量卡片
│   │   ├── charts/                  # 图表组件子目录
│   │   ├── log-viewer/              # 日志查看器组件子目录
│   │   └── timeline/                # 时间线组件子目录
│   ├── hooks/                      # 自定义 hooks
│   │   ├── useTheme.ts             # 主题管理 (ThemeProvider + useTheme, 三种模式)
│   │   └── useAccessPoints.ts      # 接入点数据管理 (Provider/Account 加载; 创建/编辑时过滤 target_model 不在 Provider.models + Provider.default_model + DEFAULT_MODEL 哨兵的映射; 删除/切换状态/复制 URL)
│   ├── layouts/
│   │   └── AdminLayout.tsx         # 管理界面布局 (Semi Design Navigation)
│   ├── pages/
│   │   ├── LoginPage.tsx           # POST /api/auth/login
│   │   ├── DashboardPage.tsx       # 仪表盘概览
│   │   ├── ProviderManagement.tsx  # CRUD /api/providers (表格 default_model 列使用 Tag 渲染; 编辑面板模型列表 TagInput + 下方独立 default_model Select; models 为空时禁用选择; TagInput 移除模型联动清空 default_model; 保存时若 default_model 不在 models 中则自动清空)
│   │   ├── AccessPointManagement.tsx # CRUD /api/access-points (Provider 切换时, 创建态下若有 default_model 则自动生成 __unmatched__(prefix) → __default_model__ 哨兵映射; 保存委托 useAccessPoints hook 过滤无效映射)
│   │   ├── UserManagement.tsx      # CRUD /api/users
│   │   ├── ProfilePage.tsx         # 个人设置 (profile/密码/API key 管理)
│   │   ├── SessionLogPage.tsx      # 会话日志路由壳 (根据 URL 中 sessionId 参数切换列表/详情视图: 无 sessionId 渲染 SessionListView, 有 sessionId 渲染 SessionDetailView)
│   │   ├── RequestLogPage.tsx      # GET /api/logs (数据加载 + 过滤 + 委托 RequestLogTable 渲染表格)
│   │   ├── LogDetailPage.tsx       # GET /api/logs/:id (单条日志详情, 含请求/响应内容展示)
│   │   └── SettingsPage.tsx        # 设置页面
│   ├── types/                      # TypeScript 类型定义
│   └── utils/                      # 工具函数
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
  /dashboard            → DashboardPage
  /providers/*          → ProviderManagement
  /access-points        → AccessPointManagement
  /sessions             → SessionLogPage
  /sessions/:sessionId  → SessionLogPage (单会话详情)
  /logs                 → RequestLogPage
  /logs/:id             → LogDetailPage (单条日志详情)
  /users                → UserManagement
  /settings             → SettingsPage
	  /settings/profile     → ProfilePage (个人设置)
```

### API 通信层

`api.ts` 封装了基于 fetch 的 HTTP 客户端，自动附加 JWT `Authorization` 头。采用「双层防御」策略处理令牌过期：请求前检查 Access Token 是否接近过期，必要时通过 Refresh Token 静默刷新；若刷新失败或 401 响应仍到达，则清除所有本地令牌并跳转登录页。模块级 `refreshing` Promise 实现并发刷新去重，避免 Refresh Token Rotation 模式下多请求互相吊销。提供 `get`、`post`、`put`、`delete` 四个方法。

### 主题系统

前端支持 light / dark / system 三种主题模式，通过 `useTheme.ts` hook 管理。系统主题自动跟随 `prefers-color-scheme` 媒体查询。`ThemeProvider` 在根组件包装，通过 `document.body` 的 `theme-mode` 属性控制 Semi Design 暗色模式切换。`ThemeToggle` 组件位于管理面板侧边栏和登录页。

## 数据库架构详解 (migration/)

迁移使用独立的 workspace crate (`migration/`)，基于 `sea-orm-migration`。

```
migration/
├── Cargo.toml        # 依赖: sea-orm-migration
└── src/
    ├── lib.rs
    ├── m20260101_000001_initial.rs              # 初始 Schema (8 个表 + 物化视图)
    ├── m20260523_000001_user_api_keys.rs        # 用户 API key 表
    └── m20260524_000001_provider_default_model.rs # providers 表增加 default_model 列
```

### 数据库表

| 表 | 说明 | 关键字段 |
|----|------|---------|
| providers | LLM 提供商 | name, openai_base_url, anthropic_base_url, models, default_model |
| accounts | API 账号 | encrypted_key, key_tail (末 6 位), provider_id (FK) |
| users | 管理员用户 | username, password_hash |
| access_points | 接入点 | short_code (唯一), api_type, provider_id, account_id |
| refresh_tokens | JWT 刷新令牌 | user_id (FK), token_hash, expires_at, revoked; 过期记录由 tokio 后台任务每小时物理清理 |
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
1. find_by_short_code(short_code) → AccessPointEx (含已加载的 Provider 和 Account)
    │
    ▼
2. ap_ex.validate_usable() → 校验自身 + Provider + Account 三个状态
    │
    ▼
3. ap_ex.base_url() → 通过 api_type 从 Provider 获取上游 URL
    │
    ▼
4. ap_ex.decrypt_upstream_key(encryption_svc) → 解密 Account API Key
    │
    ▼
5. ap_ex.resolve_model(requested_model) → 模型映射 (精确 > 前缀 > __unmatched__) + Provider.default_model 兜底 → 替换请求体中的 model 字段
    │
    ▼
6. 构建新的上游请求 → 发送到上游 LLM API
    ├── 非流式: 完整响应 → 返回
    └── SSE 流式: 逐块转发 → 逐块返回
    │
    ▼
7. 异步写入日志 (log_metadata + log_contents + log_conversation_events + log_token_usage, 不阻塞响应)
```

## 核心架构原则

1. **Domain 层使用 SeaORM 宏定义实体**: 领域实体通过 DeriveEntityModel、DeriveActiveEnum、DeriveValueType、FromJsonQueryResult 等 SeaORM 宏定义，与基础设施层共用类型系统。消除 200+ 行 TryFrom/From 手动映射代码，但 domain 代码理论上可调用 SeaORM query API，需通过 code review 约束
2. **依赖反转**: Repository trait 在 domain 定义，infrastructure 实现；Application 引用 trait 而非具体实现；`main.rs` 完成依赖组装
3. **领域实体即 ORM 实体**: domain/entities 直接使用 SeaORM DeriveEntityModel 宏，既是领域模型也是数据库映射。行为方法（new、resolve_model、is_expired 等）直接附加在 Model 上
4. **聚合边界明确**: Provider (根+Account)、User (根+RefreshToken)、AccessPoint (根+跨聚合 UUID 引用)、LogEntry (根+LogContent)
5. **错误隔离**: 数据库错误和加密错误详情不暴露给客户端，统一转换为 `500 Internal Server Error`
6. **同源部署**: 前端构建产物嵌入 Rust 二进制，生产环境前后端同源，无需 CORS 配置
7. **依赖最小化原则**: 优先复用现有基础设施（PostgreSQL、tokio）解决问题，引入新中间件需要明确的多个使用场景作为合理性论证
8. **双层防御模式**: 前端请求前体检 + 401 兜底双层保障令牌有效性，适配浏览器后台冻结节流策略，不依赖定时器
9. **依赖倒置在认证场景的体现**: RefreshTokenRepository trait 隔离存储实现，切换 Redis 等存储时无需修改 AuthService

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
| JWT 自动刷新 | 前端「双层防御」: 请求前体检 + 401 兜底，模块级 Promise 并发去重 |
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
| 后端 | 97 个 .rs 文件, cargo check 零错误零警告 |
| 前端 | 45 个 .ts/.tsx 源文件, tsc --noEmit 零错误 |
| Schema 迁移 | 3 个迁移文件 (初始表 + user_api_keys + provider_default_model) |
| Docker 构建 | 多阶段构建就绪 |
| 容器编排 | docker-compose.yml 就绪 |

## 变更记录

| 日期 | 变更说明 |
|------|---------|
| 2026-05-19 | 初始化架构文档，记录 DDD 四层架构、代理转发流程、安全设计和项目状态 |
| 2026-05-20 | 应用层分区管理替代 pg_partman：新增 PartitionManager，迁移移除 pg_partman 依赖改为原生分区语法 + 种子分区，Config 新增 3 个分区配置项，main.rs 新增分区初始化和后台定时任务 |
| 2026-05-24 | 调整代理 Header 构造语义：`ProxyClient` 独立构建上游请求，入站 `authorization` 只用于用户 API key 认证，上游 provider 认证由账号 API key 单独生成；同时实现 `decrypt_account_key` 解密逻辑（从 stub 变为完整实现） |
| 2026-05-24 | Provider 增加 `default_model` 字段（全链路：domain entity、SeaORM entity、DTO、service、migration）；CreateAccessPointRequest 支持 `api_type` 参数（当前有效类型为 Anthropic）；ModelMapping 增加 `MatchType`（exact/prefix）和常量（`UNMATCHED_MODEL_SENTINEL`、Claude 模型族前缀）；实现统一模型匹配逻辑（精确 > 前缀 > `__unmatched__` > Provider.default_model），代理路由使用统一匹配并记录最终 `model_mapped` |
| 2026-05-24 | 前端新增主题切换系统：`useTheme` hook（light/dark/system 三种模式）、`ThemeToggle` 组件、`ThemeProvider` 包裹根组件；接入点表单新增 `api_type` 选择器和 `ModelMappingEditor`（支持 Anthropic 模型族 Opus/Sonnet/Haiku 快捷添加前缀匹配规则） |
| 2026-05-24 | 前端 Provider 表格 default_model 列使用 Tag 渲染; Provider 编辑面板 default_model Select 移至模型列表 TagInput 下方, TagInput 移除模型联动清空 default_model; ModelMappingEditor 源模型下拉展示匹配类型说明, 目标模型下拉仅含 Provider 已注册 models 且禁止创建; 保存时过滤 target_model 不在 Provider.models 的映射 (useAccessPoints hook 实现) |
| 2026-05-24 | 同步架构文档与实际代码：`__unmatched__` 视为模式匹配, 自动生成的未匹配规则使用 prefix; Select 选项用 Semi Tag 前缀显示"精准匹配/模式匹配"; 目标模型 Select 包含 Provider.models + Provider.default_model; 保存过滤也允许 Provider.default_model |
| 2026-05-24 | 服务端强化匹配类型: 新增 `normalize_match_type` 和 `is_prefix_source_model` 函数, 强制 `__unmatched__` 和 Claude 家族前缀 (claude-opus-/claude-sonnet-/claude-haiku-) 始终视为 `prefix` 匹配; AccessPointService 创建/更新时执行 match_type 标准化; 前端 ModelMappingEditor 对 apiType 做大小写兼容 |
| 2026-05-29 | 实体合并改造: 将 SeaORM DeriveEntityModel 从 `infrastructure/persistence/entities/` 迁移到 `domain/entities/`，删除基础设施层 entities 目录。domain 层引入 SeaORM 宏依赖，消除 200+ 行 TryFrom/From 手工映射代码。领域实体即 ORM 实体，不再区分 |
| 2026-05-27 | 前端组件架构拆分: 从 RequestLogPage 提取 `RequestLogTable` 组件 (表格列定义 + Table 渲染); 从 SessionLogPage 提取 `SessionListView` (会话列表视图) 和 `SessionDetailView` (会话详情视图); SessionLogPage 瘦身为路由壳, 根据 sessionId 参数切换列表/详情视图; 新增 `/logs/:id` 路由和 `LogDetailPage` 页面; 前端源文件数更新为 45 个 |
| 2026-05-26 | 认证体系优化: 前端 `api.ts` 采用「双层防御」策略（请求前体检 + 401 兜底），模块级 Promise 并发去重，解决浏览器冻结导致定时器失效问题；`JwtService` 新增 `refresh_expiry_secs` 访问器，修复 AuthService 两处误用 access 寿命写入 refresh_token expires_at 的 bug；新增 tokio 后台任务每小时物理清理过期 refresh_token，明确拒绝引入 Redis 或 pg_cron，遵循依赖最小化原则；新增架构原则 7-9（依赖最小化、双层防御、依赖倒置认证场景体现） |
| 2026-06-03 | 领域层聚合重构: 将 domain/ 层从按技术类别（entities/value_objects/repositories/services）重组为按聚合边界（access_point/provider/user/log/shared）组织。AccessPoint 引入 ModelEx 聚合根，Repository 的 `find_by_short_code` 返回已加载 Provider 和 Account 关联的完整聚合。ProxyPipeline 删除 `select_base_url` 和 `decrypt_account_key` 方法，全部操作委托 AccessPointEx 行为方法（base_url、resolve_model、validate_usable、decrypt_upstream_key）。Provider 新增 `base_url_for` 方法。AccessPointType 移至 shared 解决循环依赖。account_id 退化为纯 FK 列（不定义 belongs_to 关系）。Relation 定义保持 DeriveRelation 枚举语法（SeaORM 2.0-rc.38 兼容性） |

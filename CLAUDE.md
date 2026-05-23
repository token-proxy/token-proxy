# Token Proxy

企业级 LLM API 资源管理平台，提供统一的 API 代理、密钥管理、用量监控和访问控制能力。

## 技术栈

- **后端**: Rust (edition 2021) + axum 0.8 + SeaORM 1 + tokio
- **前端**: React 19 + TypeScript 6 + Vite 8 + Semi Design 2.97
- **数据库**: PostgreSQL 17 (应用层按月分区管理)
- **构建**: cargo-make (任务编排) + Docker (多阶段构建)
- **路由**: react-router-dom v7

## 架构: DDD 四层

```
src/
├── domain/              # 领域层 (零外部框架依赖)
│   ├── entities/        # Provider, Account, User, AccessPoint, RefreshToken, LogEntry, UserApiKey
│   ├── value_objects/   # ShortCode, ApiKey, ModelMapping, Status, AccessPointType
│   ├── repositories/    # Repository traits (接口定义)
│   └── services/        # EncryptionService trait, ModelMappingService
├── application/         # 应用层 (用例编排)
│   ├── dto/             # 请求/响应 DTO (9 组)
│   ├── services/        # 8 个应用服务 (依赖注入 domain traits)
│   └── AppState         # 全局共享状态
├── infrastructure/      # 基础设施层
│   ├── persistence/     # SeaORM 实体 (10 个) + Repository 实现 (8 个) + PartitionManager
│   ├── encryption/      # AES-256-GCM 加密
│   ├── auth/            # JWT (jsonwebtoken) + argon2 密码哈希
│   └── http_client/     # reqwest 代理转发客户端
├── presentation/        # 展示层
│   ├── routes/          # 9 组 axum handlers
│   └── middleware/      # JWT 认证中间件
└── shared/              # 共享
    ├── error.rs         # AppError (9 种变体)
    └── types.rs         # PaginatedResult, PaginationParams
```

## 项目状态

- Phase 1 MVP 已完成
- 后端: cargo check 零错误零警告
- 前端: tsc --noEmit 零错误
- 构建: Makefile.toml (dev/build/preview/check/fmt/clippy/test)
- 部署: Dockerfile (多阶段: Node 22 构建前端 -> Rust 1.89 构建后端 -> Alpine 3.21 运行时)
- docker-compose.yml (pgvector/pgvector:pg17 + app)

## 数据库 Schema (9 个核心表)

| 表 | 说明 |
|---|---|
| providers | LLM 提供商 (OpenAI/Anthropic 等) |
| accounts | API 账号 (AES-256-GCM 加密存储 Key) |
| users | 管理员用户 (argon2id 密码哈希) |
| user_api_keys | 用户 API key (SHA-256 哈希存储, 支持吊销) |
| access_points | 接入点 (短码、模型映射) |
| refresh_tokens | JWT 刷新令牌 |
| log_metadata | 代理日志元数据 (PartitionManager 按月分区) |
| log_contents | 代理日志内容 (JSON 请求/响应) |
| audit_logs | 操作审计日志 |

- 物化视图: `daily_request_stats` (按天聚合统计, 含请求量/平均耗时/错误数)

## 关键决策

- **接入 URL**: `/ap/<short_code>` -- 用户指定或自动生成
- **JWT**: Access Token 30min + Refresh Token 7day
- **加密**: AES-256-GCM (ENCRYPTION_KEY 环境变量 64 hex chars = 32 字节)
- **密码**: argon2id
- **分区**: PartitionManager 应用层管理, 按月 `RANGE (timestamp)`, 依赖原生 PostgreSQL 分区, 支持多副本 advisory lock 防冲突
- **代理**: SSE 流式逐块转发 + 异步日志写入
- **路由**: 公开路径 (`/api/auth/*`, `/ap/*`, `/api/health`) 跳过 JWT 认证
- **接入点认证**: `/ap/*` 路径跳过 JWT 中间件, 但在 ProxyService 中强制验证用户 API key (`Authorization: Bearer <user_api_key>`), 通过 SHA-256 哈希匹配后记录 user_id
- **用户 API key**: 个人设置页管理, key 以 `tp_` 为前缀, 生成 40 位随机字符, 数据库仅存储 SHA-256 哈希和前缀, 完整 key 只创建时返回一次; 支持吊销操作

## Makefile 任务

| 命令 | 说明 |
|---|---|
| `cargo make dev` | 并行启动前端 Vite HMR + 后端 cargo run |
| `cargo make build` | 顺序构建前端 npm build + 后端 cargo build --release |
| `cargo make check` | 并行执行 cargo check + tsc --noEmit |
| `cargo make preview` | build 并运行 release 二进制 |
| `cargo make fmt` | cargo fmt |
| `cargo make clippy` | clippy (deny warnings) |
| `cargo make test` | cargo test |
| `cargo make clean` | cargo clean |

## 环境变量

| 变量 | 说明 | 默认值 |
|---|---|---|
| DATABASE_URL | PostgreSQL 连接串 | -- (必填) |
| JWT_SECRET | JWT 签名密钥 | -- (必填) |
| ENCRYPTION_KEY | 64 hex chars (32 字节) | -- (必填) |
| SERVER_PORT | 监听端口 | 3000 |
| LOG_LEVEL | 日志级别 | info |
| PARTITION_CHECK_INTERVAL_SECS | 分区检查间隔 (秒) | 3600 |
| PARTITION_PREMAKE_MONTHS | 提前创建未来分区月数 | 3 |
| PARTITION_RETENTION_MONTHS | 分区保留月数 | 12 |

## 代码规范

### 通用

- **汉字与数字/字母/英文符号之间必须保留一个空格**
- 如: `服务监听地址: {}`, `接入点 '{}' 未找到`
- 错误消息使用中文, 技术标识符/日志字段使用英文
- 后端: 67 个 Rust 源文件, 遵循 Rust 2021 edition 惯例
- 前端: 15 个 TypeScript 源文件, 遵循 TypeScript 6 严格模式

### 后端 (Rust)

- 模块按 DDD 分层组织, `mod.rs` 只做 re-export
- `AppError` 9 种变体: Validation, NotFound, Conflict, Unauthorized, Forbidden, Encryption, Database, Upstream, Internal
- 使用 `Arc<dyn Trait>` 依赖注入, 在 `main.rs` 组装
- 应用层 Service 构造时注入 Repository traits, 不直接依赖 SeaORM

### 前端 (React + Semi Design)

- 页面组件集中在 `frontend/src/pages/`
- 使用 `@douyinfe/semi-ui` 组件库
- 路由: react-router-dom v7 (BrowserRouter + Routes + AdminLayout)
- 路由结构: `/login`, `/dashboard`, `/providers`, `/access-points`, `/sessions`, `/logs`, `/users`, `/settings`, `/settings/profile`
- 后端通信: `frontend/src/api.ts` (axios/fetch 封装)
- **防重复点击**: 所有触发 API 调用或异步操作的按钮必须设置 `loading`/`disabled` 状态, 操作完成后才解除锁定。管理列表页使用 `operatingId` 实现行级按钮独立锁定
- **改密自动登出**: 修改密码操作成功后, 前端必须清除所有 localStorage 令牌 (`access_token`, `refresh_token`, `username`, `display_name`) 并跳转 `/login`, 强制用户重新认证

## 注意事项

- `.rs` 空文件留作占位用, 不应删除
- 前端构建产物 (`frontend/dist/`) 会被嵌入后端二进制
- 所有 Repository 实现以 `SeaOrm` 为前缀 (如 `SeaOrmProviderRepository`)
- 迁移文件在 `src/migrations/` 目录下, 使用 `sea-orm-migration`
- 分区管理由 `src/infrastructure/persistence/partition_manager.rs` 的 PartitionManager 处理, 通过 pg_inherits 系统表管理分区
- `log_metadata` 分区表的 `PRIMARY KEY` 必须包含 `timestamp`

## 核心文件路径

| 文件 | 说明 |
|---|---|
| `src/main.rs` | 启动入口 (依赖组装 + Router 构建 + 分区管理器初始化) |
| `src/lib.rs` | Crate 根模块 |
| `src/config.rs` | 环境变量配置加载 |
| `src/application/mod.rs` | AppState 定义 |
| `src/shared/error.rs` | AppError 错误类型 |
| `src/application/services/proxy_service.rs` | 核心代理转发引擎 (含用户 API key 认证) |
| `src/application/services/user_api_key_service.rs` | 用户 API key 管理服务 |
| `src/presentation/routes/me_routes.rs` | 个人设置路由 (`/api/users/me/*`) |
| `src/migrations/m20260101_000001_initial.rs` | 初始数据库 Schema |
| `src/migrations/m20260523_000001_user_api_keys.rs` | 用户 API key 表迁移 |
| `src/presentation/routes/mod.rs` | 路由聚合 |
| `src/presentation/middleware/jwt_auth.rs` | JWT 认证中间件 + CurrentUser extractor |
| `src/infrastructure/persistence/partition_manager.rs` | 分区管理器 |
| `src/infrastructure/persistence/repositories/user_api_key_repository.rs` | UserApiKey 仓储实现 |
| `src/domain/entities/user_api_key.rs` | 用户 API key 领域实体 |
| `src/domain/repositories/user_api_key_repository.rs` | UserApiKey Repository trait |
| `frontend/src/App.tsx` | 前端路由定义 |
| `frontend/src/pages/ProfilePage.tsx` | 个人设置页 (个人资料/改密/API key 管理) |
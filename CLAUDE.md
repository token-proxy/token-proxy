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
- 特性亮点: Provider.default_model (从 models 列表选择)、模型映射 MatchType (exact/prefix)、统一模型匹配优先级、AccessPoint.api_type、DEFAULT_MODEL_SENTINEL 哨兵值

## 数据库 Schema (9 个核心表)

| 表 | 说明 |
|---|---|
| providers | LLM 提供商 (含 default_model 字段) |
| accounts | API 账号 (AES-256-GCM 加密存储 Key) |
| users | 管理员用户 (argon2id 密码哈希) |
| user_api_keys | 用户 API key (SHA-256 哈希存储, 支持吊销) |
| access_points | 接入点 (短码、api_type、模型映射、跨聚合引用 Provider+Account) |
| refresh_tokens | JWT 刷新令牌 |
| log_metadata | 代理日志元数据 (PartitionManager 按月分区) |
| log_contents | 代理日志内容 (JSON 请求/响应) |
| audit_logs | 操作审计日志 |

- 物化视图: `daily_request_stats` (按天聚合统计, 含请求量/平均耗时/错误数)
- 迁移: 3 个文件 (初始 Schema + user_api_keys 表 + provider_default_model 列)

## 关键决策

- **接入 URL**: `/ap/<short_code>` -- 用户指定或自动生成
- **JWT**: Access Token 30min + Refresh Token 7day
- **加密**: AES-256-GCM (ENCRYPTION_KEY 环境变量 64 hex chars = 32 字节)
- **密码**: argon2id
- **分区**: PartitionManager 应用层管理, 按月 `RANGE (timestamp)`, 依赖原生 PostgreSQL 分区, 支持多副本 advisory lock 防冲突
- **代理**: SSE 流式逐块转发 + 异步日志写入
- **路由**: 公开路径 (`/api/auth/*`, `/ap/*`, `/api/health`) 跳过 JWT 认证
- **接入点认证**: `/ap/*` 路径跳过 JWT 中间件, 但在 ProxyService 中强制验证用户 API key (`Authorization: Bearer <user_api_key>`), 通过 SHA-256 哈希匹配后记录 user_id
- **代理 Header 构造**: `ProxyClient` 构建新的上游请求时, 入站 `authorization` 只用于用户 API key 认证, 不参与上游请求构造; 上游请求使用解密后的账号 API key 设置 `Authorization: Bearer <account_key>`; 仅复制 `x-*` 自定义头、`accept`、`content-type` 等业务头, 并排除入站 `authorization` / `x-api-key`
- **用户 API key**: 个人设置页管理, key 以 `tp_` 为前缀, 生成 40 位随机字符, 数据库仅存储 SHA-256 哈希和前缀, 完整 key 只创建时返回一次; 支持吊销操作
- **模型映射匹配方式**: 由源模型值决定——`__unmatched__` 哨兵和 Claude 家族预设 (`claude-opus-*`/`claude-sonnet-*`/`claude-haiku-*`) 自动使用 `prefix` 匹配; 普通值或自定义输入使用 `exact` 匹配
- **模型映射优先级**: 请求模型按以下顺序匹配——精确匹配 > 前缀匹配 > `__unmatched__` 哨兵规则 (target 为 `__default_model__` 时通过 `resolve_final_model` 解析为 Provider 当前 default_model) > Provider.default_model 兜底 > 原始请求模型
- **Claude 模型族前缀常量**: `claude-opus-` / `claude-sonnet-` / `claude-haiku-`, 在前端 ModelMappingEditor 中以预设选项形式展示
- **Provider.default_model**: 可选配置, 当模型映射均未匹配时作为兜底, 替换请求体中的 model 字段。表格中使用 Tag 展示; 编辑表单中 Select 位于模型列表下方, 紧邻 TagInput; 模型列表为空时 Select 禁用; 模型列表移除当前默认模型时立即清空 Select 值; 保存时若 models 不含 default_model 则提交空字符串清空
- **AccessPoint.api_type**: 接入点 API 类型枚举, 当前有效类型为 `Anthropic`; 前端表单限定 Select 选项
- **DEFAULT_MODEL_SENTINEL**: 模型映射目标模型哨兵值 `__default_model__`, 前后端常量同名但独立定义 (后端 `DEFAULT_MODEL_SENTINEL`, 前端 `DEFAULT_MODEL`)。当映射的 target_model 为该哨兵值时, 代理转发通过 `resolve_final_model` 函数动态解析为 Provider 当前 `default_model`。`__unmatched__ -> 默认模型` 自动规则的 target_model 保存为 `__default_model__`, 接入点映射无需跟随 Provider 默认模型变更

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
- 后端: 97 个 Rust 源文件, 遵循 Rust 2021 edition 惯例
- 前端: 32 个 TypeScript 源文件, 遵循 TypeScript 6 严格模式

### 后端 (Rust)

- 模块按 DDD 分层组织, `mod.rs` 只做 re-export
- `AppError` 9 种变体: Validation, NotFound, Conflict, Unauthorized, Forbidden, Encryption, Database, Upstream, Internal
- 使用 `Arc<dyn Trait>` 依赖注入, 在 `main.rs` 组装
- 应用层 Service 构造时注入 Repository traits, 不直接依赖 SeaORM

### 前端 (React + Semi Design)

- 页面组件集中在 `src-dashboard/pages/`
- 使用 `@douyinfe/semi-ui` 组件库
- 路由: react-router-dom v7 (BrowserRouter + Routes + AdminLayout)
- 路由结构: `/login`, `/dashboard`, `/providers`, `/access-points`, `/sessions`, `/logs`, `/users`, `/settings`, `/settings/profile`
- 后端通信: `src-dashboard/api.ts` (axios/fetch 封装)
- **防重复点击**: 所有触发 API 调用或异步操作的按钮必须设置 `loading`/`disabled` 状态, 操作完成后才解除锁定。管理列表页使用 `operatingId` 实现行级按钮独立锁定
- **Modal 表单提交**: 包含 `Form` 的 `Modal` 必须使用 `footer` 承载取消 / 确认按钮, 不要把操作按钮放在 `Form` 内容区; `footer` 中的确认按钮必须通过 `getFormApi` 保存的 `formApi.submitForm()` 触发表单提交, 并设置 `loading`/`disabled` 防重复触发
- **改密自动登出**: 修改密码操作成功后, 前端必须清除所有 localStorage 令牌 (`access_token`, `refresh_token`, `username`, `display_name`) 并跳转 `/login`, 强制用户重新认证
- **主题切换**: 使用 `useTheme` hook + `ThemeProvider` (src-dashboard/hooks/useTheme.ts) 管理主题状态, 通过 localStorage key `theme_mode` 持久化, 支持 light / dark / system 三种模式; `ThemeToggle` 组件以 Dropdown 菜单形式展示在 AdminLayout 顶栏
- **ModelMappingEditor**: 接入点表单中的模型映射编辑器, 仅保留一个"添加映射"按钮。源模型使用 Semi Select, 支持搜索和 allowCreate; 选项以 Semi Tag 前缀显示匹配类型 (`精准匹配`/`模式匹配`); 源模型选项包括: `__unmatched__`(模式匹配)、Claude Opus/Sonnet/Haiku 预设(模式匹配)、以及 Provider 的 models 列表(精准匹配)。`__unmatched__` 和 Claude 家族源模型保存为 `prefix` 匹配类型, 普通值/自定义输入值保存为 `exact` 匹配类型。目标模型使用 Semi Select, 选项包含 Provider.models 和 `__default_model__` 哨兵 (展示为"默认模型"), 不允许 allowCreate; 当源模型为 `__unmatched__` 时, 目标模型自动填充为 `__default_model__`
- **AccessPointDrawer 映射管理**: 接入点创建时选择 Provider 后, 自动预填一条 `__unmatched__ -> __default_model__` 的映射规则。保存接入点时过滤映射, target_model 必须属于 Provider.models 或等于 `__default_model__` 哨兵

## 注意事项

- `.rs` 空文件留作占位用, 不应删除
- 前端构建产物 (`dist/`) 会被嵌入后端二进制
- 所有 Repository 实现以 `SeaOrm` 为前缀 (如 `SeaOrmProviderRepository`)
- 迁移文件在 `src/migrations/` 目录下, 使用 `sea-orm-migration`
- 分区管理由 `src/infrastructure/persistence/partition_manager.rs` 的 PartitionManager 处理, 通过 pg_inherits 系统表管理分区
- `log_metadata` 分区表的 `PRIMARY KEY` 必须包含 `timestamp`
- `__unmatched__` 哨兵是模型映射中的特殊 source_model, 使用 `prefix` 匹配类型, 用于为所有未精确/前缀匹配的请求模型指定目标模型, 每个接入点最多一个。接入点创建时, Drawer 自动预填一条 `__unmatched__ -> __default_model__` 映射。保存接入点时过滤映射, target_model 必须属于 Provider.models 或等于 `__default_model__` 哨兵
- `api_type` 枚举的实际范围在 `AccessPointType` 值对象中定义, 新增类型需要同步修改 Rust 枚举 + 数据库列约束 + 前端 Select 选项

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
| `src/domain/entities/provider.rs` | Provider 实体 (含 default_model) |
| `src/domain/value_objects/model_mapping.rs` | ModelMapping + MatchType + 模型族前缀常量 |
| `src/domain/value_objects/access_point_type.rs` | AccessPointType 枚举 (Anthropic) |
| `src/domain/services/model_mapping_service.rs` | 模型匹配领域服务 (精确 > 前缀 > __unmatched__ > default_model) |
| `src/domain/repositories/user_api_key_repository.rs` | UserApiKey Repository trait |
| `src/migrations/m20260524_000001_provider_default_model.rs` | providers 表增加 default_model 列 |
| `src-dashboard/App.tsx` | 前端路由定义 |
| `src-dashboard/pages/ProfilePage.tsx` | 个人设置页 (个人资料/改密/API key 管理) |
| `src-dashboard/pages/ProviderManagement.tsx` | Provider 管理页 (表格中 default_model 用 Tag 展示; 表单中 Select 位于模型列表下方; 模型列表移除当前默认模型时立即清空) |
| `src-dashboard/components/ModelMappingEditor.tsx` | 模型映射编辑器 (单个添加按钮, 源 Select + search + allowCreate + label 显示匹配类型, 含未匹配/Claude 预设/Provider 模型, Claude 预设存 prefix 其余存 exact; 目标 Select 仅含 Provider models, 不允许 allowCreate) |
| `src-dashboard/components/AccessPointDrawer.tsx` | 接入点创建/编辑抽屉 (选择 Provider 后有 default_model 则自动预填 `__unmatched__ -> default_model` 映射; 保存时过滤 target_model 不在 Provider.models 的映射) |
| `src-dashboard/components/ThemeToggle.tsx` | 主题切换组件 (light/dark/system) |
| `src-dashboard/hooks/useTheme.ts` | 主题 Hook + ThemeProvider |
| `src-dashboard/hooks/useAccessPoints.ts` | 接入点管理 Hook (含 api_type 传递) |
| `src-dashboard/types/accessPoint.ts` | 接入点类型定义 (含 api_type, ModelMapping) |
# Token Proxy

企业级 LLM API 资源管理平台，提供统一的 API 代理、密钥管理、用量监控和访问控制能力。

## 技术栈

- **后端**: Rust (edition 2021) + axum 0.8 + SeaORM 2 + tokio
- **前端**: React 19 + TypeScript 6 + Vite 8 + Semi Design 2.97
- **数据库**: PostgreSQL 17 (应用层按月分区管理)
- **构建**: cargo-make (任务编排) + Docker (多阶段构建)
- **路由**: react-router-dom v7

## 架构: DDD 四层

```
src/
├── domain/              # 领域层 (零外部框架依赖，按聚合组织)
│   ├── access_point/    # AccessPoint 聚合 (接入点 + ShortCode + ModelMapping + AccessPointType + Repository trait)
│   ├── provider/        # Provider 聚合 (Provider + Account + ModelList + Repository traits)
│   ├── user/            # User 聚合 (User + RefreshToken + UserApiKey + Repository traits)
│   ├── log/             # Log 聚合 (LogMetadata + LogContent + LogTokenUsage + AuditLog + Repository traits)
│   └── shared/          # 跨聚合共享 (Status + ApiKey + AccessPointType + EncryptionService trait + RequestSnapshotProtocol trait)
├── application/         # 应用层 (用例编排)
│   ├── access_point/    # AccessPoint 用例 (服务 + DTO)
│   ├── auth/            # 认证用例 (服务 + DTO, 跨聚合)
│   ├── log/             # 日志用例 (服务 + DTO)
│   ├── provider/        # Provider / Account 用例 (服务 + DTO)
│   ├── proxy/           # 代理转发用例 (跨聚合)
│   ├── user/            # User / ApiKey 用例 (服务 + DTO)
│   └── mod.rs           # AppState
├── infrastructure/      # 基础设施层
│   ├── persistence/     # Repository 实现 (8 个) + PartitionManager
│   ├── encryption/      # AES-256-GCM 加密
│   ├── auth/            # JWT (jsonwebtoken) + argon2 密码哈希
│   └── http_client/     # 代理 HTTP 客户端 (ProxyClient + ProcessedRequest + ProxyLogger)
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
- 实体合并: 所有 SeaORM Model 实体定义统一在 `domain/` 的各聚合目录中, 不再存在独立的 `infrastructure/persistence/entities/` 目录。实体字段直接使用领域类型 (Status/ShortCode/AccessPointType/ModelMappingCollection) 通过 DeriveActiveEnum/DeriveValueType/FromJsonQueryResult 自动转换
- 聚合根: AccessPoint 的 `ModelEx` (= `AccessPointEx`) 是代理管道的聚合根, 包含已加载的 Provider 和 Account 关联。`find_by_short_code` 返回 `AccessPointEx`, ProxyPipeline 仅与该聚合根交互, 不再直接引用 Provider/Account 类型

## 数据库 Schema (11 个核心表)

| 表 | 说明 |
|---|---|
| providers | LLM 提供商 (含 default_model 字段) |
| accounts | API 账号 (AES-256-GCM 加密存储 Key) |
| users | 管理员用户 (argon2id 密码哈希) |
| user_api_keys | 用户 API key (SHA-256 哈希存储, 支持吊销) |
| access_points | 接入点 (短码、api_type、模型映射、跨聚合引用 Provider+Account) |
| refresh_tokens | JWT 刷新令牌 |
| log_metadata | 代理日志元数据和可展示摘要 (PartitionManager 按月分区) |
| log_contents | 代理日志原始内容 (按月分区, 请求头/请求体/响应体, 按需查看) |
| log_token_usage | token 用量统计 (永久保留, input/output/cache/thinking/total) |
| audit_logs | 操作审计日志 |

- 迁移: 5 个文件 (初始 Schema + user_api_keys 表 + provider_default_model 列 + system_settings 表 + log_contents 分区化)

## 关键决策

- **接入 URL**: `/ap/<short_code>` -- 用户指定或自动生成 (默认 16 位随机短码)
- **JWT**: Access Token 30min + Refresh Token 7day
- **JWT 自动刷新**: 前端采用「请求前体检 + 401 兜底」双层防御。`ensureFreshToken` 在每次请求前检查 access_token 剩余有效期, 不足 `REFRESH_THRESHOLD_SEC = 300` 秒时主动调用 `/api/auth/refresh`; 401 响应触发单次重试兜底 (覆盖时钟漂移等边缘场景)。不使用 `setTimeout` 定时刷新 (浏览器后台 tab 冻结会导致定时器失效)
- **refresh_token 并发安全**: 后端 Refresh Token Rotation 模式下, 前端用模块级 `refreshing: Promise<string> | null` 缓存确保所有并发请求 await 同一个 refresh, 避免多个并发 refresh 互相吊销
- **过期 refresh_token 清理**: 后端 tokio 后台任务复用 `partition_check_interval_secs` 间隔定时调用 `delete_expired()`, 使用 `MissedTickBehavior::Skip` 跳过首个立即触发的 tick。不引入 Redis 也不使用 pg_cron, 保持单一 PG 依赖
- **加密**: AES-256-GCM (ENCRYPTION_KEY 环境变量 64 hex chars = 32 字节)
- **密码**: argon2id
- **分区**: PartitionManager 应用层管理, 按月 `RANGE (timestamp)`, 依赖原生 PostgreSQL 分区, 支持多副本 advisory lock 防冲突
- **代理**: SSE 流式逐块转发 + 异步日志写入; 写日志时同步解析 Claude Code 请求头、请求体、SSE 响应、thinking、tool_use 和 usage, 将可展示摘要写入 `log_metadata`, 将 token 统计写入 `log_token_usage`, 原始请求 / 响应保存在 `log_contents` 中按需查看
- **代理日志统一积累**: `ProxyLogger`（`infrastructure/http_client/`）贯穿整个代理生命周期。构造时从 `ProcessedRequest` + `AccessPointEx` 提取字段，填充 `ProxyLogData` DTO（防腐职责）；运行时通过 `append_body`/`set_body` 积累响应体；结束时调用 `flush()` 将完成的 DTO 交给 `LogService::record_proxy_log()`。无论流式/非流式/客户端断开，最终都统一走这条路径。Drop 中自动标记 `is_interrupted` 并触发 flush
- **响应头透明化**: 代理转发仅过滤 hop-by-hop 头 (`transfer-encoding`, `connection`, `keep-alive` 等)，其余上游响应头全部透传给客户端。不再硬编码 `content-type: text/event-stream` 等预设值
- **运输方式由响应决定**: 流式/非流式的判断依据上游响应头 `content-type` 是否包含 `text/event-stream`，替代此前基于请求特征 (`processed.is_streaming`) 的预设
- **ProxyLogData DTO**: `ProxyLogger` 持有并逐步填充的 DTO（`application/log/dto/`），flush 时一次性交给 `LogService`。ProxyLogger 不再构造领域实体——`LogService::record_proxy_log()` 内部从 DTO 构造 `LogMetadata`，消除了 `LogEntry::from_proxy_data()` 等拼装方法
- **LogMetadata 命名**: `log_metadata` 表的实体从 `LogEntry` 改名为 `LogMetadata`，与 `LogContent`、`LogTokenUsage` 构成统一的 `Log*` 家族命名
- **日志记录三阶段独立错误处理**: `record_proxy_log` 按元数据保存 → 内容保存 → token 用量解析三个阶段串行执行，元数据失败立即 return，内容/token 阶段失败仅记录 warn/error 不阻断后续流程
- **路由**: 公开路径 (`/api/auth/*`, `/ap/*`, `/api/health`) 跳过 JWT 认证
- **接入点认证**: `/ap/*` 路径跳过 JWT 中间件, 但在 ProxyService 中强制验证用户 API key (`Authorization: Bearer <user_api_key>`), 通过 SHA-256 哈希匹配后记录 user_id
- **代理 Header 构造**: `ProxyClient` 构建新的上游请求时, 入站 `authorization` 只用于用户 API key 认证, 不参与上游请求构造; 上游请求使用解密后的账号 API key 设置 `Authorization: Bearer <account_key>`; 仅复制 `x-*` 自定义头、`accept`、`content-type` 等业务头, 并排除入站 `authorization` / `x-api-key`
- **用户 API key**: 个人设置页管理, key 以 `tp_` 为前缀, 生成 40 位随机字符, 数据库仅存储 SHA-256 哈希和前缀, 完整 key 只创建时返回一次; 支持吊销操作
- **模型映射匹配方式**: 由源模型值决定——`__unmatched__` 哨兵和 Claude 家族预设 (`claude-opus-*`/`claude-sonnet-*`/`claude-haiku-*`) 自动使用 `prefix` 匹配; 普通值或自定义输入使用 `exact` 匹配。后端有双重安全网: (1) **写时归一化**——`AccessPointService` 创建/更新时调用 `normalize_match_type` 将 match_type 强制归一化; (2) **读时归一化**——`ModelMapping::matches` 和 `find_matching_mapping` 在匹配逻辑中再次调用 `normalize_match_type`, 确保无论数据库存储值如何, 匹配行为始终正确。`normalize_match_type` 定义在 `model_mapping.rs` 中, 依赖 `is_prefix_source_model` 判断 source_model 是否属于上述前缀类别
- **模型映射优先级**: 请求模型按以下顺序匹配——精确匹配 > 前缀匹配 > `__unmatched__` 哨兵规则 (target 为 `__default_model__` 时通过 `resolve_final_model` 解析为 Provider 当前 default_model) > Provider.default_model 兜底 > 原始请求模型
- **Claude 模型族前缀常量**: `claude-opus-` / `claude-sonnet-` / `claude-haiku-`, 在前端 ModelMappingEditor 中以预设选项形式展示
- **Provider.default_model**: 可选配置, 当模型映射均未匹配时作为兜底, 替换请求体中的 model 字段。表格中使用 Tag 展示; 编辑表单中 Select 位于模型列表下方, 紧邻 TagInput; 默认模型 label 样式与模型列表一致; 编辑态提交按钮文案统一使用 "更新"; 模型列表为空时 Select 禁用; 模型列表移除当前默认模型时立即清空 Select 值; 保存时若 models 不含 default_model 则提交空字符串清空
- **AccessPoint.api_type**: 接入点 API 类型枚举, 当前有效类型为 `Anthropic`; 前端表单限定 Select 选项
- **DEFAULT_MODEL_SENTINEL**: 模型映射目标模型哨兵值 `__default_model__`, 前后端常量同名但独立定义 (后端 `DEFAULT_MODEL_SENTINEL`, 前端 `DEFAULT_MODEL`)。当映射的 target_model 为该哨兵值时, 代理转发通过 `resolve_final_model` 函数动态解析为 Provider 当前 `default_model`; 前端选择该哨兵时 UI 展示格式为 "默认模型 (实际模型)", 通过请求 Provider 详情动态显示当前 default_model 名称。`__unmatched__ -> 默认模型` 自动规则的 target_model 保存为 `__default_model__`, 接入点映射无需跟随 Provider 默认模型变更

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
- 后端: 94 个 Rust 源文件, 遵循 Rust 2021 edition 惯例
- 前端: 45 个 TypeScript 源文件, 遵循 TypeScript 6 严格模式

### 后端 (Rust)

- 模块按 DDD 分层组织, `mod.rs` 只做 re-export
- `AppError` 9 种变体: Validation, NotFound, Conflict, Unauthorized, Forbidden, Encryption, Database, Upstream, Internal
- 使用 `Arc<dyn Trait>` 依赖注入, 在 `main.rs` 组装
- 应用层 Service 构造时注入 Repository traits, 不直接依赖 SeaORM

### 前端 (React + Semi Design)

- 页面组件集中在 `src-dashboard/pages/`, 负责数据加载和视图切换; 表格/详情等具体渲染逻辑提取到 `src-dashboard/components/` 中
- 使用 `@douyinfe/semi-ui` 组件库
- 路由: react-router-dom v7 (BrowserRouter + Routes + AdminLayout)
- 路由结构: `/login`, `/dashboard`, `/providers`, `/access-points`, `/sessions`, `/logs`, `/users`, `/settings`, `/settings/profile`
- 后端通信: `src-dashboard/api.ts` (axios/fetch 封装, 含 ensureFreshToken 自动刷新、401 兜底重试、clearAuthAndRedirect 登出)
- **JWT 自动刷新**: `api.ts` 使用 `getTokenExp(token)` 本地 base64url 解码 JWT payload 读取 exp 字段 (不引入 jwt-decode 依赖)。`ensureFreshToken` 请求前体检, `REFRESH_THRESHOLD_SEC=300`; 模块级 `refreshing` Promise 去重防止并发刷新互相吊销; `clearAuthAndRedirect` 清理全部 localStorage auth 字段并跳转 `/login`
- **防重复点击**: 所有触发 API 调用或异步操作的按钮必须设置 `loading`/`disabled` 状态, 操作完成后才解除锁定。管理列表页使用 `operatingId` 实现行级按钮独立锁定
- **Modal 表单提交**: 包含 `Form` 的 `Modal` 必须使用 `footer` 承载取消 / 确认按钮, 不要把操作按钮放在 `Form` 内容区; `footer` 中的确认按钮必须通过 `getFormApi` 保存的 `formApi.submitForm()` 触发表单提交, 并设置 `loading`/`disabled` 防重复触发
- **改密自动登出**: 修改密码操作成功后, 前端必须清除所有 localStorage 令牌 (`access_token`, `refresh_token`, `username`, `display_name`) 并跳转 `/login`, 强制用户重新认证
- **主题切换**: 使用 `useTheme` hook + `ThemeProvider` (src-dashboard/hooks/useTheme.ts) 管理主题状态, 通过 localStorage key `theme_mode` 持久化, 支持 light / dark / system 三种模式; `ThemeToggle` 组件以 Dropdown 菜单形式展示在 AdminLayout 顶栏
- **ModelMappingEditor**: 接入点表单中的模型映射编辑器, 仅保留一个"添加映射"按钮。源模型使用 Semi Select, 支持搜索和 allowCreate; 选项以 Semi Tag 前缀显示匹配类型 (`精准匹配`/`模式匹配`); 源模型选项包括: `__unmatched__`(模式匹配)、Claude Opus/Sonnet/Haiku 预设(模式匹配)、以及 Provider 的 models 列表(精准匹配)。`__unmatched__` 和 Claude 家族源模型保存为 `prefix` 匹配类型, 普通值/自定义输入值保存为 `exact` 匹配类型。目标模型使用 Semi Select, 选项包含 Provider.models 和 `__default_model__` 哨兵, 不允许 allowCreate; 选择 `__default_model__` 时 UI 显示为"默认模型 (实际模型)", 动态解析 Provider 当前 default_model 并展示; 当源模型为 `__unmatched__` 时, 目标模型自动填充为 `__default_model__`
- **AccessPointDrawer 映射管理**: 接入点创建/编辑时选择 Provider 后, 会请求 `GET /api/providers/{id}` 刷新 Provider 的 models 和 default_model 列表; 创建态选择带默认模型的 Provider 时自动预填一条 `__unmatched__ -> __default_model__` 的映射规则。保存接入点时过滤映射, target_model 必须属于 Provider.models 或等于 `__default_model__` 哨兵
- **日志前端展示**: 请求日志列表使用 `RequestLogTable` 组件, 优先展示 `log_metadata.message_preview`, 内容超出时用 Semi Tooltip 展示 `message_full`; 会话日志页面 `SessionLogPage` 瘦身为路由壳, 负责数据加载和视图切换; 列表模式使用 `SessionListView` (筛选栏 + 表格), 详情模式使用 `SessionDetailView` (信息卡片 + `ClaudeSessionTimeline` 时间线 + 事件摘要表格 + `RawContentModal`); 默认不批量读取 `log_contents`, 点击”原始内容”时再请求 `/api/logs/{id}/raw`

## 设计原则

以下原则来自实际重构中的错误，与具体业务逻辑无关，适用于任何涉及职责分配的场景。

### 参数所有权测试

判断一个方法是否放在正确的对象上，看它的**所有参数是否都属于该对象的概念范围**。如果方法签名的某个参数不是该类型的字段、也不属于该类型所代表的领域概念，那这个方法大概率放错了位置 — 它应该属于更上层的聚合根。

- 方法 `find_rule(request)` — `request` 是外部输入，不是集合自身的概念。但查找规则的**逻辑**（匹配方式、优先级）完全由集合内部的规则项定义，且 `request` 作为查询条件只是提供一个匹配目标，不携带外部上下文。这种情况参数所有权不构成问题。
- 方法 `resolve(request, default)` — `default` 既不是该类型的字段，也不属于该类型代表的概念。它来自另一个聚合的字段。方法需要这个参数意味着它的职责范围已经超出了该类型。

简单判断：**如果一个参数放进构造函数里会觉得别扭，那它就不该出现在该类型的方法签名里。**

### 聚合根作为行为入口

聚合根对外提供行为，内部的值对象负责执行底层操作。调用方不应该穿透聚合根去访问内部值对象的方法来获取决策结果：

```
// 错误：调用方穿透了两层
aggr.inner_collection.resolve(x, y)

// 正确：调用方只对聚合根发问
aggr.resolve(x, y)
```

聚合根的方法内部可以委托给值对象（`self.inner_collection.find(x)`），但值对象的方法只应返回自己的数据结构，不接收自己领域之外的数据作为输入参数。

### “行为靠近数据”的边界

“把行为移到数据旁边”是一条好原则，但它有边界 — 只适用于**数据是自己、参数也只涉及自己**的场景。一旦方法签名引入了不属于该类型的概念，就说明行为不属于这里，应该移到能容纳所有参数的最小子系统中（通常就是上一层的聚合根）。

## 注意事项

- `.rs` 空文件留作占位用, 不应删除
- 前端构建产物 (`dist/`) 会被嵌入后端二进制
- 所有 Repository 实现以 `SeaOrm` 为前缀 (如 `SeaOrmProviderRepository`)
- 迁移文件在 `src/migrations/` 目录下, 使用 `sea-orm-migration`
- 分区管理由 `src/infrastructure/persistence/partition_manager.rs` 的 PartitionManager 处理, 通过 pg_inherits 系统表管理分区
- `log_metadata` 分区表的 `PRIMARY KEY` 必须包含 `timestamp`
- 日志默认展示不得依赖 `log_contents`; `log_metadata` 必须足够支撑请求日志列表, `log_contents` 仅用于原始明细弹窗（按需加载）
- `__unmatched__` 哨兵是模型映射中的特殊 source_model, 使用 `prefix` 匹配类型, 用于为所有未精确/前缀匹配的请求模型指定目标模型, 每个接入点最多一个。接入点创建时, Drawer 自动预填一条 `__unmatched__ -> __default_model__` 映射。保存接入点时过滤映射, target_model 必须属于 Provider.models 或等于 `__default_model__` 哨兵
- `api_type` 枚举的实际范围在 `AccessPointType` 值对象中定义, 新增类型需要同步修改 Rust 枚举 + 数据库列约束 + 前端 Select 选项
- `domain/` 按聚合组织为 5 个子目录: access_point/ provider/ user/ log/ shared/。每个聚合目录包含其所有实体、值对象和 Repository trait。跨聚合共享类型放在 shared/。不再使用 entities/、value_objects/、repositories/、services/ 技术类别目录
- AccessPoint 聚合的 `ModelEx` (= `AccessPointEx`) 是代理管道的聚合根。Repository 的 `find_by_short_code` 返回 `AccessPointEx`（包含已加载的 Provider 和 Account）。ProxyPipeline 仅与 AccessPointEx 交互，不直接引用 Provider 或 Account 类型
- `ProxyLogger`（`infrastructure/http_client/proxy_logger.rs`）是有状态的请求级日志积累器，持有 `ProxyLogData` DTO 并逐步填充，不直接构造领域实体。`flush()` 时将 DTO 提交给 `LogService::record_proxy_log()`，后者内部构造 `LogMetadata`、`LogContent`、`LogTokenUsage` 并写入数据库

## 核心文件路径

| 文件 | 说明 |
|---|---|
| `src/main.rs` | 启动入口 (依赖组装 + Router 构建 + 分区管理器初始化) |
| `src/lib.rs` | Crate 根模块 |
| `src/config.rs` | 环境变量配置加载 |
| `src/application/mod.rs` | AppState 定义 |
| `src/shared/error.rs` | AppError 错误类型 |
| `src/application/proxy/proxy_pipeline.rs` | 核心代理转发管道 (聚合根模式: 加载 AccessPointEx → validate_usable → decrypt_upstream_key → ProcessedRequest.prepare → 转发; 日志收集委托给 ProxyLogger) |
| `src/infrastructure/http_client/proxy_logger.rs` | ProxyLogger: 请求级日志积累器，持有 ProxyLogData DTO，从 ProcessedRequest + AccessPointEx 提取字段（防腐），flush 时提交 DTO 给 LogService |
| `src/infrastructure/http_client/processed_request.rs` | ProcessedRequest (编排 inbound → outbound 变换、URL 构造、session 提取) |
| `src/application/log/log_service.rs` | 日志写入和查询服务 (从 ProxyLogData DTO 构造 LogMetadata → 写入 metadata/content/token usage 三阶段) |
| `src/application/log/dto/proxy_log_data.rs` | ProxyLogData DTO: ProxyLogger → LogService 之间的数据契约 |
| `src/infrastructure/parsers/log_content.rs` | 请求体、SSE、thinking、tool_use 和 token usage 解析器 |
| `src/infrastructure/parsers/claude_code.rs` | Claude Code 请求头解析器 (`x-claude-code-session-id` / `x-claude-code-agent-id`) |
| `src/application/user/api_key_service.rs` | 用户 API key 管理服务 |
| `src/presentation/routes/me_routes.rs` | 个人设置路由 (`/api/users/me/*`) |
| `src/migrations/m20260101_000001_initial.rs` | 初始数据库 Schema |
| `src/migrations/m20260523_000001_user_api_keys.rs` | 用户 API key 表迁移 |
| `src/presentation/routes/mod.rs` | 路由聚合 |
| `src/presentation/middleware/jwt_auth.rs` | JWT 认证中间件 + CurrentUser extractor |
| `src/infrastructure/persistence/partition_manager.rs` | 分区管理器 |
| `src/infrastructure/persistence/repositories/user_api_key_repository.rs` | UserApiKey 仓储实现 |
| `src/domain/user/user_api_key.rs` | 用户 API key 领域实体 (User 聚合) |
| `src/domain/provider/provider.rs` | Provider 实体 (含 default_model + base_url_for 方法) |
| `src/domain/access_point/model_mapping.rs` | ModelMapping + MatchType + 模型族前缀常量 + normalize_match_type/is_prefix_source_model |
| `src/domain/access_point/access_point.rs` | AccessPoint 聚合根 (SeaORM Model + ModelEx + base_url/resolve_model/transform_request_snapshot/validate_usable/decrypt_upstream_key) |
| `src/domain/shared/request_snapshot.rs` | RequestSnapshot 值对象 (headers + body + model) |
| `src/domain/shared/request_snapshot_protocol.rs` | RequestSnapshotProtocol trait (extract_model / replace_model / is_streaming / transform_headers) |
| `src/domain/shared/api_type.rs` | AccessPointType 枚举 (Anthropic) |
| `src/infrastructure/protocols/anthropic.rs` | AnthropicRequestSnapshot 实现 |
| `src/migrations/m20260524_000001_provider_default_model.rs` | providers 表增加 default_model 列 |
| `src-dashboard/App.tsx` | 前端路由定义 |
| `src-dashboard/pages/ProfilePage.tsx` | 个人设置页 (个人资料/改密/API key 管理) |
| `src-dashboard/pages/ProviderManagement.tsx` | Provider 管理页 (表格中 default_model 用 Tag 展示; 表单中 Select 位于模型列表下方; 模型列表移除当前默认模型时立即清空) |
| `src-dashboard/components/ModelMappingEditor.tsx` | 模型映射编辑器 (单个添加按钮, 源 Select + search + allowCreate + label 显示匹配类型, 含未匹配/Claude 预设/Provider 模型, Claude 预设存 prefix 其余存 exact; 目标 Select 仅含 Provider models, 不允许 allowCreate) |
| `src-dashboard/components/AccessPointDrawer.tsx` | 接入点创建/编辑抽屉 (选择 Provider 后有 default_model 则自动预填 `__unmatched__ -> default_model` 映射; 保存时过滤 target_model 不在 Provider.models 的映射) |
| `src-dashboard/components/ThemeToggle.tsx` | 主题切换组件 (light/dark/system) |
| `src-dashboard/components/ClaudeSessionTimeline.tsx` | 会话事件流展示组件 (用户消息、thinking、tool_use、Agent 调用) |
| `src-dashboard/components/RequestLogTable.tsx` | 请求日志表格 (列定义、分页、空态) |
| `src-dashboard/components/SessionListView.tsx` | 会话列表视图 (筛选栏 + 表格) |
| `src-dashboard/components/SessionDetailView.tsx` | 会话详情视图 (信息卡片 + 时间线 + 事件表格 + RawContentModal) |
| `src-dashboard/hooks/useTheme.ts` | 主题 Hook + ThemeProvider |
| `src-dashboard/hooks/useAccessPoints.ts` | 接入点管理 Hook (含 api_type 传递) |
| `src/application/auth/service.rs` | 认证服务 (login/refresh/logout, Refresh Token Rotation, expires_at 修复) |
| `src/infrastructure/auth/jwt.rs` | JWT 服务 (access + refresh 双 token 签发、refresh_expiry_secs 访问器) |
| `src-dashboard/api.ts` | 前端 API 封装 (fetch 封装、ensureFreshToken 自动刷新、401 兜底重试、并发去重) |
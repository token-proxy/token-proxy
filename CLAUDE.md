# Token Proxy

企业级 LLM API 资源管理平台，提供统一的 API 代理、密钥管理、用量监控和访问控制能力。

> 架构详情见 [ARCHITECTURE.md](./ARCHITECTURE.md)

## 技术栈

- **后端**: Rust (edition 2021) + axum 0.8 + SeaORM 2 + tokio
- **前端**: React 19 + TypeScript + Vite + Semi Design 2.97（pnpm 管理依赖）
- **数据库**: PostgreSQL 17（应用层按月分区管理）
- **代码质量**: Prettier + lint-staged + simple-git-hooks（pre-commit 自动格式化）、cargo fmt/clippy
- **CI/CD**: GitHub Actions（fmt + clippy + build + PostgreSQL 集成测试）、Dependabot（每周依赖检查）
- **构建**: cargo-make + Docker 多阶段构建（.dockerignore 优化构建上下文）
- **工具链**: Rust 工具链固定于 1.96（rust-toolchain.toml），前端 pnpm 版本通过 `packageManager` 固定

## 架构概要

DDD 四层：领域层（`domain/`）→ 应用层（`application/`）→ 基础设施层（`infrastructure/`）→ 展示层（`presentation/`）

| 层         | 路径                                                                            | 职责                                |
| ---------- | ------------------------------------------------------------------------------- | ----------------------------------- |
| 领域层     | `src/domain/{access_point,provider,proxy,user,log,system,shared}/`              | 实体、值对象、仓储 trait            |
| 应用层     | `src/application/{access_point,auth,dashboard,log,provider,proxy,system,user}/` | 用例编排、DTO                       |
| 基础设施层 | `src/infrastructure/{persistence,encryption,auth,http_client,parsers}/`         | 仓储实现（SeaORM）、加密、JWT、HTTP |
| 展示层     | `src/presentation/{routes,middleware}/`                                         | axum 路由处理、认证中间件           |
| 共享       | `src/shared/`                                                                   | AppError（9 种）、PaginatedResult   |
| 前端       | `src-dashboard/`                                                                | React SPA，构建产物嵌入二进制       |

### 核心规则

- **依赖注入**: `Arc<dyn Trait>`，`main.rs` 组装；Service 注入仓储 trait，不直接依赖 SeaORM
- **聚合根**: `AccessPointEx` = 接入点 + 账户池 + 路由网格，代理管道（`ProxyPipeline`）唯一交互入口
- **仓储命名**: 所有仓储实现以 `SeaOrm` 为前缀，如 `SeaOrmAccessPointRepository`
- **实体 = ORM 实体**: 领域层直接使用 SeaORM DeriveEntityModel，聚合子目录内聚所有类型
- **审计日志**: `AuditAction`（18 variant）和 `AuditEntityType`（8 variant）类型安全审计；所有 Service 通过私有 `log_audit`
  辅助方法即发即忘（fire-and-forget）写入，禁止 `?` 传播审计错误
- **后台写入**: 所有异步落库必须通过 `TrackedSpawner::spawn(operation, future)` 入队，禁止裸 `tokio::spawn`（绕过
  `in_flight_writes` 计数和优雅关闭守卫）
- **仓储 trait 契约**: 所有仓储 trait 必须使用 `#[async_trait] + Send + Sync`，方法返回 `Result<..., AppError>`
- **DTO 目录统一**: 每个 Service 的 DTO 放在 `dto/` 子目录，`mod.rs` 用 `pub use` 重导出；外部引用使用绝对路径
  `crate::application::<聚合>::dto::*`
- **Cargo.lock 与 pnpm-lock.yaml 纳入版本控制**（确定性构建），`package-lock.json` 不纳入（`.gitignore` 排除）
- **`.`rs` 空文件留作占位**，不应删除

## 术语表

| 中文           | 英文原文                     | 说明                                                                                                             |
| -------------- | ---------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| 接入点         | Access Point                 | 对外暴露的 API 调用入口，通过短码 URL 提供服务                                                                   |
| 服务商         | Provider                     | 上游 LLM API 服务商（如 Anthropic、OpenAI）                                                                      |
| 账号           | Account                      | 服务商下的具体 API 账号                                                                                          |
| 短码           | Short Code                   | 接入点的 URL 标识，用户指定或自动生成 16 位随机码                                                                |
| 词元           | Token                        | LLM 用量度量单位                                                                                                 |
| 未命中缓存输入 | Input Tokens (cache miss)    | 全新提交到模型处理的输入词元，不走缓存（`input_tokens` 列）                                                      |
| 缓存命中输入   | Cache Read Input Tokens      | 从上下文缓存直接读取的输入词元，可享受计费折扣（`cache_read_input_tokens` 列）                                   |
| 缓存创建输入   | Cache Creation Input Tokens  | 首次写入上下文缓存的输入词元，全价计费（`cache_creation_input_tokens` 列）                                       |
| 输出词元       | Output Tokens                | 模型生成的输出词元，不含思考过程词元（`output_tokens` 列）                                                       |
| 思考词元       | Thinking Tokens              | 模型内部推理过程产生的词元，Anthropic 为 `thinking_tokens`，OpenAI 为 `reasoning_tokens`（`thinking_tokens` 列） |
| 路由策略       | Routing Strategy             | 账户池排序策略：Priority（按优先级）/ Weighted（权重随机）                                                       |
| 模型路由网格   | Model Routing Grid           | 二维表格（source_model × provider_id），精确匹配 > 前缀匹配 > `__unmatched__` 兜底                               |
| 会话粘滞       | Session Affinity             | 同一会话复用同一账号                                                                                             |
| 客户端类型     | Client Type                  | ClaudeCode / Codex / Other / Unknown，与协议类型正交                                                             |
| 协议类型       | API Type / Access Point Type | Anthropic / OpenAi，挂载 5 个协议适配方法                                                                        |
| 代理管道       | Proxy Pipeline               | 核心转发调度骨架                                                                                                 |
| 审计日志       | Audit Log                    | 记录所有管理操作的审计轨迹                                                                                       |
| 故障检测       | Fault Detection              | 上游响应自动归类并触发账号禁用                                                                                   |
| 重试决策       | Retry Decision               | `Return(Response)` 终止 / `Continue(AppError)` 切换下一候选                                                      |
| 优雅关闭       | Graceful Shutdown            | 关闭期间新请求短路，等待在途请求完成                                                                             |
| 即发即忘       | Fire-and-Forget              | 异步写入不阻塞主业务，失败仅记录日志                                                                             |
| 聚合根         | Aggregate Root               | 聚合的唯一入口（如 `AccessPointEx`）                                                                             |
| 值对象         | Value Object                 | 无独立标识的领域概念（如 `RoutingStrategy`、`ShortCode`）                                                        |
| 领域服务       | Domain Service               | 跨实体的领域逻辑（如 `FaultService`）                                                                            |
| 仓储           | Repository                   | 持久化抽象（trait 在领域层、实现在基础设施层）                                                                   |

## 架构决策与约束

### 代理转发

- **接入 URL**: `/ap/<short_code>`，用户指定或自动生成 16 位随机短码
- **入站认证隔离**: 入站 `Authorization` 仅用于用户 API key 认证；上游请求独立构建，API key 注入由
  `AccessPointType::inject_api_key` 协议方法负责
- **Header 透传规则**: 仅透传 `x-*`、`accept`、`content-type` 等业务头；响应端过滤逐跳头（`transfer-encoding`、`connection`、
  `keep-alive` 等），其余透明转发
- **流式判断**: 由 `AccessPointType::is_sse_response(&resp_headers)` 依据上游 `Content-Type: text/event-stream`
  判定，非基于请求特征预设
- **响应分类**: `UpstreamOutcome` enum（`Success` / `ClientError` / `Fault` / `ServerError`），`classify` 是唯一入口；SSE
  错误路径 `resp_body=None`，body-based 故障规则静默忽略
- **重试决策**: `RetryDecision` enum（`Return(Response)` / `Continue(AppError)`），类型系统强制重试携带错误原因
- **账户池路由**: 失败自动重试下一账号（`AccountSelector` 迭代候选 + `RetryDecision::Continue` 串联）
- **会话粘滞**: `session_affinity` 表（`access_point_id` + `session_id`），首次创建、后续复用；写入通过 `TrackedSpawner` 即发即忘
- **优雅关闭短路**: `ProxyPipeline::execute` 第 0 步检查关闭信号，关闭期间新请求立即返回 `AppError::Upstream`
- **模型路由网格**: 匹配优先级：精确匹配 > 前缀匹配 > `__unmatched__` 兜底 > 原始模型值；`__unmatched__` 行为兜底规则，每个接入点自动生成

### 协议适配

- **协议方法挂在 `AccessPointType` 枚举上**: `parse_inbound` / `extract_session_id` / `inject_api_key` /
  `replace_model_in_body` / `is_sse_response`，具体实现位于 `domain/shared/protocols/<name>.rs`
- **新增协议只需补 enum variant + 新建协议文件**，编译器会自动指出所有需要补 match 的位置
- **`ClientType` 与 `AccessPointType` 正交**: 同一 OpenAI 接入点可被 Claude Code 和 Codex 同时访问；
  `ClientType::from_request` 按品牌 header → UA 关键词 → 可识别特征 → Unknown 四级降级识别
- **`session_id` 解析由 `ClientType` 驱动**（ClaudeCode → `x-claude-code-session-id`，Codex → `thread-id`），
  `AccessPointType::extract_session_id` 仅作协议层兜底。`session_id` 在请求路径上是 `Option<String>`（`None` 表示未携带），写入
  `log_metadata.session_id`（NOT NULL）时回落 `"unknown"`
- **OpenAI 词元归一化**: Chat Completions（`prompt_tokens`/`completion_tokens`）和 Responses API（`input_tokens`/
  `output_tokens`）统一映射到 `log_token_usage` 列

### 安全与认证

- **JWT**: Access Token 30min + Refresh Token 7day
- **JWT 自动刷新**: 前端「请求前体检 + 401 兜底」双层防御，`REFRESH_THRESHOLD_SEC=300`；模块级 `refreshing` Promise 去重防并发
- **过期 refresh_token 清理**: tokio 后台任务 + `MissedTickBehavior::Skip`，不引入 Redis 或 pg_cron
- **加密**: AES-256-GCM（`ENCRYPTION_KEY` 64 hex = 32 字节）；密码使用 argon2id 哈希
- **敏感信息脱敏**: `headers_to_json()` 自动将 Authorization/Cookie/Set-Cookie 替换为 `[REDACTED]`；日志严禁记录密钥明文、JWT
  token、密码
- **用户 API key**: SHA-256 哈希存储，仅创建时返回一次完整值

### 数据库与分区

- **分区策略**: `log_metadata` / `log_contents` 按月 RANGE 分区（`PartitionManager` 自动管理，advisory lock 防冲突）；
  `log_token_usage` 永久保留不分
- **迁移文件**: `src/migrations/` 下，使用 `sea-orm-migration`
- **`log_metadata` 分区表 PRIMARY KEY 必须包含 `timestamp`**

### 审计日志

- **全覆盖**: 7 个 Service 的 26 个写操作全部写入 `audit_logs` 表
- **写入模式**: 统一即发即忘 + `tracing::error!`，不阻塞主业务
- **operator_id**: 管理员操作传入 `CurrentUser` UUID，系统自动操作（如 AutoRecover、DiscoverModels）传 `None`；entity_id
  无意义时传 `None`，不构造伪 UUID
- **前端可见**: 审计日志页面（`/audit-logs`）对所有登录用户开放，支持按操作类型、实体类型、操作者、时间范围筛选；操作者名字通过后端 LEFT JOIN users 表获取；中文映射在前端 `utils/auditLog.ts` 维护，后端返回 snake_case 原始值
- **查询端点**: `GET /api/audit-logs`，支持 `action`、`entity_type`（逗号分隔多选）、`operator_id`、`start_time`、`end_time`、`page`、`page_size` 筛选

### 枚举新增同步规则

以下枚举新增 variant 需同步修改三处：Rust 枚举定义 + 数据库列约束（VARCHAR）+ 前端展示/Select

- `AccessPointType`（协议类型）、`DisabledReason`（禁用原因）、`AuditAction`（审计操作）、`AuditEntityType`（审计实体）

### 日志记录

- **三阶段**: 元数据 → 内容 → 词元用量；元数据失败立即 return，后续失败仅 warn/error 不阻断
- **记录器位置**: `ProxyCallRecord` 位于 `application/proxy/`（直接接受领域聚合根），API 反映业务时序：
  `start → attach_response → append_body/set_body → finish`，`Drop` 兜底 SSE 中断
- **Drop 兜底不可移除**: SSE 客户端中断时唯一可靠的落库机制
- **列表默认不依赖 `log_contents`**：优先用 `log_metadata`，原始内容按需加载（`/api/logs/{id}/raw`）

### Dashboard 个人用量报告

- **个人视角（无管理员/全局视图）**: 所有端点强制 `user_id` 过滤，handler 从 `CurrentUser` 注入；所有聚合 SQL 含
  `WHERE user_id = ?`
- **5 个 GET 端点**: `/api/getting-started/{heatmap,kpi,top-models,top-access-points,quality}`
- **时间窗口**: 除 heatmap 外共享 `?range=today|last7|last30|custom`（默认 `last7`）；heatmap 固定当前自然年，支持请求次数 /
  词元总量 toggle；切换到 custom 时，前端 `TimeRangeSelector` 不立即触发 `onChange`（不发起后端请求），仅根据当前已选范围初始化草稿日期并打开 DatePicker：today 为当天 00:00 到当前时间，last7 为最近 7 天，last30 为最近 30 天，已有 custom start/end 时保留原自定义范围；用户选择完自定义日期后，DatePicker 回调才提交 `onChange`。数据指标和用量趋势两个时间选择器都遵循该规则
- **时区 SQL 注入防护**: 热力图按浏览器时区分桶，`validate_timezone()` 用 `chrono_tz::Tz::from_str` 白名单校验后
  `format!` 拼接 `AT TIME ZONE`（PostgreSQL 不接受参数占位符，白名单是唯一防线）；heatmap 数据获取 deps 为 `[]`（空数组），仅挂载时加载，刷新由 `UsageOverviewCard` 自身的刷新按钮触发 `refetch`，不受 `timeRange` 影响
- **sparkline 空桶补齐**: SQL 端 `generate_series` 完成，应用层无需再补
- **趋势对比**: 覆盖 5 种边界（up / down / flat / new / empty）

### SSE 实时广播

- `LogService::record_proxy_log` 完成后通过 `broadcast::channel(256)` 广播 `NewLogEvent`（仅含标识信息，不含敏感数据）
- 满时丢弃最旧事件（仅 warn 日志），前端 `onVisibilityRecover` 全量刷新兜底
- 前端 `EventSource` 消费 `GET /api/logs/events`，JWT 通过 URL query 参数 `?token=` 传递（EventSource 不支持自定义 header）
- `useLogEvents` Hook: 页面隐藏时暂停处理，恢复可见时全量刷新而非逐条消费积压

## 编码规范

### 注释规范

**三必写:**

- **模块入口必写**: 非平凡 `.rs` 文件必须以 `//!` 模块文档开头（说明层级、聚合、主要类型）。例外：仅含 `pub mod` 的 barrel 文件
- **公开 API 必写**: 每个 `pub` struct/enum/trait/fn/method 必须有 `///` 文档。例外：自解释的简单访问器
- **类型契约必写**: DTO struct 必须有文档说明用途；关键字段必须有行内文档；前端 `types/` 下每个 interface 必须有 JSDoc

**一不写:**

- **简单代码不写冗余注释**: 不重复代码语义、不记录变更历史（属于 Git）、自解释访问器可不写

**行内注释规则:**

- 复杂逻辑（3 步以上）用编号行内注释（`// 1. 精确匹配` `// 2. 前缀匹配` ...）
- 功能区域用分隔线：`// ─── 领域行为 ───`（Rust）或 `// --- SSE 解析 ---`（TypeScript）
- 注释说"为什么"不说"是什么"：解释设计意图、边界条件、非显而易见的 hack

**语言和格式:**

- 所有注释使用中文；技术标识符（类型名、方法名）保持英文
- 中文与英文/数字之间必须保留空格
- 错误消息使用中文，日志字段使用英文

**前端 JSDoc:**

- 导出函数/组件必须有 JSDoc；hook 返回值必须文档化
- 组件 props 接口必须文档化（接口级 JSDoc + 非自解释字段）
- 复杂解析算法需要文件级 JSDoc + 步骤编号注释

**注释标杆参考:** `src/domain/shared/protocols/anthropic.rs`、`src/domain/shared/protocols/openai.rs`、
`src/application/proxy/proxy_pipeline.rs`、`src/application/proxy/proxy_call_record.rs`、`src/main.rs`、
`src-dashboard/utils/parseLogs.ts`、`src-dashboard/components/session/TurnCard.tsx`

### 日志规范

**框架**: 仅使用 `tracing` 宏（`info!`、`warn!`、`error!`、`debug!`、`trace!`），禁止 `println!`/`eprintln!`。级别由 `LOG_LEVEL`
环境变量控制（默认 `info`），生产环境输出 JSON。

| 级别     | 使用场景                                                               |
| -------- | ---------------------------------------------------------------------- |
| `error!` | 不可恢复的错误（数据库连接失败、加密失败、分区维护失败）               |
| `warn!`  | 可恢复的异常（账号禁用、会话保存失败、词元解析失败、审计日志写入失败） |
| `info!`  | 关键业务事件（启动/关闭、请求到达/完成、账号池选择、分区创建/清理）    |
| `debug!` | 诊断细节（账号选择过程、URL 构造、请求变换前后对比）                   |
| `trace!` | 极细粒度调试（逐 chunk SSE 转发、JSON 解析中间态）                     |

**结构化字段（强制）:** 所有日志必须使用 `field = %value` 格式，禁止字符串插值 `"xxx: {}", val`。

**关键路径必须记录（`proxy_pipeline.rs`）：** 请求到达 → 接入点加载 → 每次账号尝试 → 上游请求发出 → 请求完成 → 重试决策 →
账号耗尽 → 账号自动禁用。

**严**禁记录（日志 + 审计日志 details）：API 账号密钥明文、用户 API key 完整值、JWT
token、密码（明文或哈希）、Authorization/Cookie/Set-Cookie 头值、`ENCRYPTION_KEY`/`JWT_SECRET` 环境变量值。

**`#[instrument]` 属性:** Service 公开方法建议添加 `#[tracing::instrument(skip_all, fields(...))]`，`skip_all` 防敏感参数记录。

**前端日志:** API 错误必须 `console.error`（含方法、URL、状态码、错误消息）；禁止空 `catch {}`（至少 `console.warn`）；禁止输出
token/key/密码到控制台；提取 `X-Request-ID` 在 Toast 中展示。

### 通用编码约束

**后端约束:**

- `AppError` 9 种变体: `Validation(400)` / `NotFound(404)` / `Conflict(409)` / `Unauthorized(401)` / `Forbidden(403)` /
  `Encryption(500)` / `Database(500)` / `Upstream(502)` / `Internal(500)`
- 数据库错误和加密错误详情不暴露给客户端，统一转 500
- 引用对象（access_points / providers）一律 LEFT JOIN 并以 `Option<String>` 容忍删除
- DTO 字段类型优先使用 `String`（而非领域枚举），枚举转换在 Service 层进行
- 健康检查 `/api/health` 优雅关闭期间仍返回 200（防止 Docker 重启），就绪检查用 `/api/ready`

**前端约束:**

- 路径别名 `@components` → `src-dashboard/components/`，引用不带 `.tsx` 后缀
- 类型断言优先使用 `satisfies` 关键字替代 `as`
- 数据获取优先使用 `useFetch` Hook（`loading` 初始 `true`，`refetch` 自动设置 `loading=true`，所有 `setState`
  在异步回调中执行、卸载后不更新）
- 派生状态使用 `useMemo` 而非 `useState + useEffect`
- 所有异步按钮设置 `loading`/`disabled` 防重复；列表行级锁定使用 `operatingIdsRef`（useRef + Set）防并发
- Modal 表单：`footer` 承载按钮，确认通过 `formApi.submitForm()` 触发
- Semi Design ColorPicker 受控模式（修改 `value` 后通过 `onChange` 更新状态）
- CSS 变量统一使用 `var(--semi-color-*)` 确保暗色模式兼容
- 已删除接入点统一用全局 `.dashboard-deleted` CSS class（灰色 + monospace）
- 改密成功后清除所有 localStorage 令牌并跳转 `/login`
- 组件工具函数分离：纯函数/类型/常量分离到同级 `xxxUtils.ts` 或 `xxx.ts`
- 主题：`useTheme` hook + `ThemeProvider`，localStorage key `theme_mode`，支持 light/dark/system
- 无全局状态管理库（仅 theme 使用 Context）
- `AdminLayout` 侧边栏：`selectedKeys` 通过 `useMemo` 派生；折叠状态拆分为用户控制 + 自动（详情页）
- `AccessPointDrawer`：`rowSelectedProviders` 通过 `useMemo` 从 `formData.accounts` 和 `allKnownAccounts` 派生
- 前端按 `api_type` 顶层分发渲染：Anthropic → `parseLogs.ts`，OpenAI → `parseOpenAI.ts`
- 会话详情页轮次判定使用 `buildConversationTurns()`，**不要使用** `buildConversationEvents()` 渲染详情页
- 响应体格式检测优先通过 `Content-Type` 头部判定，`isJsonFormat` 用 JSON.parse 试探兜底

## 设计原则

### 核心纪律：应用层编排，领域层决策

- **应用层**（`*Service` / `ProxyPipeline`）：知道"先做什么、后做什么"——加载、调度、保存。不包含业务判断
- **领域层**（实体 / 值对象 / 领域服务）：知道"怎样判断、怎样计算"——验证、匹配、策略选择。不接触基础设施

### 贫血检测四信号

1. 枚举有 variant 但行为在别处 → 行为移入枚举 impl
2. 结构体公开字段被外部逐字段赋值 → 封装为实体行为方法
3. 自由函数接受领域类型返回领域结果但放在应用层 → 移到对应类型上或领域服务中
4. 应用层直接修改聚合根内部集合 → 封装为聚合根方法

### 逻辑归属决策速查

| 问题                            | 归属                                                   |
| ------------------------------- | ------------------------------------------------------ |
| 操作单个实体自身字段？          | 实体方法（如 `Account::disable_for`）                  |
| 操作值对象数据 + 外部数据？     | 值对象方法（如 `RecoverType::calculate_available_at`） |
| 协调两个或多个领域对象？        | 领域服务（如 `FaultService::detect`）                  |
| 涉及 Repository / HTTP / 加密？ | 应用服务（如 `ProxyPipeline`）                         |
| 纯粹 HTTP / 序列化 / 外部 API？ | 基础设施层（如 `ProxyClient`）                         |

### 聚合根作为行为入口

调用方不穿透聚合根访问内部值对象。聚合根对外暴露行为，内部委托给值对象。

## 核心文件速查

| 文件                                                 | 说明                                                                                    |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `src/main.rs`                                        | 启动入口（依赖组装 + 路由 + 分区 + 后台任务）                                           |
| `src/application/proxy/proxy_pipeline.rs`            | 代理管道（60 行调度骨架 + try_one_account）                                             |
| `src/application/proxy/proxy_call_record.rs`         | 代理调用记录器（start → attach → append/set → finish；Drop 兜底）                       |
| `src/application/proxy/tracked_spawner.rs`           | 后台写入调度器（统一 spawn 模板）                                                       |
| `src/application/proxy/account_selector.rs`          | 候选账号迭代器（加载 → 跳过 → 解密四步）                                                |
| `src/application/proxy/upstream_dispatcher.rs`       | 上游转发执行器（forward + 120s 超时）                                                   |
| `src/application/proxy/response_builder.rs`          | 响应构造（streaming / buffered + 逐跳头过滤）                                           |
| `src/domain/access_point/access_point.rs`            | AccessPointEx 聚合根（sort_accounts + apply_session_affinity + build_upstream_request） |
| `src/domain/access_point/routing_strategy.rs`        | 路由策略值对象                                                                          |
| `src/domain/access_point/model_routing_grid.rs`      | 模型路由网格值对象（二维匹配）                                                          |
| `src/domain/shared/api_type.rs`                      | AccessPointType 枚举 + 5 个协议方法                                                     |
| `src/domain/shared/client_type.rs`                   | ClientType 枚举（from_request + extract_session_id）                                    |
| `src/domain/shared/protocols/anthropic.rs`           | Anthropic 协议适配                                                                      |
| `src/domain/shared/protocols/openai.rs`              | OpenAI 协议适配（Chat Completions + Responses API）                                     |
| `src/domain/proxy/upstream_outcome.rs`               | UpstreamOutcome（Success/ClientError/Fault/ServerError）+ classify                      |
| `src/domain/proxy/retry_decision.rs`                 | RetryDecision（Return/Continue）                                                        |
| `src/domain/provider/fault_service.rs`               | 故障检测领域服务                                                                        |
| `src/domain/log/audit_action.rs`                     | 审计操作类型枚举（18 variant）                                                          |
| `src/domain/log/audit_entity_type.rs`                | 审计实体类型枚举（8 variant）                                                           |
| `src/domain/log/repository_audit_log.rs`             | AuditLogQuery 筛选条件 + AuditLogWithUsername 读模型 + AuditLogRepository trait         |
| `src/domain/log/dashboard_query.rs`                  | Dashboard 领域查询类型                                                                  |
| `src/application/log/log_service.rs`                 | 日志写入/查询（三阶段 + SSE 广播 + 审计日志查询）                                       |
| `src/application/log/dto/audit_log_filter_params.rs` | 审计日志筛选参数 DTO                                                                    |
| `src/application/log/dto/audit_log_response.rs`      | 审计日志列表响应项 DTO                                                                  |
| `src/application/dashboard/dashboard_service.rs`     | Dashboard 聚合服务（5 个个人视角方法）                                                  |
| `src/application/dashboard/timezone.rs`              | IANA 时区白名单校验                                                                     |
| `src/presentation/middleware/jwt_auth.rs`            | JWT 认证中间件 + CurrentUser extractor                                                  |
| `src/presentation/middleware/user_api_key_auth.rs`   | 用户 API key 认证中间件                                                                 |
| `src/presentation/routes/log_routes.rs`              | log 路由（含 `/api/audit-logs` 审计日志查询端点）                                       |
| `src-dashboard/api.ts`                               | 前端 API 封装（JWT 自动刷新 + auditLogApi）                                             |
| `src-dashboard/hooks/useFetch.ts`                    | 通用数据获取 Hook                                                                       |
| `src-dashboard/hooks/useLogEvents.ts`                | SSE 实时事件消费 Hook                                                                   |
| `src-dashboard/pages/GettingStartedPage.tsx`         | 「我的用量报告」顶层                                                                    |
| `src-dashboard/pages/AuditLogPage.tsx`               | 审计日志查看页（筛选 + 分页 + JSON 详情展开）                                           |
| `src-dashboard/types/auditLog.ts`                    | AuditLogItem、AuditLogFilters 接口                                                      |
| `src-dashboard/utils/parseLogs.ts`                   | 日志/会话解析（buildConversationEvents + buildConversationTurns）                       |
| `src-dashboard/utils/parseOpenAI.ts`                 | OpenAI 响应/请求体解析                                                                  |
| `src-dashboard/utils/auditLog.ts`                    | 审计日志中文映射（ACTION_LABELS、ENTITY_TYPE_LABELS 等）                                |

## Makefile 任务

| 命令                 | 说明                                      |
| -------------------- | ----------------------------------------- |
| `cargo make dev`     | 并行启动前端 Vite HMR + 后端              |
| `cargo make build`   | 顺序构建前端 + 后端 release               |
| `cargo make check`   | 并行 cargo check + pnpm exec tsc --noEmit |
| `cargo make preview` | build 并运行 release 二进制               |
| `cargo make fmt`     | cargo fmt                                 |
| `cargo make clippy`  | clippy（deny warnings）                   |
| `cargo make test`    | cargo test                                |

**Pre-commit**: `simple-git-hooks` + `lint-staged`，pre-commit 自动运行 eslint + prettier + cargo fmt。安装依赖后首次执行
`pnpm run prepare` 激活 hook。

## 环境变量

| 变量                            | 说明                    | 默认值   |
| ------------------------------- | ----------------------- | -------- |
| `DATABASE_URL`                  | PostgreSQL 连接串       | **必填** |
| `JWT_SECRET`                    | JWT 签名密钥            | **必填** |
| `ENCRYPTION_KEY`                | 64 hex chars（32 字节） | **必填** |
| `SERVER_PORT`                   | 监听端口                | 3000     |
| `LOG_LEVEL`                     | 日志级别                | info     |
| `PARTITION_CHECK_INTERVAL_SECS` | 分区检查间隔（秒）      | 3600     |
| `PARTITION_PREMAKE_MONTHS`      | 提前创建未来分区月数    | 3        |
| `PARTITION_RETENTION_MONTHS`    | 分区保留月数            | 12       |

## 发布流程

- **main 分支**: 版本号占位 `0.0.0`，实际版本仅在 release 分支设定
- **Git tag**: 无 `v` 前缀（`0.1.0`、`0.2.0-rc.1`）
- **CHANGELOG**: `git-cliff` 自动生成，约定式提交分组（feat→Added / fix→Fixed / perf+refactor→Changed）
- **发布步骤**: 从 main 创建 release 分支 → `cliff generate` 生成 CHANGELOG → 更新 Cargo.toml 版本号并打 tag → rebase
  合并 → cherry-pick CHANGELOG 回 main
- 使用 `/release <version> [<description>]` 命令执行发布流程

## 前端路由

```
/login → LoginPage
/ → AdminLayout → /getting-started (GettingStartedPage)
/dashboard → redirect /getting-started（兼容重定向）
/providers (ProviderManagement)
/access-points (AccessPointManagement)
/sessions → /sessions/:sessionId (SessionLogPage)
/logs → /logs/:id (LogDetailPage)
/users (UserManagement)
/audit-logs (AuditLogPage)
/settings (SettingsPage)
/profile (ProfilePage)
```

管理侧边栏: 开始使用、服务商管理、接入点管理、会话日志、请求日志、用户管理、审计日志、系统设置

## 注意事项（易错点）

- 跨聚合共享类型放 `domain/shared/`，不要放单个聚合内
- `domain/` 按聚合组织（7 个子目录），不使用 entities/value_objects/repositories 技术类别目录
- `UpstreamOutcome::classify` 是响应分类**唯一入口**，不要绕过直接判断
- `RetryDecision::Continue(AppError)` 强制携带错误原因，类型系统已杜绝裸 `continue`
- 响应体格式检测：前端各解析函数接受可选 `format` 参数避免重复检测
- Dashboard 所有聚合方法首参为 `user_id: Uuid`，SQL 必含 `WHERE user_id = ?`；不要恢复 `top_users`/`top_accounts` 等跨用户聚合
- Dashboard sparkline 空桶补齐由 SQL `generate_series` 完成，新增时间粒度需同步扩展 `DashboardWindow` 与步长
- `ProxyCallRecord` 持有 `ProxyLogInput` 入参（LogService 一次性入参契约），`LogService::record_proxy_log()` 内部构造
  `LogMetadata`
- `log_metadata.account_id` 记录实际使用的账号
- 响应体格式检测优先通过 `response_headers` 的 `Content-Type` 判定，`isJsonFormat` 作 JSON.parse 试探兜底
- `AccessPointService::new()` 和 `AuthService::new()` 需注入 `Arc<dyn AuditLogRepository>`

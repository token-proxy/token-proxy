# Token Proxy

企业级 LLM API 资源管理平台，提供统一的 API 代理、密钥管理、用量监控和访问控制能力。

> 架构详情见 [ARCHITECTURE.md](./ARCHITECTURE.md)

## 技术栈

- **后端**: Rust (edition 2021) + axum 0.8 + SeaORM 2 + tokio
- **前端**: React 19 + TypeScript + Vite + Semi Design 2.97
- **数据库**: PostgreSQL 17 (应用层按月分区管理)
- **代码质量**: Prettier (前端/JSON/MD)、lint-staged + simple-git-hooks (pre-commit 自动格式化)、cargo fmt/clippy
- **CI/CD**: GitHub Actions (fmt + clippy + build + PostgreSQL 集成测试)、Dependabot (每周依赖检查)
- **构建**: cargo-make + Docker 多阶段构建 (.dockerignore 优化构建上下文)
- **工具链**: Rust 工具链固定 (rust-toolchain.toml, channel = "1.96")

## 架构概要

DDD 四层：`domain/` → `application/` → `infrastructure/` → `presentation/`

| 层         | 路径                                                                            | 职责                             |
| ---------- | ------------------------------------------------------------------------------- | -------------------------------- |
| 领域层     | `src/domain/{access_point,provider,proxy,user,log,system,shared}/`              | 实体、值对象、Repository trait   |
| 应用层     | `src/application/{access_point,auth,dashboard,log,provider,proxy,system,user}/` | 用例编排、DTO                    |
| 基础设施层 | `src/infrastructure/{persistence,encryption,auth,http_client,parsers}/`         | Repository 实现、加密、JWT、HTTP |
| 展示层     | `src/presentation/{routes,middleware}/`                                         | axum handlers、认证中间件        |
| 共享       | `src/shared/`                                                                   | AppError (9 种)、PaginatedResult |
| 前端       | `src-dashboard/`                                                                | React SPA，构建产物嵌入二进制    |

- **依赖注入**: `Arc<dyn Trait>`，`main.rs` 组装；应用层 Service 注入 Repository trait，不直接依赖 SeaORM
- **聚合根**: `AccessPointEx` = 接入点 + 账户池 + 路由网格，ProxyPipeline 唯一交互入口
- **仓库命名**: 所有 Repository 实现以 `SeaOrm` 为前缀
- **实体 = ORM 实体**: domain 层直接使用 SeaORM DeriveEntityModel，聚合子目录内聚所有类型

## 关键决策（与编码直接相关）

- **接入 URL**: `/ap/<short_code>` — 用户指定或自动生成 16 位随机短码
- **JWT**: Access Token 30min + Refresh Token 7day
- **JWT 自动刷新**: 前端「请求前体检 + 401 兜底」双层防御，`REFRESH_THRESHOLD_SEC=300`；模块级 `refreshing` Promise 去重防止并发刷新互相吊销；不使用 `setTimeout`（浏览器冻结失效）
- **过期 refresh_token 清理**: tokio 后台任务 + `MissedTickBehavior::Skip`，复用 `PARTITION_CHECK_INTERVAL_SECS` 间隔；不引入 Redis 或 pg_cron
- **加密**: AES-256-GCM（`ENCRYPTION_KEY` 64 hex = 32 字节）
- **密码**: argon2id
- **分区**: `PartitionManager` 按月 RANGE 分区（`log_metadata`、`log_contents`），advisory lock 防冲突；`log_token_usage` 永久保留不分
- **代理 Header 构造**: 入站 `authorization` 仅用于用户 API key 认证；上游请求独立构建（API key 注入由 `AccessPointType::inject_api_key` 协议方法负责）；仅透传 `x-*`、`accept`、`content-type` 等业务头
- **响应头透明化**: 仅过滤 hop-by-hop 头（`transfer-encoding`、`connection`、`keep-alive`），其余透传
- **流式判断**: 由 `AccessPointType::is_sse_response(&resp_headers)` 判定，依据上游响应 `content-type` 是否包含 `text/event-stream`，非基于请求特征预设
- **协议适配点**: `AccessPointType` 枚举挂载 5 个协议方法（`parse_inbound` / `extract_session_id` / `inject_api_key` / `replace_model_in_body` / `is_sse_response`），具体实现位于 `domain/shared/protocols/<name>.rs`；加新协议只需补 enum variant + 新建协议文件，编译器会自动指出所有需要补 match 的位置
- **OpenAI 协议**: `OpenAi` variant 对应 `domain/shared/protocols/openai.rs`，内部按请求路径（`/chat/completions` vs `/responses`）通过 `remainder` 参数分发 `parse_inbound`；认证使用 `Authorization: Bearer <key>` 格式；Chat Completions 验证 `messages` 数组、Responses API 验证 `input` 字段；`is_sse_response` 依据 `Content-Type: text/event-stream` 判定（与 Anthropic 相同）
- **响应分类**: `UpstreamOutcome` enum 显式建模四种结果（`Success` / `ClientError` / `Fault` / `ServerError`），由 `UpstreamOutcome::classify` 调用 `FaultService::detect` 后归类；嵌套 if 树和重复的故障检测调用已被消除
- **重试决策**: `RetryDecision` enum (`Return(Response)` / `Continue(AppError)`) 由类型系统强制重试时必须携带错误原因，取代易遗漏的 `last_error + continue` 配对
- **响应体格式检测**: `detectResponseFormat(responseHeaders)` 通过 `Content-Type` 判定 `'sse'` / `'json'`；`isJsonFormat(body)` 通过 JSON.parse 试探兜底；前端各解析函数（`parseStructuredBlocks`、`detectHasThinking`、`buildConversationEvents` 等）接受可选 `format` 参数避免重复检测
- **客户端类型识别**: `ClientType` 枚举（ClaudeCode / Codex / Other / Unknown）与 `AccessPointType` 正交——同一 OpenAI 接入点可被 Claude Code 和 Codex 同时访问；`ClientType::from_request` 按品牌 header 优先 → UA 关键词 → 可识别特征 → Unknown 四级降级识别；`extract_session_id` 改为 `ClientType` 驱动（ClaudeCode → `x-claude-code-session-id`，Codex → `thread-id`），而非 `AccessPointType`
- **OpenAI Token 用量归一化**: `ParsedTokenUsage::from_response` 支持 Chat Completions（`usage.prompt_tokens` / `usage.completion_tokens` / `usage.total_tokens`）和 Responses API（`usage.input_tokens` / `usage.output_tokens` / `usage.total_tokens`），统一映射到现有 `log_token_usage` 列（`prompt_tokens` / `completion_tokens` / `total_tokens`）
- **账户池路由**: `RoutingStrategy` — Priority（同优先级排序，失败降级）或 Weighted（权重随机）；失败自动重试下一账号（由 `AccountSelector` 迭代候选 + `RetryDecision::Continue` 串联）
- **会话粘滞**: `session_affinity` 表（`access_point_id`, `session_id`），ProxyPipeline 首次创建、后续复用；写入通过 `TrackedSpawner` fire-and-forget
- **优雅关闭短路**: `ProxyPipeline::execute` 第 0 步检查 `shutting_down`，关闭期间新请求立即返回 `AppError::Upstream("服务正在关闭，请稍后重试")`
- **模型路由网格**: 二维表格（source_model × provider_id），匹配优先级：精确匹配 > 前缀匹配 > `__unmatched__` 兜底 > 原始模型值
- **账号自动禁用**: `DisabledReason`（Manual/RateLimited/BalanceExhausted/Fault）+ `available_at`；`recover()` 清除；禁用账号自动跳过
- **日志记录三阶段**: 元数据 → 内容 → token 用量；元数据失败立即 return，后续失败仅 warn/error 不阻断
- **日志记录器位置**: `ProxyCallRecord` 位于 `application/proxy/`（而非基础设施层），因为它直接接受领域聚合根（`InboundRequest` / `UpstreamRequest` / `AccessPointEx`）；API 反映业务时序：`start → attach_response → append_body/set_body → finish`，`Drop` 兜底 SSE 中断
- **日志默认不依赖 `log_contents`**: 列表优先用 `log_metadata`；原始内容按需加载（`/api/logs/{id}/raw`）
- **Dashboard 数据分析**: 4 个独立 GET 端点（`/api/dashboard/kpi` / `/api/dashboard/top-users` / `/api/dashboard/top-accounts` / `/api/dashboard/top-clients`）共享 `?range=today|last7|last30|custom` 时间窗口；KPI 端点内嵌 sparkline（避免重复 SQL）；对比窗口为等长前期；所有 LEFT JOIN 容忍 `users` / `accounts` / `providers` 删除（`Option<String>` + 前端降级为 `已删除成员/账号 · <uuid 前 8 位>`）；`DashboardService` 仅依赖 `LogRepository`，所有聚合在 SQL 层（无 N+1）；缓存命中率分母为 `input + cache_read`（不含 cache_creation）；趋势对比覆盖 5 种边界（up/down/flat/new/empty）
- **SSE 实时广播**: `LogService::record_proxy_log` 完成后通过 `tokio::sync::broadcast` channel 广播 `NewLogEvent`；channel 容量 256，满时丢弃最旧事件；前端通过 `EventSource` 消费 `GET /api/logs/events` 端点（JWT 通过 URL query 参数传递，因 EventSource 不支持自定义 header）；页面不可见时暂停处理，恢复可见时触发全量刷新；仅用于通知前端有新数据到达，不含敏感内容（request_body、response_body 等）

## Makefile 任务

| 命令                 | 说明                            |
| -------------------- | ------------------------------- |
| `cargo make dev`     | 并行启动前端 Vite HMR + 后端    |
| `cargo make build`   | 顺序构建前端 + 后端 release     |
| `cargo make check`   | 并行 cargo check + tsc --noEmit |
| `cargo make preview` | build 并运行 release 二进制     |
| `cargo make fmt`     | cargo fmt (Rust)                |
| `cargo make clippy`  | clippy (deny warnings)          |
| `cargo make test`    | cargo test                      |

### Pre-commit 自动格式化

- `simple-git-hooks` + `lint-staged` 管理 pre-commit hook，`npm run prepare` 初始化
- pre-commit 自动运行：`eslint --fix` + `prettier --write` 处理 `.ts/.tsx`、`prettier --write` 处理 `.json/.css/.md`、`cargo fmt` 处理 `.rs`
- 安装依赖后首次执行 `npm run prepare` 以激活 hook

## 环境变量

| 变量                          | 说明                   | 默认值 |
| ----------------------------- | ---------------------- | ------ |
| DATABASE_URL                  | PostgreSQL 连接串      | 必填   |
| JWT_SECRET                    | JWT 签名密钥           | 必填   |
| ENCRYPTION_KEY                | 64 hex chars (32 字节) | 必填   |
| SERVER_PORT                   | 监听端口               | 3000   |
| LOG_LEVEL                     | 日志级别               | info   |
| PARTITION_CHECK_INTERVAL_SECS | 分区检查间隔 (秒)      | 3600   |
| PARTITION_PREMAKE_MONTHS      | 提前创建未来分区月数   | 3      |
| PARTITION_RETENTION_MONTHS    | 分区保留月数           | 12     |

## 发布流程

### 版本号策略

- **main 分支**: 版本号为占位 `0.0.0`，不设置实际版本，只在 release 分支设定
- **Release 分支**: 仅在 release 分支上设置实际版本号（如 `0.1.0`、`0.2.0-rc.1`）
- **Git tag 格式**: 无 `v` 前缀（`0.1.0`、`0.2.0-rc.1`）

### 发布步骤

1. 从 main 创建 release 分支
2. 提交 A：运行 `cliff generate` 生成 CHANGELOG，仅提交 CHANGELOG.md
3. 提交 B：将 `Cargo.toml` 版本号更新为目标版本，tag 打在本次提交上
4. 合并 release 分支（使用 rebase 策略，无 merge commit）
5. 将提交 A（仅 CHANGELOG）cherry-pick 回 main 分支

### CHANGELOG 管理

- 使用 `git-cliff` 自动生成，配置见 `cliff.toml`
- 分类映射：`feat` → Added、`fix` → Fixed、`perf`/`refactor` → Changed
- 按发布日期倒序排列

### 发布技能

- 使用 `/release <version> [<description>]` 命令执行发布流程
- 技能文件位于 `.claude/skills/release/SKILL.md`

## 前端路由

```
/login → LoginPage
/ → AdminLayout → /dashboard (DashboardPage)
/providers (ProviderManagement)
/access-points (AccessPointManagement)
/sessions → /sessions/:sessionId (SessionLogPage)
/logs → /logs/:id (LogDetailPage)
/users (UserManagement)
/settings (SettingsPage)
/profile (ProfilePage)
```

管理侧边栏: Dashboard, 服务商管理, 接入点管理, 会话日志, 请求日志, 用户管理, 系统设置

- **DashboardPage**: Linear Insights / Vercel Analytics 极简风格数据分析页，含 KPI 卡片、缓存命中率卡、Top Users / Top Accounts / Top Clients 排行；顶层组件管理 `timeRange` + `refreshKey`，调度 4 个独立 `useFetch`；响应式 Grid 布局（断点 1280 / 768），暗色优先双主题适配；零 Mock 数据，所有同比箭头基于真实查询

## 编码规范

### 注释规范

每次编写或修改代码时必须遵循。以下规则基于对项目 ~220 个源文件的全面评估提炼。

**三必写：**

- **模块入口必写**: 非平凡 `.rs` 文件必须以 `//!` 模块文档开头（说明所属层级、聚合、主要类型）。例外：仅含 `pub mod` 的 barrel 文件
- **公开 API 必写**: 每个 `pub` struct/enum/trait/fn/method 必须有 `///` 文档。例外：自解释的简单访问器（如 `created_at_utc`）
- **类型契约必写**: DTO struct 必须有文档说明用途；关键字段（非自解释、有校验/业务语义）必须有行内文档；前端 `types/` 下每个 interface 必须有 JSDoc

**一不写：**

- **简单代码不写冗余注释**: 不重复代码语义（如 `// 循环数组`）；不记录变更历史（"新增""修改""移除"属于 Git）；自解释访问器可不写

**行内注释规则：**

- **复杂逻辑详写**: 3 步以上的算法用编号行内注释（`// 1. 精确匹配` `// 2. 前缀匹配` ...）
- **文件结构分隔**: 功能区域用 `// ─── 领域行为 ───`（Rust）或 `// --- SSE 解析 ---`（TypeScript）
- **注释说"为什么"不说"是什么"**: 解释设计意图、边界条件、非显而易见的 hack/workaround

**语言和格式：**

- 所有注释使用中文；技术标识符（类型名、方法名）保持英文
- 中文与英文/数字之间必须保留空格：`接入点 '{}' 未找到`
- 错误消息使用中文，日志字段使用英文

**前端 JSDoc：**

- 导出函数/组件必须有 JSDoc；hook 返回值必须文档化
- 组件 props 接口必须文档化（至少接口级 JSDoc + 非自解释字段）
- 复杂解析算法（如 `parseStructuredBlocks`）需要文件级 JSDoc + 步骤编号注释

**项目中的注释标杆（参考）：**
`src/domain/shared/protocols/anthropic.rs`、`src/domain/shared/protocols/openai.rs`、`src/domain/log/metadata.rs`、`src/application/log/dto/proxy_log_input.rs`、`src/application/proxy/proxy_pipeline.rs`、`src/application/proxy/proxy_call_record.rs`、`src/main.rs`、`src-dashboard/utils/parseLogs.ts`、`src-dashboard/components/session/TurnCard.tsx`

### 通用编码约束

- `AppError` 9 种变体: Validation(400) / NotFound(404) / Conflict(409) / Unauthorized(401) / Forbidden(403) / Encryption(500) / Database(500) / Upstream(502) / Internal(500)
- `log_metadata` 分区表 PRIMARY KEY 必须包含 `timestamp`
- `api_type` 新增类型需同步修改: Rust 枚举 + 数据库列约束 + 前端 Select（目前已有 anthropic、openai）
- `DisabledReason` 新增原因需同步修改: Rust 枚举 + 数据库列约束 + 前端展示
- `.rs` 空文件留作占位用，不应删除
- 前端路径别名 `@components` → `src-dashboard/components/`，引用不带 `.tsx` 后缀
- 前端所有异步按钮设置 `loading`/`disabled` 防重复点击；列表使用 `operatingId` 行级锁定
- 前端 Modal 表单：`footer` 承载取消/确认按钮，确认按钮通过 `getFormApi` 保存的 `formApi.submitForm()` 触发
- 前端改密成功后清除所有 localStorage 令牌并跳转 `/login`
- 前端主题：`useTheme` hook + `ThemeProvider`，localStorage key `theme_mode`，支持 light/dark/system
- 前端数据获取优先使用 `useFetch` Hook（`src-dashboard/hooks/useFetch.ts`）：`loading` 初始为 `true`，返回 `{ data, loading, error, refetch }`，deps 通过 `useMemo` 传入；所有 `setState` 在异步回调中执行，卸载后不再更新状态
- 前端类型断言优先使用 `satisfies` 关键字替代 `as`
- `AdminLayout` 侧边栏：`selectedKeys` 通过 `useMemo` 派生；`isCollapsed` 拆分为用户控制（`userCollapsed`）+ 自动（`isDetailPage`）
- `AccessPointDrawer` 中 `rowSelectedProviders` 通过 `useMemo` 从 `formData.accounts` 和 `allKnownAccounts` 派生

### 日志规范

每次编写或修改代码时必须遵循。以下规则基于对项目日志实践的全面评估提炼。

**日志框架：** 仅使用 `tracing` 宏（`info!`、`warn!`、`error!`、`debug!`）。禁止在库代码中使用 `println!`/`eprintln!`（CLI 迁移命令除外）。日志级别由 `LOG_LEVEL` 环境变量控制（`trace` / `debug` / `info` / `warn` / `error`），默认 `info`。生产环境输出 JSON 格式。

**级别使用规则：**

| 级别     | 使用场景                                                                                     |
| -------- | -------------------------------------------------------------------------------------------- |
| `error!` | 不可恢复的错误，需要人工介入（数据库连接失败、加密失败、分区维护失败）                       |
| `warn!`  | 可恢复的异常，自动降级或重试成功（账号禁用、会话保存失败、token 解析失败、审计日志写入失败） |
| `info!`  | 关键业务事件和生命周期（启动/关闭、请求到达/完成、账号池选择、分区创建/清理、模型发现结果）  |
| `debug!` | 诊断细节，生产环境默认关闭（具体账号选择过程、URL 构造、请求变换前后对比）                   |
| `trace!` | 极细粒度调试（逐 chunk SSE 转发、JSON 解析中间态）                                           |

**结构化字段（强制）：**

- 所有日志必须使用结构化字段格式 `field = %value`，禁止纯字符串插值 `"xxx: {}", val`
- 原因：JSON 输出下结构化字段成为独立 JSON key，可在日志聚合器中直接过滤和聚合；字符串插值的内容被嵌入 `message` 字段，不可检索
- 正例：`tracing::info!(account_id = %id, short_code = %code, "开始代理转发")`
- 反例：`tracing::info!("开始代理转发: account={}, code={}", id, code)`

**关键路径必须记录：**

代理管道（`proxy_pipeline.rs`）是核心业务链路，必须记录以下事件（至少 `info!` 级别）：

- 请求到达：short_code、session_id、request_id、请求模型
- 接入点加载结果：access_point_id、账户池大小
- 每次账号尝试：account_id、provider_id、尝试序号
- 账号跳过原因（debug!）：is_available 为 false 时记录 disabled_reason
- 会话粘滞命中/未命中（debug!）
- 上游请求发出（debug!）：URL、模型、Provider
- 请求完成：状态码、耗时（ms）、使用的 account_id
- 重试决策：哪些状态码触发重试、剩余可用账号数
- 所有账号耗尽：最终错误信息
- 账号自动禁用：account_id、status、reason

**严禁记录（敏感信息防护）：**

以下数据**绝对不能**出现在日志中（包括 tracing、TraceLayer、ProxyCallRecord、数据库 audit_log 的 details 字段）：

- API 账号密钥明文（`accounts.encrypted_key` 解密后的值）
- 用户 API key 完整值（创建时返回一次的 `tp_*` key）
- JWT token 值（access_token、refresh_token）
- 用户密码（明文或哈希）
- 入站 `Authorization` 请求头值
- 上游 `Authorization: Bearer <key>` 请求头值
- `Cookie` / `Set-Cookie` 头值
- `ENCRYPTION_KEY` / `JWT_SECRET` 环境变量值

`headers_to_json()` 和 `response_headers_to_json()` 已实现自动脱敏（`[REDACTED]`），写入 `log_contents` 时安全。`TraceLayer` 仅记录 method/uri/status/latency/request_id，不记录 header 和 body。

**前端日志：**

- API 调用失败时必须在 `api.ts` 的 `request()` 函数中输出 `console.error`：至少包含 HTTP 方法、URL、状态码、错误消息
- 禁止输出 token、API key、密码到浏览器控制台
- 禁止使用空的 `catch {}` 块吞掉错误——至少输出 `console.warn`
- 前端提取响应中的 `X-Request-ID` 或 `request_id` 头，在错误 Toast 中展示以便关联后端日志
- 考虑在 `AdminLayout` 中添加 React 错误边界，渲染崩溃时输出 `console.error` 并展示恢复 UI

**`#[instrument]` 属性：**

- 所有应用层 Service 的公开方法建议添加 `#[tracing::instrument(skip_all, fields(...))]` 以实现自动 span 生命周期
- `skip_all` 防止敏感参数被自动记录
- 通过 `fields` 显式声明需要记录的关键字段

**检查点（编写代码后自查）：**

- [ ] 新增的 `pub async fn` 是否有 entry 日志或 `#[instrument]`？
- [ ] 日志是否使用结构化字段格式 `field = %value`？
- [ ] 日志级别是否合适（error 仅不可恢复、warn 可恢复、info 关键事件）？
- [ ] 是否有敏感数据可能被日志记录？
- [ ] 前端 API 错误是否有 `console.error` 输出？
- [ ] 是否有空的 `catch {}` 块需要补充 `console.warn`？

## 设计原则

以下原则来自实际重构中的错误，适用于任何涉及职责分配的场景。
完整的 DDD 实践规范（贫血检测、逻辑归属决策框架、反模式清单）见 project memory `project-ddd-practice-specification`。

### 核心纪律：应用层编排，领域层决策

- **应用层**（`*Service` / `ProxyPipeline`）：知道"先做什么、后做什么"——加载、调度、保存。不包含业务判断。
- **领域层**（实体 / 值对象 / 领域服务）：知道"怎样判断、怎样计算"——验证、匹配、策略选择。不接触基础设施。

### 贫血检测四信号

1. **枚举有 variant 但行为在别处** → 行为移入枚举 impl
2. **结构体公开字段被外部逐字段赋值** → 封装为实体行为方法
3. **自由函数接受领域类型返回领域结果但放在应用层** → 移到对应类型上或领域服务中
4. **应用层直接修改聚合根内部集合** → 封装为聚合根方法

### 参数所有权测试

判断方法是否在正确对象上：看**所有参数是否都属于该对象的概念范围**。如果一个参数放进构造函数里会觉得别扭，那它就不该出现在方法签名里。

### 聚合根作为行为入口

调用方不穿透聚合根访问内部值对象的方法。聚合根对外暴露行为，内部委托给值对象：

```
// 错误：调用方穿透了两层
aggr.inner_collection.resolve(x, y)

// 正确：调用方只对聚合根发问
aggr.resolve(x, y)
```

### "行为靠近数据"的边界

这条原则只适用于**数据是自己、参数也只涉及自己**的场景。一旦方法签名引入了不属于该类型的概念（如来自另一个聚合的字段），行为应移到能容纳所有参数的最小子系统（通常是上一层聚合根或领域服务）。

### 逻辑归属决策速查

| 问题                            | 归属                                                   |
| ------------------------------- | ------------------------------------------------------ |
| 操作单个实体自身字段？          | 实体方法（如 `Account::disable_for`）                  |
| 操作值对象数据 + 外部数据？     | 值对象方法（如 `RecoverType::calculate_available_at`） |
| 协调两个或多个领域对象？        | 领域服务（如 `FaultService::detect`）                  |
| 涉及 Repository / HTTP / 加密？ | 应用服务（如 `ProxyPipeline`）                         |
| 纯粹 HTTP / 序列化 / 外部 API？ | 基础设施层（如 `ProxyClient`）                         |

## 核心文件速查

| 文件                                                         | 说明                                                                                                                            |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| `src/main.rs`                                                | 启动入口 (依赖组装 + 路由 + 分区 + 后台任务)                                                                                    |
| `src/application/proxy/proxy_pipeline.rs`                    | 代理转发管道 (60 行调度骨架 + try_one_account 子方法，领域逻辑在 domain/)                                                       |
| `src/application/proxy/proxy_call_record.rs`                 | 代理调用记录器 (start → attach_response → append/set_body → finish; Drop 兜底中断标记)                                          |
| `src/application/proxy/tracked_spawner.rs`                   | 后台写入调度器 (统一 fetch_add + try_current 守卫 + spawn + fetch_sub 模板)                                                     |
| `src/application/proxy/account_selector.rs`                  | 候选账号迭代器 (封装加载 Account → 跳过 → 加载 Provider → 解密 API Key 四步)                                                    |
| `src/application/proxy/upstream_dispatcher.rs`               | 上游转发执行器 (forward + 120s 非流式响应体读取超时)                                                                            |
| `src/application/proxy/response_builder.rs`                  | axum 响应构造 (build_streaming_response / build_buffered_response + hop-by-hop 过滤)                                            |
| `src/domain/proxy/upstream_outcome.rs`                       | 上游响应分类枚举 (Success / ClientError / Fault / ServerError) + classify 函数                                                  |
| `src/domain/proxy/retry_decision.rs`                         | 重试决策枚举 (Return(Response) / Continue(AppError))                                                                            |
| `src/domain/provider/fault_config.rs`                        | 故障配置值对象 (matches_status + calculate_available_at + extract)                                                              |
| `src/domain/provider/fault_service.rs`                       | 故障检测领域服务 (FaultService::detect + disable_account)                                                                       |
| `src/domain/access_point/access_point.rs`                    | AccessPointEx 聚合根 (sort_accounts + apply_session_affinity + build_upstream_request)                                          |
| `src/domain/access_point/routing_strategy.rs`                | 路由策略值对象 (sort_accounts)                                                                                                  |
| `src/domain/shared/api_type.rs`                              | AccessPointType 枚举 + 协议方法 (parse_inbound / extract_session_id / inject_api_key / replace_model_in_body / is_sse_response) |
| `src/domain/shared/inbound_request.rs`                       | 入站请求纯数据结构 (InboundRequest struct，无方法)                                                                              |
| `src/domain/shared/upstream_request.rs`                      | 上游请求纯数据结构 (UpstreamRequest struct，无方法)                                                                             |
| `src/domain/shared/protocols/anthropic.rs`                   | Anthropic 协议适配实现 (parse_inbound/extract_session_id/inject_api_key 等)                                                     |
| `src/domain/shared/protocols/openai.rs`                      | OpenAI 协议适配实现 (Chat Completions + Responses API 双端点，5 个协议方法)                                                     |
| `src/domain/shared/client_type.rs`                           | 客户端类型枚举 (ClaudeCode/Codex/Other/Unknown) + from_request 识别 + extract_session_id                                        |
| `src/application/log/log_service.rs`                         | 日志写入/查询 (三阶段)                                                                                                          |
| `src/application/log/dto/proxy_log_input.rs`                 | 代理日志写入入参 DTO (一次性构造，无占位值)                                                                                     |
| `src/application/dashboard/dashboard_service.rs`             | Dashboard 聚合服务 (get_kpi + get_top_users + get_top_accounts + get_top_clients + compute_trend)                               |
| `src/application/dashboard/time_window.rs`                   | 时间窗口解析 (resolve_windows 纯函数，含等长前期对比)                                                                           |
| `src/domain/log/dashboard_query.rs`                          | Dashboard 领域查询类型 (DashboardWindow + KpiAggregate + SparklineBucket + Top\*Row)                                            |
| `src/presentation/routes/dashboard_routes.rs`                | Dashboard 路由 (/api/dashboard/{kpi,top-users,top-accounts,top-clients})                                                        |
| `src/application/user/api_key_service.rs`                    | 用户 API key 管理                                                                                                               |
| `src/presentation/middleware/jwt_auth.rs`                    | JWT 认证中间件 + CurrentUser                                                                                                    |
| `src/presentation/middleware/user_api_key_auth.rs`           | 用户 API key 认证                                                                                                               |
| `src/infrastructure/persistence/partition_manager.rs`        | 分区管理                                                                                                                        |
| `src-dashboard/api.ts`                                       | 前端 API 封装 (JWT 自动刷新)                                                                                                    |
| `src-dashboard/components/access-point/modelMappingUtils.ts` | 模型映射工具 (ANTHROPIC_FAMILIES, MappingMatchType, matchTypeForSource)                                                         |
| `src-dashboard/components/log/log-detail/tokenUsage.ts`      | Token 用量工具函数 (hasTokenData)                                                                                               |
| `src-dashboard/components/session/TurnCard.tsx`              | 轮次卡片组件 (递归渲染内容块, 最大深度 5 层)                                                                                    |
| `src-dashboard/components/session/TurnNavigator.tsx`         | Sticky 轮次导航条                                                                                                               |
| `src-dashboard/hooks/useFetch.ts`                            | 通用数据获取 Hook (fetch-on-mount, {data, loading, error, refetch})                                                             |
| `src-dashboard/pages/DashboardPage.tsx`                      | Dashboard 顶层 (timeRange + refreshKey + 4 个 useFetch 调度)                                                                    |
| `src-dashboard/components/dashboard/`                        | Dashboard 组件 (KpiCard/CacheHitCard/Sparkline/ComparisonArrow/StackedBar/Top\*Ranking)                                         |
| `src-dashboard/utils/parseLogs.ts`                           | 日志/会话解析工具 (SSE + JSON 双格式, buildConversationEvents + buildConversationTurns)                                         |
| `src-dashboard/utils/parseOpenAI.ts`                         | OpenAI 响应/请求体解析工具 (Chat Completions + Responses API 双格式，SSE + 非流式)                                              |
| `src/application/log/dto/new_log_event.rs`                   | SSE 广播事件 DTO (NewLogEvent，不含敏感数据)                                                                                    |
| `src/presentation/routes/log_routes.rs`                      | 日志路由 (`GET /api/logs/events` SSE 端点 + `/api/logs` CRUD)                                                                   |
| `src-dashboard/hooks/useLogEvents.ts`                        | SSE 实时日志事件消费 Hook (EventSource 连接管理、JWT 传递、自动重连、页面可见性暂停/恢复)                                       |
| `src-dashboard/components/common/ConnectionIndicator.tsx`    | SSE 连接状态指示器 (彩色圆点 + 文字标签)                                                                                        |
| `cliff.toml`                                                 | CHANGELOG 自动生成配置 (git-cliff, feat→Added / fix→Fixed / perf→Changed)                                                       |
| `rust-toolchain.toml`                                        | Rust 工具链版本固定 (channel = "1.96")                                                                                          |
| `.prettierrc` / `.prettierignore`                            | Prettier 格式化配置与排除规则                                                                                                   |
| `.dockerignore`                                              | Docker 构建上下文排除规则                                                                                                       |
| `.github/workflows/ci.yml`                                   | CI 流水线 (fmt + clippy + build + PostgreSQL 集成测试)                                                                          |
| `.github/dependabot.yml`                                     | 依赖自动更新配置 (cargo + npm 每周)                                                                                             |
| `.claude/skills/release/SKILL.md`                            | 发布管理技能 (/release 命令)                                                                                                    |

## 注意事项（易错点）

- 迁移文件在 `src/migrations/` 下，使用 `sea-orm-migration`
- `domain/` 按聚合组织（7 个子目录：access_point/log/provider/proxy/shared/system/user），不再使用 entities/value_objects/repositories/services 技术类别目录
- 跨聚合共享类型放在 `domain/shared/`，不要放在单个聚合内
- `model_routing_grid` 的 `__unmatched__` 行是兜底规则，每个接入点自动生成
- AccessPointDrawer 保存时过滤 `provider_ids` 必须属于账户池中的 Provider
- 会话粘滞由 ProxyPipeline 自动管理，不感知前端
- `ProxyCallRecord` 持有 `ProxyLogInput` 入参（仅用作 LogService 一次性入参，不再是跨层 DTO），按业务时序累积日志数据；`LogService::record_proxy_log()` 内部构造 `LogMetadata`
- `ProxyCallRecord` 的 `Drop` 兜底是 SSE 客户端中断时唯一可靠的落库机制，不要去除
- 所有 fire-and-forget 后台写入（代理日志 + 会话粘滞）必须通过 `TrackedSpawner::spawn(operation, future)` 入队，禁止裸 `tokio::spawn`（会绕过 `in_flight_writes` 计数和 `try_current` 守卫）
- 协议适配方法（`parse_inbound` / `extract_session_id` / `inject_api_key` / `replace_model_in_body` / `is_sse_response`）挂在 `AccessPointType` 枚举上，每协议实现位于 `domain/shared/protocols/<name>.rs`；新增协议时编译器会自动指出所有需要补 match 的位置
- `ClientType` 与 `AccessPointType` 正交：同一 OpenAI 接入点可被 Claude Code 和 Codex 同时访问；`ClientType::from_request` 在 ProxyPipeline 中调用，结果存入 `InboundRequest.client_type` 字段并沿日志链路传递（ProxyLogInput → log_metadata.client_type → log_token_usage.client_type）
- `session_id` 解析由 `ClientType::extract_session_id` 驱动（ClaudeCode → `x-claude-code-session-id`，Codex → `thread-id`），而非 `AccessPointType`；`AccessPointType::extract_session_id` 仅作为协议层兜底（如 OpenAi → `thread-id`），ProxyPipeline 中优先调用 `ClientType::extract_session_id`
- OpenAI Token 用量解析在 `ParsedTokenUsage::from_response` 中按 `api_type` 分发：Chat Completions 读 `usage.prompt_tokens`/`completion_tokens`/`total_tokens`，Responses API 读 `usage.input_tokens`/`output_tokens`/`total_tokens`，统一归一化到现有 `log_token_usage` 列
- 前端按 `api_type` 顶层分发渲染：Anthropic 走 `parseLogs.ts`（`parseStructuredBlocks` 等），OpenAI 走 `parseOpenAI.ts`（`parseOpenAIChatResponse` / `parseOpenAIResponsesResponse` 等）；`ResponseContentCard` 和 `RequestContentCard` 根据日志的 api_type 选择对应解析器
- `session_id` 在请求路径上是 `Option<String>` 类型（`None` 表示请求未携带会话标识），禁止再使用字符串 `"unknown"` 作为 sentinel；只有写入 `log_metadata.session_id`（NOT NULL 列）时才回落为 `"unknown"` 字符串
- `UpstreamOutcome::classify` 是响应分类的唯一入口；SSE 错误路径传 `resp_body=None`，body-based 故障规则会被静默忽略（doc 注释已明确）
- `RetryDecision::Continue(AppError)` 强制重试时必须携带错误原因，类型系统替代了过去 `last_error = Some(...); continue;` 的配对约束
- `log_metadata` 的 `account_id` 字段记录实际使用的账号
- 会话详情页轮次判定：通过 `request_body.messages` 数组判定（非 tool_result 的用户消息 = 新轮次起点），`buildConversationTurns()` 在 `parseLogs.ts` 中实现；所有改进纯前端实施，不修改后端 API；不要使用 `buildConversationEvents()` 渲染详情页
- 响应体格式检测（`detectResponseFormat`）优先通过 `response_headers` 的 `Content-Type` 判定；若无响应头或未匹配，`isJsonFormat` 通过 JSON.parse 试探兜底；`ResponseContentCard` 将检测结果通过 `format` 参数传递给各解析函数
- Dashboard 聚合查询全部走 `LogRepository` 的 `aggregate_kpi` / `aggregate_sparkline` / `top_users` / `top_accounts` / `top_clients` 五个方法，禁止在 `LogService` 中重新实现统计逻辑（已删除旧的 `get_overview_stats` / `get_trends` / `top_access_points` / `top_models`）；新增 Dashboard 指标时优先扩展现有五个聚合方法
- Dashboard sparkline 空桶补齐由 SQL 端 `generate_series` 完成，应用层无需再补；新增时间粒度（如小时级）需同步扩展 `DashboardWindow` 与 SQL 系列生成步长
- 前端 Dashboard 已删除成员/账号统一用全局 `.dashboard-deleted` CSS class（灰色 + monospace）展示，不要单独写 `style={{ color: ... }}`
- 性能优化遗留：`log_token_usage.timestamp` 无前导索引，`top_accounts` 查询可能 Seq Scan；建议在数据量增长后追加 BRIN 索引（未在本次实施）
- SSE 广播通过 `NewLogEvent` DTO 传递，仅含标识信息（log_id / timestamp / session_id / api_type / user_id / access_point_id），不含请求体/响应体/请求头等敏感数据
- `GET /api/logs/events` 端点的 JWT 认证通过 URL query 参数 `?token=` 传递（EventSource API 不支持自定义 header），中间件需兼容 query 参数提取
- SSE channel (`broadcast::channel(256)`) 容量满时丢弃最旧事件（`send` 返回 `Lagged` 错误时仅记录 warn 日志），前端通过 `onVisibilityRecover` 全量刷新弥补丢失事件
- `useLogEvents` Hook 在页面隐藏时暂停处理事件（节省资源、避免积压），恢复可见时不逐条消费积压事件，而是触发一次全量刷新；依赖 `visibilityCallbackRef` 注册回调
- 前端 `useFetch` 的 `refetch` 方法现已自动设置 `loading=true`，因此 SSE 事件触发的 refetch 不再需要在调用方额外手动设置 loading 状态

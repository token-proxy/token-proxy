# 代理协议领域建模与 SDK 集成决策记录

> 本文记录一次针对「转发功能」的领域重构：引入请求级协议值对象 `ApiProtocol`、
> 新增 OpenAI Responses 协议支持、并集成成熟 SDK `async-openai`。
>
> 结论先行：**协议领域建模是本次改造的核心收益；SDK 被有选择地用于类型化解析，
> 而非替换字节级转发通路**——原因见第 4 节（已用实验证伪「SDK 全量接管转发」）。

## 1. 问题

改造前，协议知识集中在 `AccessPointType` 枚举上的 5 个协议方法
（`parse_inbound` / `extract_session_id` / `inject_api_key` /
`replace_model_in_body` / `is_sse_response`）。存在三个具体问题：

1. **概念混淆**：`AccessPointType` 同时承担两个正交职责——
   「接入点属于哪个协议家族」（实体字段，落库 + 前端 Select）与
   「这次请求走哪个上游协议」（请求级事实）。前者是配置，后者是运行时输入。
2. **隐式协议判定**：OpenAI 的 Chat Completions 与 Responses 两条线协议
   只能靠 `remainder.contains("responses")` 猜测，且判定结果不落日志，
   事后无法区分一条日志到底走的哪条协议。
3. **不可扩展的协议边界**：新增协议要改 `AccessPointType`（进而牵动数据库列约束
   与前端 Select），而 Responses 明明和 Chat 共用同一个接入点与 base_url。

## 2. 领域建模：两个正交概念

| 概念              | 回答的问题                 | 形态                                      | 生存周期           |
| ----------------- | -------------------------- | ----------------------------------------- | ------------------ |
| `AccessPointType` | 接入点属于哪个协议家族？   | 实体枚举（落库 VARCHAR + 前端）           | 配置级、持久       |
| `ApiProtocol`     | 这一次请求走哪个上游协议？ | 值对象（`domain/shared/api_protocol.rs`） | 请求级、运行期推导 |

`ApiProtocol` 的 3 个变体对应三套互不相同的线协议：

| 变体             | 线协议                  | 端点                   | 请求体判别字段         |
| ---------------- | ----------------------- | ---------------------- | ---------------------- |
| `Anthropic`      | Anthropic Messages      | `/v1/messages`         | `messages`             |
| `OpenAi`         | OpenAI Chat Completions | `/v1/chat/completions` | `messages`             |
| `OpenAiResponse` | OpenAI Responses        | `/v1/responses`        | `input`（数组/字符串） |

### 为什么**不**给 Responses 新增接入点类型

用户提出过这个疑问。答案是：**不需要，而且不应该**。

- Chat Completions 与 Responses 共用同一个 Provider base_url、
  同一套 `Authorization: Bearer` 认证、同一个账号池与路由网格。
  它们是**同一个接入点的两种调用方式**，把它们拆成两个接入点会强迫运维
  重复配置账号池、模型路由网格与配额，且无法在两者间共享会话粘滞。
- 若新增 `AccessPointType::OpenAiResponse`，前端 Select 与数据库约束都要跟着膨胀，
  但**没有新增任何可配置维度**——协议其实完全由入站路径决定，是推导量而非配置量。
- 因此协议由 `AccessPointType::resolve_protocol(remainder)` 在**每次请求**推导，
  再由 `allows()` 做家族守卫（`anthropic` 接入点收到 `/v1/responses` 会被明确拒绝，
  而不是把 Anthropic 请求体发去 OpenAI 端点）。

`AccessPointType::allowed_protocols()` 因此是：

```text
Anthropic → [ApiProtocol::Anthropic]
OpenAi    → [ApiProtocol::OpenAi, ApiProtocol::OpenAiResponse]
```

### 行为归属：从「枚举 + 散落函数」到「值对象自治」

`ApiProtocol` 现在拥有全部 5 个协议行为，内部 `match self` 分发到
`protocols/{anthropic,openai}.rs`。这满足领域贫血检测信号 1
（枚举有 variant 但行为在别处 → 行为移入枚举 impl）与信号 3
（自由函数接受领域类型返回领域结果但放在应用层 → 移到对应类型上）。

`AccessPointEx::build_upstream_request` 不再自行推导协议，直接复用
`inbound.protocol`——**同一事实只判定一次**，避免"路径推导"与"聚合根推导"
产生分歧。

## 3. 日志：区分「配置」与「事实」

新增 `log_requests.api_protocol VARCHAR(32) NOT NULL DEFAULT 'anthropic'`，
与既有 `api_type` 并存：

| 列             | 语义             | 取值                                       |
| -------------- | ---------------- | ------------------------------------------ |
| `api_type`     | 接入点协议家族   | `anthropic` / `openai`                     |
| `api_protocol` | 本次请求实际协议 | `anthropic` / `openai` / `openai_response` |

保留 `api_type` 而非直接改写其语义，是为了让存量日志的列含义保持稳定
（旧数据中 `api_type='openai'` 无法反推是 Chat 还是 Responses，
迁移把这类行的 `api_protocol` 保守回填为 `openai`，不猜测历史端点）。

## 4. SDK 集成：为什么只做「类型化解析」而非「替换转发通路」

引入 `async-openai 0.41.1`（`default-features = false`，
features = `["chat-completion", "responses"]`，本地 registry 缓存可离线构建）。

### 实验证据（决定了集成边界）

在决定由 SDK 接管转发前，做了五组实验：

| 实验                                                            | 结果                                          |
| --------------------------------------------------------------- | --------------------------------------------- |
| SDK 请求类型反序列化含未知顶层字段后重新序列化                  | **未知字段被丢弃**                            |
| Responses `input` 含 SDK 未建模的 `type`（联合体新成员）        | **反序列化直接失败**                          |
| SDK 原始请求原语（`post` / `post_raw` / `post_stream`）是否可用 | **均为 `pub(crate)`，crate 外不可用**         |
| `byot` 泛型方法承载原始 `serde_json::Value` 请求体              | **未知字段 + 新 role + 键序全部原样送达**     |
| `byot` 流式（`create_stream_byot`）的事件产出形态               | **产出已解析 JSON 事件值，原始 SSE 分帧丢失** |

> 注：`async_openai::RequestOptions` 本身是公开类型，但其 `with_headers` /
> `with_path` / `with_query` 构造方法同样是 `pub(crate)`，且类型化 API 不接受外部
> 预构造的 `RequestOptions`——因此公开的 `RequestOptions` 并不能用于自定义转发。

结论：**类型化 SDK 无法承担字节级透明转发**。代理的核心价值恰恰是
对上游新增字段、新 role、新联合体成员的**前向兼容**；而 OpenAI 类型化 SDK
天然滞后于 API 演进，用它中转请求会引入"客户端更新快于 SDK 就断链"的脆弱性。

### 关于 `byot`：可行性已证实，但默认不启用

启用 `byot` feature 后，SDK 的 `*_byot` 泛型方法可以接受任意 `Serialize` 输入，
从而**绕过类型化往返丢字段的问题**：把入站请求体原样以 `serde_json::Value` 传入，
实测未知字段、`developer` role、键序全部原样送达上游。
这意味着 **SDK 驱动的转发在技术上可行**，此前"完全不可行"的判断需要修正。

但它仍不足以作为**默认**转发通路，卡点在流式路径：

- `create_stream_byot` 产出的是**已解析的 JSON 事件值**（每行 `data:` 解析结果），
  原始 SSE 分帧（`event:` 行、`[DONE]` 终止符、空行边界）**不再保留**。
- 若用 SDK 转发流式响应，代理必须自行按协议**重新拼装 SSE 分帧**。流式是
  Claude Code / Codex 的绝对主路径，一旦拼装与原上游存在差异，会以
  「客户端静默卡住 / 事件丢失」形式故障——这是代理最难排查、影响最大的失效模式。
- 仅替换非流式虽可忠实（请求 `byot` 原样、响应 `Value` 重序列化），
  但会造成「同一协议两套传输」的分裂。

因此当前取舍：**默认保持 `reqwest` 字节级透传**（流式/非流式一致、语义零损失），
`byot` 作为**已证实的可选演进路径**记录在案。若未来要由 SDK 接管转发，推荐：

1. 仅对非流式启用 `byot`（零风险、可测试），流式继续字节透传；或
2. 实现按协议的 SSE 重分帧，并配以金标准对照测试（录制真实上游 SSE +
   断言重分帧等价）；或
3. 以接入点/服务商级开关灰度，默认关闭。

### 因此的集成方式

SDK 用在其类型确实**更正确、更安全**的位置：

1. **Responses usage 类型化归一化**（`parsed_token_usage.rs`）：
   用 `async_openai::types::responses::ResponseUsage` 解析，
   正确拆出 `input_tokens_details.cached_tokens` → `cache_read_input_tokens`。
   改造前该字段被硬编码为 `0`，缓存命中输入被记为全价输入，**是真实的数据缺陷**。
   类型化解析失败时回退原始 JSON 提取，保证日志能力不因 SDK 滞后而退化。
2. **Responses 请求结构探测**（`protocols/openai.rs`）：
   用 `CreateResponse` 做一次 best-effort 结构校验，**失败仅记 debug、不阻断转发**。
   借 SDK 类型系统发现手工校验覆盖不到的异常，同时不牺牲透明性。

转发通路仍由 `reqwest` 承担（`ProxyClient` → `UpstreamDispatcher`）。
`reqwest` 本身即是成熟且被广泛使用的 HTTP SDK，且被收敛在基础设施层单一适配器内——
这才是"转发功能"真正的可替换接缝。

### 顺带修复的真实缺陷：Responses 非流式词元用量被静默归零

领域建模完成后暴露出的第二个缺陷，根因是**用字段形状猜测协议**：

`Anthropic` 与 `OpenAI Responses` 的**非流式**响应都使用顶层 `usage.input_tokens`，
结构上无法区分；而 Responses 的响应体是 `{ id, object, usage }`，顶层没有 `usage`。
于是全协议探测链的走向是：

1. `parse_sse_usage` → 不匹配
2. `parse_non_streaming_usage`（Anthropic）→ 守卫命中 `output_tokens_details` 而跳过
3. `extract_openai_chat_usage` → 找到 `usage` 对象，但按 `prompt_tokens` / `completion_tokens`
   读字段 → 二者缺失，**全部归 0**
4. `extract_openai_responses_usage` 永远不会被走到（第 3 步已返回 `Some`）

结果：Responses 非流式请求的 `input/output/thinking/cache_read` **全部记 0**，
且不报错。修复方式是让词元解析**以请求级协议为准**（协议本就在管道里已确定）：
新增 `parse_usage_for_protocol(protocol, body)`，直接路由到对应协议解析器，
仅在对应解析器返回空时才回退全协议探测链（容忍上游返回与请求协议不符的形态）。

回归测试 `test_protocol_routing_responses_non_streaming_splits_reasoning`
同时断言「探测链归零」与「协议路由得到正确值」，防止再次退化。

### 若未来要由 SDK 接管转发

需要满足以下任一前提，否则不应替换：

- SDK 公开原始请求原语（raw body + 自定义 headers/URL），或
- 明确接受"类型化中转"的语义损失并承担 SDK 滞后风险，或
- 改为按协议使用各家官方 SDK 并接受多份依赖与各自的演进节奏。

## 5. 遗留与后续可选项

- **上游 SDK 版本滞后导致的 400**：若某上游 relay 自身 SDK 过旧，
  会对较新的 `role`（如 `developer`）报 `unknown variant`。
  这是**上游问题**——本代理已完成 `developer` role 的兼容验证：
  `async-openai 0.41.1` 可正常反序列化该 role。
  若确需对这类上游做兼容改写（`developer` → `system`），
  建议作为**可配置的协议兼容策略**加在领域层
  （按 Provider 或接入点开关），默认关闭、保持透明——需另行确认产品意图。
- **前端协议分发**：`api_protocol` 已随列表 / 详情 / 会话内容 / SSE 事件下发，
  前端可据此不再依赖 Chat→Responses 的试探式回退。
- **`api_protocol` 索引**：迁移已建 `idx_log_requests_api_protocol`，
  供后续按协议筛选日志使用；目前查询层尚未暴露该筛选参数。

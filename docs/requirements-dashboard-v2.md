# Dashboard 重写需求文档 v2

> 基于前两轮探索成果，整合第三轮用户修改意见（4 条核心修改）。

## 更新摘要

本版本对仪表盘需求进行了以下重大更新：

| #   | 修改点         | 变更类型     | 说明                                                             |
| --- | -------------- | ------------ | ---------------------------------------------------------------- |
| 1   | 统一统计周期   | 架构调整     | 全局 `periodDays` 控制所有图表                                   |
| 2   | 数字格式化修正 | 规范变更     | 使用四位分节的 `formatNumber`，禁用缩写                          |
| 3   | 饼图分类修正   | 数据定义变更 | 消除重复统计，输入/输出/缓存创建三分类                           |
| 4   | 新增服务商排名 | 功能新增     | 按 provider 维度的 Token 消耗排名，模型排名改为按 `model_mapped` |

## 需求概述

将当前 Dashboard（展示 4 个统计卡片 + 趋势柱状图 + Top-N 表格）重写为"数据概览"：使用 VChart 图表库，展示 4 个 Token
统计卡片、Token 趋势图、Token 分布饼图、Top-N 排名图，所有图表受全局统计周期控制。

### 业务目标

- 统一数据看板，让管理员一眼了解系统 Token 消耗全貌
- 提供多维度（时间、模型、服务商、接入点）的 Token 统计排名
- 准确反映 Token 消耗构成，消除重复统计
- 为中大规模部署提供性能参考

### 成功标准

- 4 个统计卡片展示指定周期内 Token 汇总及环比
- Token 趋势图展示每日 Token 消耗变化
- Token 分布饼图展示输入/输出/缓存创建三大构成
- 接入点、映射模型、服务商三个维度的 Top-N Token 排名
- 统一周期切换按钮影响所有图表
- 支持 API 降级（后端未就绪时显示 Mock 数据）

---

## 1. 数据设计方案

### 1.1 Token 分类语义（修正版）

所有 Token 分类和计算公式已通过代码验证：

- **总 Token** = 各字段之和：
  `total_tokens = input_tokens + output_tokens + cache_creation_input_tokens + cache_read_input_tokens + thinking_tokens`
- 验证位置：`src/infrastructure/parsers/parsed_token_usage.rs:118`

**饼图三分类（不重复统计）：**

| 饼图分类 | 计算方式                                 | 说明                               |
| -------- | ---------------------------------------- | ---------------------------------- |
| 输入     | `input_tokens + cache_read_input_tokens` | 所有进入模型的 Token（含缓存读取） |
| 输出     | `output_tokens + thinking_tokens`        | 所有模型产出的 Token（含思考过程） |
| 缓存创建 | `cache_creation_input_tokens`            | 新建缓存的 Token，独立写入成本     |

**验证**：三分类之和 = (input + cache_read) + (output + thinking) + cache_creation = input + output + cache_creation +
cache_read + thinking = 原 total_tokens。用户修改不存在统计偏差。

### 1.2 模型字段选择

本文档中所有"模型排名"使用 **`model_mapped`**（映射后模型名）而非 `model_original`（原始模型名）：

- `model_mapped` 始终有值：在 `ProxyLogData` DTO 中为 `String`（非 Option），来源为路由网格变换后的模型名
- `model_original` 存储的是用户请求的原始模型名，经过路由网格映射后可能被转换为其他模型
- `log_token_usage` 表已有 `(model_mapped, timestamp DESC)` 复合索引，查询性能有保障

---

## 2. API 设计方案

### 2.1 概述和新 API

#### `GET /api/stats/overview` — 扩展（新增 Token 统计字段）

**请求参数**（新增）：

| 参数   | 类型 | 必填 | 默认值 | 说明                     |
| ------ | ---- | ---- | ------ | ------------------------ |
| `days` | u64  | 否   | 30     | 统计天数（统一周期参数） |

**响应体**（新增 token\_\* 字段）：

```json
{
  "total_requests": 128456,
  "total_requests_change": 12.5,
  "active_access_points": 24,
  "active_access_points_change": 8.3,
  "total_tokens": 85000000,
  "total_tokens_change": 15.2,
  "input_tokens": 40000000,
  "input_tokens_change": 10.1,
  "output_tokens": 35000000,
  "output_tokens_change": 18.3,
  "cache_creation_tokens": 10000000,
  "cache_creation_tokens_change": -5.4,
  "active_users": 156,
  "active_users_change": -2.1,
  "error_rate": 2.3,
  "error_rate_change": -0.5
}
```

**实现位置**：

- 后端 DTO：`src/presentation/routes/stats/dto/overview_response.rs`
- 后端 SQL：`LogRepository::get_overview_stats()` 新增 JOIN `log_token_usage` 查询，使用 `SUM(ltu.total_tokens)`、
  `SUM(ltu.input_tokens + ltu.cache_read_input_tokens)` 等聚合
- 环比计算：查询上一周期数据做比较

#### `GET /api/stats/trends` — 扩展（新增 Token 趋势）

**请求参数**：

| 参数   | 类型 | 必填 | 默认值 | 说明     |
| ------ | ---- | ---- | ------ | -------- |
| `days` | u64  | 否   | 30     | 统计天数 |

**响应体**（新增 token 字段）：

```json
[
  {
    "date": "2026-05-20",
    "total_tokens": 2800000,
    "input_tokens": 1300000,
    "output_tokens": 1200000,
    "cache_creation_tokens": 300000
  }
]
```

**实现位置**：

- 后端 DTO：`src/presentation/routes/stats/dto/trend_item.rs`
- 后端 SQL：`LogRepository::get_token_trends(days)` 新方法，按天聚合 `log_token_usage`

#### `GET /api/stats/top-access-points` — 扩展（Token 维度）

**请求参数**：

| 参数    | 类型 | 必填 | 默认值 | 说明     |
| ------- | ---- | ---- | ------ | -------- |
| `limit` | u64  | 否   | 10     | 返回条数 |
| `days`  | u64  | 否   | 30     | 统计天数 |

**响应体**（新增 `token_total` 字段）：

```json
[
  {
    "access_point_id": "uuid",
    "short_code": "gp4",
    "name": "GPT-4 接入点",
    "request_count": 45200,
    "token_total": 28000000
  }
]
```

**实现位置**：

- 需要 JOIN `access_points` 表获取 `short_code` 和 `name`
- GROUP BY `access_point_id`，SUM token 用量

#### `GET /api/stats/top-models` — 改为按 `model_mapped`

**请求参数**：

| 参数    | 类型 | 必填 | 默认值 | 说明     |
| ------- | ---- | ---- | ------ | -------- |
| `limit` | u64  | 否   | 10     | 返回条数 |
| `days`  | u64  | 否   | 30     | 统计天数 |

**响应体**（改为 token 维度）：

```json
[
  {
    "model": "claude-sonnet-4-20250514",
    "request_count": 28500,
    "token_total": 15000000
  }
]
```

**实现位置**：

- 将 SQL GROUP BY `model_original` 改为 GROUP BY `model_mapped`
- 数据来源从 `log_metadata` 迁移到 `log_token_usage`（支持 token 聚合）
- 使用 `(model_mapped, timestamp DESC)` 索引

#### `GET /api/stats/top-providers` — 新增

**请求参数**：

| 参数    | 类型 | 必填 | 默认值 | 说明     |
| ------- | ---- | ---- | ------ | -------- |
| `limit` | u64  | 否   | 10     | 返回条数 |
| `days`  | u64  | 否   | 30     | 统计天数 |

**响应体**：

```json
[
  {
    "provider_id": "uuid",
    "provider_name": "Anthropic",
    "request_count": 42000,
    "token_total": 35000000
  }
]
```

**实现位置**：

- 后端 DTO：新建 `src/presentation/routes/stats/dto/top_provider_item.rs`
- 后端 SQL：`LogRepository::top_providers(limit, days)` 新方法
- JOIN `providers` 表获取 `name` 字段
- 数据来源：`log_token_usage` 的 `provider_id`
- 注册路由：`GET /api/stats/top-providers`

#### `GET /api/stats/token-distribution` — 新增（饼图）

**请求参数**：

| 参数   | 类型 | 必填 | 默认值 | 说明     |
| ------ | ---- | ---- | ------ | -------- |
| `days` | u64  | 否   | 30     | 统计天数 |

**响应体**：

```json
{
  "input_total": 40000000,
  "output_total": 35000000,
  "cache_creation_total": 10000000
}
```

**计算方式**（三分类不重复）：

- `input_total = SUM(input_tokens) + SUM(cache_read_input_tokens)`
- `output_total = SUM(output_tokens) + SUM(thinking_tokens)`
- `cache_creation_total = SUM(cache_creation_input_tokens)`
- `total = input_total + output_total + cache_creation_total`（与原 total_tokens 一致）

**实现位置**：

- 后端 DTO：新建 `src/presentation/routes/stats/dto/token_distribution_response.rs`
- 后端 SQL：`LogRepository::get_token_distribution(days)` 新方法

### 2.2 API 路由汇总

| 方法 | 路径                            | 说明                                     | 状态 |
| ---- | ------------------------------- | ---------------------------------------- | ---- |
| GET  | `/api/stats/overview`           | 概览（请求量 + Token 统计 + 环比）       | 改造 |
| GET  | `/api/stats/trends`             | 每日 Token 趋势                          | 改造 |
| GET  | `/api/stats/top-access-points`  | 接入点 Token 排名                        | 改造 |
| GET  | `/api/stats/top-models`         | 映射模型 Token 排名（改为 model_mapped） | 改造 |
| GET  | `/api/stats/top-providers`      | 服务商 Token 排名                        | 新增 |
| GET  | `/api/stats/token-distribution` | Token 分布（饼图三分类）                 | 新增 |

### 2.3 统一 `days` 参数

- 所有 API 接受可选的 `days` 查询参数
- 默认值统一为 30
- 前端所有 API 调用使用相同的全局 `periodDays` 状态值

---

## 3. 前端设计方案

### 3.1 安装依赖

```bash
cd /home/viktor/dev/projects/github/token-proxy/token-proxy
pnpm add @visactor/react-vchart @visactor/vchart
```

### 3.2 全局统计周期

- GettingStartedPage 维护一个 `periodDays` 状态（类型 `number`，默认 30）
- 页面顶部放置三个按钮：7 天 / 30 天 / 90 天，高亮当前选中值
- 切换时设置 `periodDays`，触发 `useEffect` 全部 API 重新请求
- 所有 API 调用使用相同的 `days=${periodDays}` 参数

### 3.3 布局结构

```
┌──────────────────────────────────────────────────────┐
│  [7天] [30天] [90天]           ← 全局周期切换按钮   │
├──────────────────────────────────────────────────────┤
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐               │
│  │总Token│ │ 输入 │ │ 输出 │ │缓存创建│              │
│  │ 值+环 │ │ 环比 │ │ 环比 │ │ 环比 │               │
│  └──────┘ └──────┘ └──────┘ └──────┘               │
├──────────────────────────────────────────────────────┤
│  ┌─────────────── Token 趋势折线图 ──────────────┐  │
│  │  多系列：总 Token、输入、输出                 │  │
│  └──────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────┤
│  ┌───────┐ ┌───────────┐ ┌──────────────┐          │
│  │ Token │ │ Top 接入点│ │ Top 服务商   │          │
│  │ 饼图  │ │ Token排名 │ │ Token排名    │          │
│  │       │ │ 柱状图    │ │ 柱状图       │          │
│  └───────┘ └───────────┘ └──────────────┘          │
├──────────────────────────────────────────────────────┤
│  ┌─────────────── Top 映射模型 Token 排名 ─────┐    │
│  │ 排名 | 映射模型 | 请求次数 | Token 消耗     │    │
│  └──────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

### 3.4 数据类型定义

```typescript
// src-getting-started/types/getting-started.ts

/** Dashboard 概览统计数据 */
export interface OverviewData {
  total_requests: number;
  total_requests_change?: number;
  active_access_points: number;
  active_access_points_change?: number;
  /** Token 相关统计 */
  total_tokens: number;
  total_tokens_change?: number;
  input_tokens: number;
  input_tokens_change?: number;
  output_tokens: number;
  output_tokens_change?: number;
  cache_creation_tokens: number;
  cache_creation_tokens_change?: number;
  active_users?: number;
  active_users_change?: number;
  error_rate?: number;
  error_rate_change?: number;
}

/** 趋势数据点（包含 Token 维度） */
export interface TrendItem {
  date: string;
  total_tokens: number;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
}

/** Top-N 接入点（Token 维度） */
export interface TopAccessPoint {
  access_point_id: string;
  short_code: string;
  name: string;
  request_count: number;
  token_total: number;
}

/** Top-N 映射模型（Token 维度） */
export interface TopModel {
  model: string;
  request_count: number;
  token_total: number;
}

/** Top-N 服务商 */
export interface TopProvider {
  provider_id: string;
  provider_name: string;
  request_count: number;
  token_total: number;
}

/** Token 分布（饼图） */
export interface TokenDistribution {
  input_total: number;
  output_total: number;
  cache_creation_total: number;
}
```

### 3.5 数字格式化规范

- 所有 Token 数字展示使用 `formatNumber(value)`（默认 `useChineseStyle = true`）
- 示例：`128000000` 显示为 `1,2800,0000`（中文四位分节）
- 禁用缩写显示（如 `12.8M`、`1.2B`）
- 工具函数已存在于 `src-dashboard/utils/format.ts`，无需新增

### 3.6 前端类型与后端对齐

业务层（dashboard）不需要对 `top_provider_item` 中的 `provider_id` 做 UUID 格式化——使用 Semi `Code` 组件展示，等宽字体已通过
CSS 实现，不需要别的方式。

---

## 4. 后端变更清单

### 4.1 新增文件

| 文件路径                                                           | 说明                 |
| ------------------------------------------------------------------ | -------------------- |
| `src/presentation/routes/stats/dto/top_provider_item.rs`           | Top 服务商排名响应体 |
| `src/presentation/routes/stats/dto/token_distribution_response.rs` | Token 分布响应体     |

### 4.2 修改文件

| 文件路径                                                        | 变更                             |
| --------------------------------------------------------------- | -------------------------------- |
| `src/presentation/routes/stats/dto/overview_response.rs`        | 新增 token 字段                  |
| `src/presentation/routes/stats/dto/trend_item.rs`               | 新增 token 字段                  |
| `src/presentation/routes/stats/dto/top_access_point_item.rs`    | 新增 token_total 字段            |
| `src/presentation/routes/stats/dto/top_model_item.rs`           | 新增 request_count + token_total |
| `src/presentation/routes/stats/dto/top_query.rs`                | 新增 days 字段                   |
| `src/presentation/routes/stats_routes.rs`                       | 注册新路由，修改现有路由参数     |
| `src/domain/log/repository_log.rs`                              | 新增统计方法 trait 签名          |
| `src/infrastructure/persistence/repositories/log_repository.rs` | 实现新统计 SQL 查询              |
| `src/application/log/log_service.rs`                            | 新增统计方法实现                 |

### 4.3 Token 用量解析需关注的变更

饼图数据直接来自 SQL 聚合，无需修改 Token 用量解析逻辑。`total_tokens` 的原始计算方式保持不变（各字段之和），饼图三分类在前端展示层组合。

---

## 5. 前端变更清单

### 5.1 新增文件

| 文件路径                                       | 说明 |
| ---------------------------------------------- | ---- |
| 无（图表组件直接在 GettingStartedPage 内实现） |      |

### 5.2 修改文件

| 文件路径                                            | 变更                                                                       |
| --------------------------------------------------- | -------------------------------------------------------------------------- |
| `src-dashboard/package.json`                        | 新增 `@visactor/react-vchart` 和 `@visactor/vchart` 依赖                   |
| `src-dashboard/types/dashboard.ts`                  | 新增 TopProvider / TokenDistribution 等类型                                |
| `src-dashboard/pages/GettingStartedPage.tsx`        | 全面重写：全局周期控制 + 4 卡片 + VChart 趋势/饼图 + 3 排名柱状图 + 排名表 |
| `src-dashboard/components/dashboard/StatCard.tsx`   | 无需修改（兼容 token 类型）                                                |
| `src-dashboard/components/dashboard/TrendChart.tsx` | 降级/废弃（主 Dashboard 不再使用）                                         |

### 5.3 GettingStartedPage 核心逻辑

```typescript
// 核心状态
const [periodDays, setPeriodDays] = useState(30);
const [loading, setLoading] = useState(true);

// 统一周期切换
const periods = [7, 30, 90];

// 所有 API 调用使用 periodDays
useEffect(() => {
  Promise.all([
    api.get(`/api/stats/overview?days=${periodDays}`),
    api.get(`/api/stats/trends?days=${periodDays}`),
    api.get(`/api/stats/token-distribution?days=${periodDays}`),
    api.get(`/api/stats/top-access-points?limit=5&days=${periodDays}`),
    api.get(`/api/stats/top-models?limit=5&days=${periodDays}`),
    api.get(`/api/stats/top-providers?limit=5&days=${periodDays}`),
  ]);
}, [periodDays]);
```

---

## 6. 相关文件

### 6.1 后端核心文件

| 文件                                                            | 本需求中角色                                            |
| --------------------------------------------------------------- | ------------------------------------------------------- |
| `src/domain/log/token_usage.rs`                                 | `log_token_usage` 实体定义，所有 Token 统计的数据库来源 |
| `src/domain/log/repository_log.rs`                              | Repository trait，新增统计方法签名                      |
| `src/infrastructure/parsers/parsed_token_usage.rs`              | Token 用量解析，`total_tokens` 计算公式所在文件         |
| `src/infrastructure/persistence/repositories/log_repository.rs` | 具体 SQL 实现                                           |
| `src/application/log/log_service.rs`                            | 应用层编排，已有 `get_overview_stats` 等方法            |
| `src/presentation/routes/stats_routes.rs`                       | 统计路由注册                                            |
| `src/presentation/routes/stats/dto/`                            | 已有 6 个 DTO 文件，需扩展                              |

### 6.2 前端核心文件

| 文件                                                | 本需求中角色              |
| --------------------------------------------------- | ------------------------- |
| `src-dashboard/utils/format.ts`                     | `formatNumber` 函数已存在 |
| `src-dashboard/types/dashboard.ts`                  | 类型定义                  |
| `src-dashboard/pages/GettingStartedPage.tsx`        | 主页面                    |
| `src-dashboard/components/dashboard/StatCard.tsx`   | 统计卡片组件              |
| `src-dashboard/components/dashboard/TrendChart.tsx` | 将降级/废弃               |

---

## 7. 无更多问题需要澄清

经过三轮需求探索（初始探索 → 核心方向批准 → 4 条修改意见整合），以及以下代码验证工作：

1. **Token 字段语义验证**：确认 `total_tokens = input + output + cache_creation + cache_read + thinking`，用户提议的三分类之和等于原
   total_tokens
2. **`formatNumber` 已存在**：`src-dashboard/utils/format.ts` 中已有实现，四位分节模式
3. **`model_mapped` 始终有值**：`ProxyLogData` 中为 `String` 非 Option，`(model_mapped, timestamp DESC)` 索引已存在
4. **`providers` 表结构确认**：`id: Uuid` + `name: String`，可通过 `provider_id` 关联
5. **现有统计路由确认**：4 个端点均需扩展，2 个新增
6. **前端依赖确认**：VChart 需要 `@visactor/react-vchart` 和 `@visactor/vchart` 新安装

**无更多问题需要澄清**。以上需求文档可直接作为后续设计和开发阶段的技术规范。

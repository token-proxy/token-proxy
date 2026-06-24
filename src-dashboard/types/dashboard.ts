/**
 * Dashboard 相关 TypeScript 类型定义。
 *
 * 与后端 `src/application/dashboard/dto/` 的 Rust DTO 一一对应。
 * 字段命名遵循后端 serde 序列化结果（snake_case 字段名 + 枚举 lowercase）。
 */

// --- 时间范围 ---

/**
 * 时间范围预设。
 *
 * - `today` — 今日（小时桶 + 对比昨日）
 * - `last7` — 近 7 天（日桶 + 对比上 7 天）
 * - `last30` — 近 30 天（日桶 + 对比上 30 天）
 * - `custom` — 自定义起止时间（必须提供 start / end）
 */
export type TimeRangePreset = 'today' | 'last7' | 'last30' | 'custom';

/**
 * Dashboard 时间范围查询参数。
 *
 * 作为所有 Dashboard 数据源（KPI / Top Users / Top Accounts）的统一过滤器。
 */
export interface TimeRangeQuery {
  /** 时间范围预设 */
  range: TimeRangePreset;
  /** 自定义起始时间（ISO 8601），仅 custom 模式必填 */
  start?: string;
  /** 自定义结束时间（ISO 8601），仅 custom 模式必填 */
  end?: string;
}

// --- KPI 与趋势 ---

/**
 * 趋势徽章。
 *
 * - `up` / `down` / `flat` — 双窗皆有数据时的常规趋势
 * - `new` — 当前 > 0 且上一窗 = 0（无百分比）
 * - `empty` — 双窗皆为 0（无趋势可言）
 */
export type TrendBadge = 'up' | 'down' | 'flat' | 'new' | 'empty';

/**
 * KPI 趋势项（适用于请求数 / 词元量 / 活跃成员数）。
 */
export interface KpiTrendItem {
  /** 当前窗口值 */
  current: number;
  /** 上一等长窗口值 */
  previous: number;
  /** 趋势徽章 */
  trend: TrendBadge;
  /** 百分比变化（如 +12.3 表示上升 12.3%）；null 表示无法计算（empty 或 new） */
  change_pct: number | null;
}

/**
 * 缓存命中率（独立类型，因为是比率而非计数）。
 *
 * 分母为 0 时 `rate = null`，前端应显示 `—` 占位。
 */
export interface CacheHitRate {
  /** 当前命中率（0.0 - 1.0）；null = 无可命中数据 */
  rate: number | null;
  /** 上一窗命中率；null = 无可命中数据 */
  previous_rate: number | null;
  /** 命中率百分比变化；null = 无法计算 */
  change_pct: number | null;
  /** 趋势徽章 */
  trend: TrendBadge;
}

/**
 * Sparkline 时间序列桶。
 *
 * 三条序列（请求 / 词元 / 成员数）共享同一 bucket_start。
 */
export interface SparklineBucket {
  /** 桶起始时间（ISO 8601） */
  bucket_start: string;
  /** 该桶请求数 */
  request_count: number;
  /** 该桶词元总量 */
  total_tokens: number;
  /** 该桶活跃成员数（去重） */
  active_user_count: number;
}

/**
 * Dashboard KPI 完整响应（4 张卡 + sparkline 序列）。
 *
 * 后端单次查询返回，前端解构后分发到 KpiCard / CacheHitCard / Sparkline。
 */
export interface KpiResponse {
  /** 请求数 KPI */
  request_count: KpiTrendItem;
  /** 词元总量 KPI */
  total_tokens: KpiTrendItem;
  /** 活跃成员数 KPI */
  active_user_count: KpiTrendItem;
  /** 缓存命中率 KPI */
  cache_hit_rate: CacheHitRate;
  /** 内嵌时间序列（供前 3 张 KPI 的 sparkline 使用） */
  sparkline: { buckets: SparklineBucket[] };
}

// --- 排行榜 ---

/**
 * 单个成员排行项。
 *
 * `username` 和 `display_name` 均为 null 时表示该用户已被删除，
 * 前端应降级显示 `已删除成员 · <uuid 前 8 位>`。
 */
export interface TopUserItem {
  /** 成员 UUID（即使删除也保留） */
  user_id: string;
  /** 用户名；null = 已删除 */
  username: string | null;
  /** 显示名；null = 已删除或未设置 */
  display_name: string | null;
  /** 窗口内请求数 */
  request_count: number;
  /** 窗口内词元总消耗 */
  total_tokens: number;
}

/**
 * 单个账号排行项。
 *
 * `account_name` 和 `provider_name` 均为 null 时表示该账号已被删除，
 * 前端应降级显示 `已删除账号 · <uuid 前 8 位>`。
 */
export interface TopAccountItem {
  /** 账号 UUID */
  account_id: string;
  /** 账号名；null = 已删除 */
  account_name: string | null;
  /** 所属服务商 UUID；null = 账号已删除 */
  provider_id: string | null;
  /** 服务商名；null = 已删除或账号已删除 */
  provider_name: string | null;
  /** 当前禁用原因（字符串化的 DisabledReason）；null = 正常可用 */
  disabled_reason: string | null;
  /** 输入词元数 */
  input_tokens: number;
  /** 输出词元数 */
  output_tokens: number;
  /** 缓存读取词元数 */
  cache_read_tokens: number;
  /** 缓存写入词元数 */
  cache_creation_tokens: number;
  /** 总词元数（用于排序） */
  total_tokens: number;
}

/**
 * 成员排行响应（按 request_count 降序）。
 */
export interface TopUsersResponse {
  /** 排行项数组 */
  items: TopUserItem[];
}

/**
 * 账号排行响应（按 total_tokens 降序）。
 */
export interface TopAccountsResponse {
  /** 排行项数组 */
  items: TopAccountItem[];
}

// --- Top Clients 排行 ---

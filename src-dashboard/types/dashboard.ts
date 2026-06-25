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
 * 作为所有 Dashboard 数据源的统一过滤器。
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
 * KPI 趋势项（适用于请求数 / 词元量等可累计指标）。
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
 * 比率趋势项（适用于成功率 / 缓存命中率等 0.0 - 1.0 指标）。
 */
export interface RateTrendItem {
  /** 当前窗口比率（0.0 - 1.0）；null = 无样本 */
  rate: number | null;
  /** 上一等长窗口比率；null = 无样本 */
  previous_rate: number | null;
  /** 百分比变化（如 +12.3 表示上升 12.3%）；null 表示无法计算 */
  change_pct: number | null;
  /** 趋势徽章 */
  trend: TrendBadge;
}

/**
 * 缓存命中率（独立类型，因为是比率而非计数）。
 *
 * 分母为 0 时 `rate = null`，前端应显示 `—` 占位。
 */
/** 缓存命中率（语义别名，字段同 RateTrendItem） */
export type CacheHitRate = RateTrendItem;

/**
 * Sparkline 时间序列桶。
 *
 * 请求数与词元量两条序列共享同一 bucket_start。
 */
export interface SparklineBucket {
  /** 桶起始时间（ISO 8601） */
  bucket_start: string;
  /** 该桶请求数 */
  request_count: number;
  /** 该桶词元总量 */
  total_tokens: number;
}

/**
 * 词元构成 5 维度绝对值。
 *
 * 与后端 `log_token_usage` 表的 5 个词元列对齐，反映当前窗口内的真实消耗结构，
 * 用于堆叠条 / 环形图等可视化组件。
 */
export interface TokenComposition {
  /** 未命中缓存输入词元数 */
  input_tokens: number;
  /** 输出词元数 */
  output_tokens: number;
  /** 缓存创建输入词元数 */
  cache_creation_tokens: number;
  /** 缓存命中输入词元数 */
  cache_read_tokens: number;
  /** 思考词元数 */
  thinking_tokens: number;
}

/**
 * Dashboard KPI 完整响应。
 *
 * 后端单次查询返回，前端解构后分发到对应的可视化组件。
 * `cache_hit_rate` 与 `composition` 同源，避免二次查询。
 */
export interface KpiResponse {
  /** 会话数 KPI（不重复 session_id 计数） */
  session_count: KpiTrendItem;
  /** 请求数 KPI */
  request_count: KpiTrendItem;
  /** 词元总量 KPI */
  total_tokens: KpiTrendItem;
  /** 输入词元 KPI（不含缓存） */
  input_tokens: KpiTrendItem;
  /** 输出词元 KPI */
  output_tokens: KpiTrendItem;
  /** 词元构成 5 维度 */
  composition: TokenComposition;
  /** 缓存命中率（与构成同源，避免二次查询） */
  cache_hit_rate: CacheHitRate;
  /** 内嵌时间序列（供 KPI 的 sparkline 使用） */
  sparkline: { buckets: SparklineBucket[] };
}

/**
 * 用量趋势时间序列桶。
 *
 * 每个桶包含请求数、会话数和 5 类词元用量，供趋势图一次性绘制面积图与堆叠柱状图。
 */
export interface UsageTrendBucket {
  /** 桶起始时间（ISO 8601） */
  bucket_start: string;
  /** 该桶请求数 */
  request_count: number;
  /** 该桶不重复会话数 */
  session_count: number;
  /** 该桶词元总量 */
  total_tokens: number;
  /** 未命中缓存输入词元数 */
  input_tokens: number;
  /** 输出词元数 */
  output_tokens: number;
  /** 缓存创建输入词元数 */
  cache_creation_tokens: number;
  /** 缓存命中输入词元数 */
  cache_read_tokens: number;
  /** 思考词元数 */
  thinking_tokens: number;
}

/**
 * 用量趋势响应。
 *
 * 后端已按所选时间窗口补齐空桶，前端可直接按顺序渲染。
 */
export interface UsageTrendsResponse {
  /** 趋势桶数组 */
  buckets: UsageTrendBucket[];
}

// --- 活跃度热力图 ---

/**
 * 热力图单元格（按日聚合）。
 *
 * 每个单元格代表一天的活跃强度，用于 GitHub 贡献图风格的日历热力图。
 */
export interface HeatmapCell {
  /** ISO 日期字符串，格式 `YYYY-MM-DD` */
  day: string;
  /** 该日词元总量 */
  total_tokens: number;
  /** 该日请求数 */
  request_count: number;
}

/**
 * 活跃度热力图响应。
 *
 * 单元格按日期升序排列，缺失日期视为零活跃（前端自行补全占位）。
 */
export interface HeatmapResponse {
  /** 热力图单元格数组 */
  cells: HeatmapCell[];
}

// --- 模型排行 ---

/**
 * 单个模型排行项。
 */
export interface TopModelItem {
  /** 模型名（原始字符串，未做归一化） */
  model: string;
  /** 窗口内请求数 */
  request_count: number;
  /** 窗口内词元总量 */
  total_tokens: number;
}

/**
 * 模型排行响应（按 total_tokens 降序）。
 */
export interface TopModelsResponse {
  /** 排行项数组 */
  items: TopModelItem[];
}

// --- 接入点排行 ---

/**
 * 单个接入点排行项。
 *
 * `name` 和 `short_code` 均为 null 时表示该接入点已被删除，
 * 前端应降级显示 `已删除接入点 · <uuid 前 8 位>`。
 */
export interface TopAccessPointItem {
  /** 接入点 UUID（即使删除也保留） */
  access_point_id: string;
  /** 接入点名；null = 已删除 */
  name: string | null;
  /** 短码；null = 已删除 */
  short_code: string | null;
  /** 窗口内请求数 */
  request_count: number;
  /** 窗口内词元总量 */
  total_tokens: number;
}

/**
 * 接入点排行响应（按 total_tokens 降序）。
 */
export interface TopAccessPointsResponse {
  /** 排行项数组 */
  items: TopAccessPointItem[];
}

// --- 服务质量 ---

/**
 * 服务质量指标响应。
 *
 * 各 rate 字段在样本数为 0 时返回 null，前端应显示 `—` 占位；
 * 延迟字段在无成功请求时为 null。
 */
export interface QualityResponse {
  /** 窗口内总请求数（含全部状态分类） */
  total_count: number;
  /** 成功率趋势（2xx 占比，0.0 - 1.0） */
  success_rate: RateTrendItem;
  /** 客户端错误率（4xx 占比，0.0 - 1.0）；null = 无样本 */
  client_error_rate: number | null;
  /** 服务端错误率（5xx 占比，0.0 - 1.0）；null = 无样本 */
  server_error_rate: number | null;
  /** 中断率（SSE 流式提前断开占比，0.0 - 1.0）；null = 无样本 */
  interrupted_rate: number | null;
  /** 平均耗时（毫秒）；null = 无成功样本 */
  avg_duration_ms: number | null;
  /** P95 耗时（毫秒）；null = 无成功样本 */
  p95_duration_ms: number | null;
}

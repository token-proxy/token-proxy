/** Dashboard 概览统计数据 */
export interface OverviewData {
  /** 近 30 天总请求量 */
  total_requests: number;
  total_requests_change?: number;
  active_access_points: number;
  active_access_points_change?: number;
  active_users?: number;
  active_users_change?: number;
  /** 错误率百分比 */
  error_rate?: number;
  error_rate_change?: number;
}

/** 趋势数据点 */
export interface TrendItem {
  date: string;
  count: number;
}

/** Top-N 接入点 */
export interface TopAccessPoint {
  short_code: string;
  name: string;
  count: number;
}

/** Top-N 模型 */
export interface TopModel {
  model: string;
  count: number;
}

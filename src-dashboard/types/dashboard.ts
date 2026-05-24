export interface OverviewData {
  total_requests: number;
  total_requests_change?: number;
  active_access_points: number;
  active_access_points_change?: number;
  active_users?: number;
  active_users_change?: number;
  error_rate?: number;
  error_rate_change?: number;
}

export interface TrendItem {
  date: string;
  count: number;
}

export interface TopAccessPoint {
  short_code: string;
  name: string;
  count: number;
}

export interface TopModel {
  model: string;
  count: number;
}

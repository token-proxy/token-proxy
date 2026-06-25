/** 未匹配模型的代用标识，后端路由网格以此行作为兜底规则 */
export const UNMATCHED_MODEL = '__unmatched__';

/**
 * 模型映射编辑器中的单条映射记录
 *
 * 表示一条源模型到目标模型的转换规则。匹配类型由编辑器根据源模型值自动推导，
 * 不在映射记录中存储。
 */
export interface ModelMapping {
  /** 源模型名称或模型族前缀 */
  source_model: string;
  /** 目标模型名称 */
  target_model: string;
}

/** 账号池中的单条账号条目 */
export interface AccountEntry {
  account_id: string;
  /** 所属服务商 ID */
  provider_id?: string;
  /** 权重路由模式下使用的权重值 */
  weight?: number;
  /** 优先级路由模式下使用的优先级序号 */
  priority?: number;
  /** 账号状态: enabled | disabled */
  status?: string;
}

/** 模型路由网格的一行：源模型到各服务商目标模型的映射 */
export interface ModelRoutingRow {
  /** 原始模型名或模型族前缀（如 claude-sonnet-） */
  source_model: string;
  /** 按 provider_id 映射到目标模型名 */
  targets: Record<string, string | null>;
}

/** 模型路由网格：二维表结构（source_model x provider_id） */
export interface ModelRoutingGrid {
  provider_ids: string[];
  rows: ModelRoutingRow[];
}

/** 接入点完整信息，由后端返回 */
export interface AccessPoint {
  id: string;
  name: string;
  short_code: string;
  /** API 类型，如 anthropic */
  api_type: string;
  accounts: AccountEntry[];
  /** 路由策略: weighted | priority */
  routing_strategy: string;
  model_routing_grid: ModelRoutingGrid;
  access_url: string;
  /** 接入点状态: enabled | disabled */
  status: string;
  created_at: string;
  updated_at: string;
}

/** 接入点创建/编辑表单数据 */
export interface AccessPointFormData {
  name: string;
  short_code: string;
  api_type: string;
  accounts: AccountEntry[];
  routing_strategy: string;
  model_routing_grid: ModelRoutingGrid;
}

/** 服务商故障配置的 JSON 表示形式，与 Rust `FaultConfig` 的 serde 序列化格式对应 */
export interface FaultConfigJson {
  status_codes: string[];
  type: string;
  delay?: { value: number; unit: string };
  config?: {
    source: string;
    source_path: string;
    regex_pattern: string;
    kind: { type: string; unit?: string };
    on_extract_failed?: {
      type: string;
      delay?: { value: number; unit: string };
    };
  };
}

/** 服务商选项，用于接入点编辑时选择服务商 */
export interface ProviderOption {
  id: string;
  name: string;
  models?: string[];
  account_count?: number;
  available_account_count?: number;
  rate_limit_config?: FaultConfigJson;
  balance_exhausted_config?: FaultConfigJson;
}

/** 账号选项，用于接入点编辑时选择账号 */
export interface AccountOption {
  id: string;
  provider_id: string;
  name: string;
  /** API Key 末尾几位，用于区分不同账号 */
  api_key_suffix: string;
  /** 账号状态: enabled | disabled */
  status: string;
  disabled_reason?: string;
  available_at?: string;
}

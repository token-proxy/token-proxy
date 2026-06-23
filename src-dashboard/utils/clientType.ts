/**
 * 客户端类型中文名称映射。
 *
 * 将后端返回的 `client_type` 枚举值转换为中文展示名。
 * 未匹配时返回原值作为兜底。
 */
export const CLIENT_TYPE_LABELS: Record<string, string> = {
  claude_code: 'Claude Code',
  codex: 'Codex',
  other: '其他客户端',
  unknown: '未知客户端',
};

/**
 * 客户端类型对应的 Tag 颜色。
 *
 * - Claude Code → 蓝色（primary）
 * - Codex → 绿色（success）
 * - other / unknown / 兜底 → 灰色
 */
export const CLIENT_TYPE_COLORS: Record<string, 'blue' | 'green' | 'grey'> = {
  claude_code: 'blue',
  codex: 'green',
  other: 'grey',
  unknown: 'grey',
};

/** 将 client_type 转换为 Tag 颜色，未匹配时回退灰色 */
export function tagColorFor(type: string): 'blue' | 'green' | 'grey' {
  return CLIENT_TYPE_COLORS[type] ?? 'grey';
}

/** 将 client_type 转换为中文展示名，未匹配时返回原值 */
export function clientTypeLabel(type: string): string {
  return CLIENT_TYPE_LABELS[type] ?? type;
}

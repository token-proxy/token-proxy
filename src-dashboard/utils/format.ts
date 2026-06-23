/** 格式化为中文日期时间字符串，非法输入返回 '-' */
export function formatDateTime(ts: string | null | undefined): string {
  if (!ts) return '-';
  try {
    return new Date(ts).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  } catch {
    return ts;
  }
}

/** 格式化耗时（毫秒），非法输入返回 '-' */
export function formatDuration(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return '-';
  return `${ms} ms`;
}

/** 截断字符串，超长部分替换为 '...' */
export function truncate(str: string | null | undefined, maxLen: number): string {
  if (!str) return '-';
  return str.length > maxLen ? str.slice(0, maxLen) + '...' : str;
}

/** 从中间截断字符串，保留首尾 */
export function truncateMiddle(str: string | null | undefined, maxLen = 16): string {
  if (!str) return '-';
  if (str.length <= maxLen) return str;
  const half = Math.floor((maxLen - 3) / 2);
  return str.slice(0, half) + '...' + str.slice(-half);
}

/** 格式化为中文短日期时间 */
export function formatDate(dateStr: string): string {
  const date = new Date(dateStr);
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/**
 * 紧凑 token 数字格式化（用于 Dashboard 大数字展示）。
 *
 * - 值 >= 1_000_000 → "X.YM"
 * - 值 >= 1_000 → "X.YK"
 * - 否则保留原数字（中文千分位）
 *
 * 非有限数返回占位符 '—'。
 *
 * @example
 * formatTokenCompact(12_500_000) // "12.5M"
 * formatTokenCompact(8_429) // "8.4K"
 * formatTokenCompact(342) // "342"
 */
export function formatTokenCompact(n: number): string {
  if (!Number.isFinite(n)) return '—';
  if (Math.abs(n) >= 1_000_000) {
    return `${(n / 1_000_000).toFixed(1)}M`;
  }
  if (Math.abs(n) >= 1_000) {
    return `${(n / 1_000).toFixed(1)}K`;
  }
  return n.toLocaleString('zh-CN');
}

/**
 * 格式化数字为带分隔符的字符串
 *
 * Chinese style (useChineseStyle=true) 时每四位加逗号，符合中文数字习惯；
 * useChineseStyle=false 时使用 en-US 千位分隔。
 */
export function formatNumber(num: number, useChineseStyle = true): string {
  if (useChineseStyle) {
    // 先转为字符串，处理可能传入的 number
    const str = String(num);
    // 使用正则分离：符号、整数部分、小数部分
    const match = str.match(/^([+-]?)(\d*)\.?(\d*)$/);
    if (!match) return str; // 理论上不会发生
    const [, sign, intPart, decPart] = match;
    // 如果整数部分为空（例如 .5），补为 "0"
    const int = intPart || '0';
    // 在整数部分从右往左每四位插入逗号
    const formattedInt = int.replace(/\B(?=(\d{4})+(?!\d))/g, ',');
    // 拼接符号、整数、小数（如果有）
    return sign + formattedInt + (decPart ? '.' + decPart : '');
  } else {
    return num.toLocaleString('en-US');
  }
}

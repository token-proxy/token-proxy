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

export function formatDuration(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return '-';
  return `${ms} ms`;
}

export function truncate(str: string | null | undefined, maxLen: number): string {
  if (!str) return '-';
  return str.length > maxLen ? str.slice(0, maxLen) + '...' : str;
}

export function truncateMiddle(str: string | null | undefined, maxLen = 16): string {
  if (!str) return '-';
  if (str.length <= maxLen) return str;
  const half = Math.floor((maxLen - 3) / 2);
  return str.slice(0, half) + '...' + str.slice(-half);
}

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

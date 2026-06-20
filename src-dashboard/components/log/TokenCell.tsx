import { type ReactNode } from 'react';
import { Tooltip } from '@douyinfe/semi-ui';
import { formatNumber } from '../../utils/format.ts';

/** TokenCell 组件 Props */
interface TokenCellProps {
  input_tokens?: number | null;
  output_tokens?: number | null;
  cache_creation_input_tokens?: number | null;
  cache_read_input_tokens?: number | null;
  thinking_tokens?: number | null;
  total_tokens?: number | null;
}

/** Token 分类显示配置，按 label 排序展示在 Tooltip 中 */
const CATEGORIES = [
  { label: '总计',        value: (_v: Values) => _v.total },
  { label: '新输入',      value: (_v: Values) => _v.input },
  { label: '缓存创建',    value: (_v: Values) => _v.cacheCreate },
  { label: '缓存读取',    value: (_v: Values) => _v.cacheRead },
  { label: '输出',        value: (_v: Values) => _v.output },
  { label: '思考',        value: (_v: Values) => _v.thinking },
];

interface Values {
  input: number;
  output: number;
  cacheCreate: number;
  cacheRead: number;
  thinking: number;
  total: number;
}

/**
 * TokenCell - Token 用量表格单元格组件
 *
 * 精简显示 ↑输入 / ↓输出，Tooltip 悬浮展示各分类详细数量。
 */
export default function TokenCell(props: TokenCellProps): ReactNode {
  const {
    input_tokens,
    output_tokens,
    cache_creation_input_tokens,
    cache_read_input_tokens,
    thinking_tokens,
    total_tokens,
  } = props;

  const v: Values = {
    input: input_tokens ?? 0,
    output: output_tokens ?? 0,
    cacheCreate: cache_creation_input_tokens ?? 0,
    cacheRead: cache_read_input_tokens ?? 0,
    thinking: thinking_tokens ?? 0,
    total: total_tokens ?? 0,
  };

  // 全部为空或全部为 0 则不展示
  if (v.total === 0 && v.input === 0 && v.output === 0) {
    return <span style={{ color: 'var(--semi-color-text-2)' }}>-</span>;
  }

  const inputSum = v.input + v.cacheCreate + v.cacheRead;
  const outputSum = v.output + v.thinking;

  const tooltipContent = (
    <div style={{ fontSize: 12, lineHeight: 1.8, whiteSpace: 'nowrap' }}>
      {CATEGORIES.map(c => {
        const n = c.value(v);
        if (n <= 0) return null;
        return <div key={c.label}>{c.label}：{formatNumber(n)}</div>;
      })}
    </div>
  );

  return (
    <Tooltip content={tooltipContent}>
      <span style={{ whiteSpace: 'nowrap', cursor: 'default' }}>
        ↑{formatNumber(inputSum)} / ↓{formatNumber(outputSum)}
      </span>
    </Tooltip>
  );
}

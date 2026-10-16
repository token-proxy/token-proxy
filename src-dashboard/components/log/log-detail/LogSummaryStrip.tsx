import { type ReactNode } from 'react';
import type { LogDetailFull } from '../../../types/log.ts';
import { formatDateTime, formatNumber } from '../../../utils/format.ts';
import './LogSummaryStrip.css';

/** LogSummaryStrip 组件 Props */
interface LogSummaryStripProps {
  data: LogDetailFull;
}

/** 词元指标（顺序固定，六列等宽对齐） */
const TOKEN_METRICS: ReadonlyArray<{ key: string; label: string }> = [
  { key: 'token_total_tokens', label: '总计' },
  { key: 'token_input_tokens', label: '未命中缓存' },
  { key: 'token_cache_read_input_tokens', label: '缓存命中输入' },
  { key: 'token_cache_creation_input_tokens', label: '缓存创建输入' },
  { key: 'token_output_tokens', label: '输出' },
  { key: 'token_thinking_tokens', label: '思考' },
];

/**
 * LogSummaryStrip - 详情页顶部的一行式汇总条
 *
 * 基础信息与词元用量压成**一张卡两行**：第一行键值对inline 排布、第二行六个
 * 等宽列对齐的数字。目的是把纵向空间让给下方的运行轨迹（滚动只发生在内部容器）。
 */
export default function LogSummaryStrip({ data }: LogSummaryStripProps): ReactNode {
  const basics: Array<[string, ReactNode]> = [
    ['时间', formatDateTime(data.timestamp)],
    ['状态码', String(data.status_code ?? '-')],
    ['耗时', `${formatNumber(data.duration_ms ?? 0)} ms`],
    [
      '模型',
      data.model_original && data.model_mapped && data.model_original !== data.model_mapped
        ? `${data.model_original} → ${data.model_mapped}`
        : data.model_mapped || data.model_original || '-',
    ],
    ['来源', data.conversation_source || '-'],
    ['客户端', [data.client_type, data.client_version].filter(Boolean).join(' ') || '-'],
    ['接入点', data.access_point_name || '-'],
    ['会话', data.session_id || '-'],
  ];

  return (
    <section className="log-summary">
      <div className="log-summary__basics">
        {basics.map(([label, value]) => (
          <span className="log-summary__item" key={label}>
            <span className="log-summary__label">{label}</span>
            <span className="log-summary__value">{value}</span>
          </span>
        ))}
      </div>

      <div className="log-summary__tokens">
        {TOKEN_METRICS.map(({ key, label }) => (
          <span className="log-summary__metric" key={key}>
            <span className="log-summary__label">{label}</span>
            <span className="log-summary__number">
              {formatNumber((data[key as keyof LogDetailFull] as number | null) ?? 0)}
            </span>
          </span>
        ))}
      </div>
    </section>
  );
}

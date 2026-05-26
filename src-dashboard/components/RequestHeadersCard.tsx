import { type ReactNode } from 'react';
import CollapsibleCard from './CollapsibleCard.tsx';

const SENSITIVE_HEADER_KEYS = new Set([
  'authorization',
  'x-api-key',
  'api-key',
  'proxy-authorization',
]);

export function formatHeadersWithMasking(
  headers: Record<string, unknown> | null | undefined,
): string {
  if (!headers) return '(无请求头)';

  return Object.entries(headers)
    .map(([key, value]) => {
      if (SENSITIVE_HEADER_KEYS.has(key.toLowerCase())) {
        return `${key}: [已隐藏]`;
      }
      return `${key}: ${String(value)}`;
    })
    .join('\n');
}

interface RequestHeadersCardProps {
  headers: Record<string, unknown> | null | undefined;
  style?: React.CSSProperties;
}

export default function RequestHeadersCard({ headers, style }: RequestHeadersCardProps): ReactNode {
  const formatted = formatHeadersWithMasking(headers);

  return (
    <div style={style}>
      <CollapsibleCard
        title="请求头"
        defaultCollapsed
        copyText={formatted}
        bodyStyle={{ padding: '0 24px 20px' }}
      >
        <pre
          style={{
            background: 'var(--semi-color-fill-0)',
            padding: 12,
            borderRadius: 4,
            fontSize: 12,
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-all',
            margin: 0,
          }}
        >
          {formatted}
        </pre>
      </CollapsibleCard>
    </div>
  );
}

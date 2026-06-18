import { type ReactNode, useMemo, useState } from 'react';
import { Button, Descriptions } from '@douyinfe/semi-ui';
import CollapsibleCard from '@components/common/CollapsibleCard';
import { IconCopy } from '@douyinfe/semi-icons';
import Text from '@douyinfe/semi-ui/lib/es/typography/text';

function formatHeadersToArray(
  headers: Record<string, unknown> | null | undefined,
): Array<{ key: string; value: string }> {
  if (!headers) return [];
  return Object.entries(headers).map(([key, value]) => ({
    key,
    value: String(value),
  }));
}

function formatHeadersForCopy(
  headers: Record<string, unknown> | null | undefined,
): string {
  if (!headers) return '';

  return Object.entries(headers)
    .map(([key, value]) => {
      return `${key}: ${String(value)}`;
    })
    .join('\n');
}

interface HadersCardProps {
  headers: Record<string, unknown> | null | undefined;
  style?: React.CSSProperties;
  title: String;
}

export default function HeadersCard({headers, style, title}: HadersCardProps): ReactNode {
  const items = formatHeadersToArray(headers);
  const [copying, setCopying] = useState(false);
  const copyText = useMemo(() => formatHeadersForCopy(headers), [headers]);

  const handleCopy = async () => {
    if (!copyText) return;
    setCopying(true);
    try {
      await navigator.clipboard.writeText(copyText);
    } finally {
      setCopying(false);
    }
  };

  const copyButton = (
    <Button
      icon={<IconCopy/>}
      size="small"
      type="tertiary"
      loading={copying}
      onClick={(e) => {
        e.stopPropagation();
        e.preventDefault();
        handleCopy();
      }}
    >
      复制
    </Button>
  );

  return (
    <CollapsibleCard
      title={title}
      defaultCollapsed
      headerExtraContent={copyButton}
      style={style}
    >
      {items.length > 0
        ? <Descriptions data={items} size="small"/>
        : <Text>无{title}</Text>
      }
    </CollapsibleCard>
  );
}

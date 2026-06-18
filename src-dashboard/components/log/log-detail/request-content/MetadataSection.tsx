import { type ReactNode } from 'react';
import { Descriptions } from '@douyinfe/semi-ui';
import SectionHeading from './SectionHeading';

interface MetadataSectionProps {
  metadata: unknown;
}

export default function MetadataSection({
  metadata,
}: MetadataSectionProps): ReactNode {
  if (metadata == null) return null;

  const metaObj: Record<string, unknown> =
    typeof metadata === 'string'
      ? (() => {
        try {
          return JSON.parse(metadata);
        } catch {
          return {};
        }
      })()
      : (metadata as Record<string, unknown>);

  const entries = Object.entries(metaObj);
  if (entries.length === 0) return null;

  return (
    <div style={{marginBottom: 24}}>
      <SectionHeading>元数据</SectionHeading>
      <Descriptions
        row
        size="small"
        data={entries.map(([k, v]) => ({
          key: k,
          value: typeof v === 'string' ? v : JSON.stringify(v),
        }))}
      />
    </div>
  );
}

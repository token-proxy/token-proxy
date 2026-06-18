import { Card, Spin, Typography } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { TrendItem } from '../../types/dashboard.ts';

const {Text} = Typography;

interface TrendChartProps {
  data: TrendItem[];
  loading: boolean;
}

export default function TrendChart({data, loading}: TrendChartProps): ReactNode {
  const maxCount = Math.max(...data.map((d) => d.count), 1);
  const barMaxHeight = 180;

  return (
    <Card
      title="请求趋势 (近 7 天)"
      style={{backgroundColor: 'var(--semi-color-bg-0)'}}
    >
      {loading ? (
        <div style={{textAlign: 'center', padding: '60px 0'}}>
          <Spin/>
        </div>
      ) : (
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-end',
            gap: 12,
            paddingTop: 24,
            paddingBottom: 4,
          }}
        >
          {data.map((item) => {
            const barHeight = Math.max((item.count / maxCount) * barMaxHeight, 4);
            return (
              <div
                key={item.date}
                style={{
                  flex: 1,
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                }}
              >
                <Text
                  type="secondary"
                  style={{fontSize: 11, marginBottom: 4, whiteSpace: 'nowrap'}}
                >
                  {item.count >= 1000
                    ? `${(item.count / 1000).toFixed(1)}K`
                    : item.count}
                </Text>
                <div
                  style={{
                    width: '100%',
                    maxWidth: 40,
                    height: barHeight,
                    backgroundColor: 'var(--semi-color-primary)',
                    borderRadius: '4px 4px 0 0',
                    transition: 'height 0.3s ease',
                    opacity: 0.85,
                  }}
                />
                <Text
                  type="secondary"
                  style={{fontSize: 12, marginTop: 8}}
                >
                  {item.date}
                </Text>
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}

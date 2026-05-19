import type { ReactNode } from 'react';
import { Card, Typography } from '@douyinfe/semi-ui';

const { Title, Text } = Typography;

export default function DashboardPage(): ReactNode {
  return (
    <div>
      <Title heading={3} style={{ marginBottom: 24 }}>Dashboard</Title>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: 16 }}>
        <Card title="Provider 数量" style={{ minHeight: 120 }}>
          <Text type="secondary">统计数据将在 Phase 2 实现</Text>
        </Card>
        <Card title="接入点数量" style={{ minHeight: 120 }}>
          <Text type="secondary">统计数据将在 Phase 2 实现</Text>
        </Card>
        <Card title="今日请求量" style={{ minHeight: 120 }}>
          <Text type="secondary">统计数据将在 Phase 2 实现</Text>
        </Card>
        <Card title="活跃会话" style={{ minHeight: 120 }}>
          <Text type="secondary">统计数据将在 Phase 2 实现</Text>
        </Card>
      </div>
    </div>
  );
}
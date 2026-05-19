import type { ReactNode } from 'react';
import { Card, Typography } from '@douyinfe/semi-ui';

const { Title, Text } = Typography;

export default function SessionLogPage(): ReactNode {
  return (
    <div>
      <Title heading={3} style={{ marginBottom: 24 }}>会话日志</Title>
      <Card>
        <Text type="secondary">会话日志功能将在后续版本实现</Text>
      </Card>
    </div>
  );
}
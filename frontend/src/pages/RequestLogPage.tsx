import type { ReactNode } from 'react';
import { Card, Typography } from '@douyinfe/semi-ui';

const { Title, Text } = Typography;

export default function RequestLogPage(): ReactNode {
  return (
    <div>
      <Title heading={3} style={{ marginBottom: 24 }}>请求日志</Title>
      <Card>
        <Text type="secondary">请求日志功能将在后续版本实现</Text>
      </Card>
    </div>
  );
}
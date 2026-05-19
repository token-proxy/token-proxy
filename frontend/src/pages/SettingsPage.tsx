import type { ReactNode } from 'react';
import { Card, Typography } from '@douyinfe/semi-ui';

const { Title, Text } = Typography;

export default function SettingsPage(): ReactNode {
  return (
    <div>
      <Title heading={3} style={{ marginBottom: 24 }}>系统设置</Title>
      <Card>
        <Text type="secondary">系统设置功能将在后续版本实现</Text>
      </Card>
    </div>
  );
}
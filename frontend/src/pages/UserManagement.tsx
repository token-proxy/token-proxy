import type { ReactNode } from 'react';
import { Card, Typography } from '@douyinfe/semi-ui';

const { Title, Text } = Typography;

export default function UserManagement(): ReactNode {
  return (
    <div>
      <Title heading={3} style={{ marginBottom: 24 }}>用户管理</Title>
      <Card>
        <Text type="secondary">用户管理功能将在后续版本实现</Text>
      </Card>
    </div>
  );
}
import { type ReactNode } from 'react';
import { Typography } from '@douyinfe/semi-ui';

const { Text } = Typography;

interface SectionHeadingProps {
  children: ReactNode;
}

/**
 * SectionHeading - 区块标题组件
 *
 * 提供统一的加粗标题样式。
 */
export default function SectionHeading({ children }: SectionHeadingProps): ReactNode {
  return (
    <Text
      strong
      style={{
        display: 'block',
        marginBottom: 12,
        fontSize: 16,
        color: 'var(--semi-color-text-0)',
      }}
    >
      {children}
    </Text>
  );
}

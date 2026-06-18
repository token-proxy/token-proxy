import { type ReactNode } from 'react';
import { Typography } from '@douyinfe/semi-ui';

const {Text} = Typography;

interface SectionHeadingProps {
  children: ReactNode;
}

export default function SectionHeading({
  children,
}: SectionHeadingProps): ReactNode {
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

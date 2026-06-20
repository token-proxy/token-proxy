import { type KeyboardEvent, type MouseEvent, type ReactNode } from 'react';
import { Typography } from '@douyinfe/semi-ui';
import { IconCopy, IconTick } from '@douyinfe/semi-icons';

const {Text} = Typography;

/** CopyableIdText 组件 Props */
type CopyableIdTextProps = {
  /** 要显示的 ID 文本 */
  value: string;
};

/**
 * CopyableIdText - 可复制的 ID 文本组件
 *
 * 使用 Semi Typography Text 的 copyable 功能，展示 monospace 样式的 ID。
 */
export default function CopyableIdText({value}: CopyableIdTextProps): ReactNode {
  return (
    <Text
      className="nowrap-text"
      copyable={{content: value, render: renderCopyIcon}}
    >
      <span className="monospace-text">{value}</span>
    </Text>
  );
}

function renderCopyIcon(
  copied: boolean,
  doCopy: (e: MouseEvent) => void,
) {
  const Icon = copied ? IconTick : IconCopy;
  const className = copied ? 'copyable-id-icon copyable-id-icon-success' : 'copyable-id-icon';

  return (
    <span
      className={className}
      role="button"
      tabIndex={0}
      onClick={doCopy}
      onKeyDown={(e: KeyboardEvent<HTMLSpanElement>) => {
        if (e.key === 'Enter') {
          doCopy(e as unknown as MouseEvent);
        }
      }}
    >
      <Icon/>
    </span>
  );
}

import { type ReactNode, useState } from 'react';
import { Button, Toast } from '@douyinfe/semi-ui';
import { IconCopy } from '@douyinfe/semi-icons';
import CodeHighlight from '@components/common/CodeHighlight';

/** RawResponseView 组件 Props */
interface RawResponseViewProps {
  body: string;
}

/**
 * RawResponseView - 原始响应内容展示组件
 *
 * 展示原始 SSE 响应文本，支持一键复制。
 */
export default function RawResponseView({ body }: RawResponseViewProps): ReactNode {
  const [copying, setCopying] = useState(false);

  const handleCopy = async () => {
    setCopying(true);
    try {
      await navigator.clipboard.writeText(body);
      Toast.success('已复制到剪贴板');
    } catch {
      Toast.error('复制失败，请手动复制');
    } finally {
      setCopying(false);
    }
  };

  return (
    <>
      <div
        style={{
          display: 'flex',
          justifyContent: 'flex-end',
          marginBottom: 8,
        }}
      >
        <Button
          icon={<IconCopy />}
          size="small"
          type="tertiary"
          loading={copying}
          onClick={handleCopy}
        >
          复制
        </Button>
      </div>
      <CodeHighlight content={body || '(空)'} />
    </>
  );
}

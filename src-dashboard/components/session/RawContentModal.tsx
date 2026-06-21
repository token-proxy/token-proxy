import { Modal } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';

/** RawContentModal 组件 Props */
interface RawContentModalProps {
  title: string;
  visible: boolean;
  content: string;
  onClose: () => void;
}

/**
 * RawContentModal - 原始日志内容弹窗组件
 *
 * 以 Modal 弹窗形式展示日志的原始请求/响应内容。
 */
export default function RawContentModal({
  title,
  visible,
  content,
  onClose,
}: RawContentModalProps): ReactNode {
  return (
    <Modal
      title={title}
      visible={visible}
      onCancel={onClose}
      onOk={onClose}
      width={800}
      style={{ maxHeight: '80vh' }}
    >
      <pre
        style={{
          background: 'var(--semi-color-fill-0)',
          padding: 16,
          borderRadius: 4,
          fontSize: 12,
          overflow: 'auto',
          maxHeight: 500,
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-all',
        }}
      >
        {content}
      </pre>
    </Modal>
  );
}

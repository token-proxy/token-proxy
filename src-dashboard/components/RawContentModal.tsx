import { Modal } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';

interface RawContentModalProps {
  title: string;
  visible: boolean;
  content: string;
  onClose: () => void;
}

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

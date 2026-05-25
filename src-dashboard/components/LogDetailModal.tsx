import { IconCopy } from '@douyinfe/semi-icons';
import { Button, Empty, Modal, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { type ReactNode, useState } from 'react';
import type { LogDetail } from '../types/log.ts';
import { formatDateTime, formatDuration } from '../utils/format.ts';

const { Text } = Typography;

type CopyTarget = 'headers' | 'request' | 'response';

interface LogDetailModalProps {
  visible: boolean;
  loading: boolean;
  data: LogDetail | null;
  onClose: () => void;
}

export default function LogDetailModal({
  visible,
  loading,
  data,
  onClose,
}: LogDetailModalProps): ReactNode {
  const [copyingTarget, setCopyingTarget] = useState<CopyTarget | null>(null);

  const copyContent = async (target: CopyTarget, content: string) => {
    setCopyingTarget(target);
    try {
      await navigator.clipboard.writeText(content);
      Toast.success('已复制到剪贴板');
    } catch {
      Toast.error('复制失败，请手动复制');
    } finally {
      setCopyingTarget(null);
    }
  };

  const renderContentBlock = (target: CopyTarget, title: string, content: string, maxHeight: number, marginTop = 0) => (
    <div style={{ marginTop }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
        <Text strong>{title}:</Text>
        <Button
          icon={<IconCopy />}
          size="small"
          type="tertiary"
          loading={copyingTarget === target}
          disabled={copyingTarget !== null && copyingTarget !== target}
          onClick={() => copyContent(target, content)}
        >
          复制
        </Button>
      </div>
      <pre
        style={{
          background: 'var(--semi-color-fill-0)',
          padding: 12,
          borderRadius: 4,
          fontSize: 12,
          overflow: 'auto',
          maxHeight,
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-all',
        }}
      >
        {content}
      </pre>
    </div>
  );

  return (
    <Modal
      title="请求详情"
      visible={visible}
      onCancel={onClose}
      onOk={onClose}
      width={900}
      style={{ maxHeight: '80vh' }}
      footer={
        <Button type="primary" onClick={onClose}>关闭</Button>
      }
    >
      {loading ? (
        <div style={{ textAlign: 'center', padding: 40 }}>
          <Text type="secondary">加载中...</Text>
        </div>
      ) : data ? (
        <div>
          <div
            style={{
              background: 'var(--semi-color-fill-0)',
              borderRadius: 8,
              padding: 12,
              marginBottom: 16,
              display: 'flex',
              flexDirection: 'column',
              gap: 6,
              fontSize: 13,
            }}
          >
            <Text>
              <strong>时间:</strong> {formatDateTime(data.timestamp)}
            </Text>
            <Text>
              <strong>会话 ID:</strong>
              {' '}
              <span style={{ fontFamily: 'monospace', fontSize: 12 }}>
                {data.session_id}
              </span>
            </Text>
            <Text>
              <strong>模型:</strong> {data.model_original || '-'}
              {' '}
              &rarr;
              {' '}
              {data.model_mapped || '-'}
            </Text>
            <Text>
              <strong>状态码:</strong>
              {' '}
              <Tag
                color={(data.status_code ?? 0) >= 400 ? 'red' : 'green'}
                size="small"
              >
                {data.status_code ?? '-'}
              </Tag>
              <strong style={{ marginLeft: 16 }}>耗时:</strong>
              {' '}
              {formatDuration(data.duration_ms)}
            </Text>
            {data.error_message && (
              <Text>
                <strong style={{ color: 'var(--semi-color-danger)' }}>错误:</strong>
                {' '}
                {data.error_message}
              </Text>
            )}
          </div>

          {renderContentBlock(
            'headers',
            '请求头',
            JSON.stringify(data.request_headers, null, 2) || '(空)',
            200,
          )}
          {renderContentBlock(
            'request',
            '请求体',
            JSON.stringify(data.request_body, null, 2) || '(空)',
            300,
            12,
          )}
          {renderContentBlock(
            'response',
            '响应体',
            data.response_body || '(空)',
            400,
            12,
          )}
        </div>
      ) : (
        <Empty description="暂无数据" />
      )}
    </Modal>
  );
}

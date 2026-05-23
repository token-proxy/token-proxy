import { Button, Empty, Modal, Tag, Typography } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { LogDetail } from '../types/log.ts';
import { formatDateTime, formatDuration } from '../utils/format.ts';

const { Text } = Typography;

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

          <Text strong style={{ display: 'block', marginBottom: 4 }}>请求头:</Text>
          <pre
            style={{
              background: 'var(--semi-color-fill-0)',
              padding: 12,
              borderRadius: 4,
              fontSize: 12,
              overflow: 'auto',
              maxHeight: 200,
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
            }}
          >
            {JSON.stringify(data.request_headers, null, 2) || '(空)'}
          </pre>

          <Text strong style={{ display: 'block', marginTop: 12, marginBottom: 4 }}>请求体:</Text>
          <pre
            style={{
              background: 'var(--semi-color-fill-0)',
              padding: 12,
              borderRadius: 4,
              fontSize: 12,
              overflow: 'auto',
              maxHeight: 300,
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
            }}
          >
            {JSON.stringify(data.request_body, null, 2) || '(空)'}
          </pre>

          <Text strong style={{ display: 'block', marginTop: 12, marginBottom: 4 }}>响应体:</Text>
          <pre
            style={{
              background: 'var(--semi-color-fill-0)',
              padding: 12,
              borderRadius: 4,
              fontSize: 12,
              overflow: 'auto',
              maxHeight: 400,
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
            }}
          >
            {data.response_body || '(空)'}
          </pre>
        </div>
      ) : (
        <Empty description="暂无数据" />
      )}
    </Modal>
  );
}

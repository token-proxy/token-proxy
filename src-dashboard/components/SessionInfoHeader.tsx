import { Typography } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { LogSummary } from '../types/log.ts';
import { formatDateTime } from '../utils/format.ts';

const { Text } = Typography;

interface SessionInfoHeaderProps {
  sessionId: string;
  sessionLogs: LogSummary[];
  userMap: Record<string, string>;
}

export default function SessionInfoHeader({
  sessionId,
  sessionLogs,
  userMap,
}: SessionInfoHeaderProps): ReactNode {
  return (
    <div
      style={{
        background: 'var(--semi-color-fill-0)',
        borderRadius: 8,
        padding: 16,
        marginBottom: 24,
      }}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        <Text>
          <strong>会话 ID:</strong>
          {' '}
          <span style={{ fontFamily: 'monospace', fontSize: 13 }}>{sessionId}</span>
        </Text>
        <Text><strong>请求总数:</strong> {sessionLogs.length}</Text>
        {sessionLogs.length > 0 && (
          <>
            <Text>
              <strong>时间范围:</strong>
              {' '}
              {formatDateTime(sessionLogs[0].timestamp)}
              {' '}
              ~
              {' '}
              {formatDateTime(sessionLogs[sessionLogs.length - 1].timestamp)}
            </Text>
            <Text>
              <strong>用户:</strong>
              {' '}
              {(() => {
                const uid = sessionLogs[0].user_id;
                return uid ? (userMap[uid] || uid) : '-';
              })()}
            </Text>
          </>
        )}
      </div>
    </div>
  );
}

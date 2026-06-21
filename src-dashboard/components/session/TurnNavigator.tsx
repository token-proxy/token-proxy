import type { ReactNode } from 'react';
import { useMemo } from 'react';
import { Button, Tag, Tooltip } from '@douyinfe/semi-ui';
import type { ConversationTurn } from '../../types/log.ts';

interface TurnNavigatorProps {
  turns: ConversationTurn[];
  activeTurnId: string;
  onTurnClick: (turnId: string) => void;
}

/**
 * 轮次导航条
 *
 * Sticky 定位的横向导航组件，帮助用户快速定位长会话中的特定轮次。
 * 高亮当前视口内的轮次，标记包含子代理调用的轮次。
 *
 * 轮次数 <= 1 时不渲染，因无导航必要。
 */
export default function TurnNavigator({
  turns,
  activeTurnId,
  onTurnClick,
}: TurnNavigatorProps): ReactNode {
  // 1. 检查是否有子代理 block
  const hasAgentCall = useMemo(() => {
    const map = new Map<string, boolean>();
    for (const turn of turns) {
      map.set(
        turn.id,
        turn.blocks.some((b) => b.type === 'agent_call'),
      );
    }
    return map;
  }, [turns]);

  if (turns.length <= 1) return null;

  return (
    <div
      style={{
        position: 'sticky',
        top: 0,
        zIndex: 10,
        background: 'var(--semi-color-bg-1)',
        borderBottom: '1px solid var(--semi-color-border)',
        padding: '8px 16px',
        overflowX: 'auto',
        whiteSpace: 'nowrap',
        display: 'flex',
        alignItems: 'center',
        gap: 8,
      }}
    >
      {turns.map((turn) => {
        const isActive = turn.id === activeTurnId;
        const showAgentTag = hasAgentCall.get(turn.id) ?? false;

        const label = (
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
            <span>{`轮次 ${turn.turnIndex}`}</span>
            {showAgentTag && (
              <Tag
                size="small"
                color="green"
                style={{ margin: 0, padding: '0 4px', lineHeight: '16px', fontSize: 11 }}
              >
                子代理
              </Tag>
            )}
          </span>
        );

        return (
          <Tooltip
            key={turn.id}
            content={
              turn.userMessage
                ? turn.userMessage.length > 60
                  ? turn.userMessage.slice(0, 60) + '...'
                  : turn.userMessage
                : '(空消息)'
            }
            mouseEnterDelay={300}
          >
            <Button
              size="small"
              theme={isActive ? 'solid' : 'light'}
              type={isActive ? 'primary' : 'tertiary'}
              style={{
                flexShrink: 0,
                fontWeight: isActive ? 600 : 400,
              }}
              onClick={() => onTurnClick(turn.id)}
            >
              {label}
            </Button>
          </Tooltip>
        );
      })}
    </div>
  );
}

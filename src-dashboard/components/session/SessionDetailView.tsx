import type { ReactNode } from 'react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { Button, Empty, Spin, Tag, Typography } from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import TurnCard from './TurnCard.tsx';
import TurnNavigator from './TurnNavigator.tsx';
import RawContentModal from './RawContentModal.tsx';
import type { ConversationTurn } from '../../types/log.ts';
import { formatDateTime, formatNumber } from '../../utils/format.ts';

const { Title, Text } = Typography;

/**
 * SessionDetailView 组件 Props
 *
 * 以轮次（turn）视图替代原有的事件时间线 + 事件摘要表格布局，
 * 每个轮次包含用户消息、模型响应、工具调用、子代理调用等完整内容。
 */
interface SessionDetailViewProps {
  /** 会话 ID */
  sessionId: string;
  /** 构建完成的轮次数据列表 */
  turns: ConversationTurn[];
  /** 是否正在加载轮次详情 */
  detailLoading: boolean;
  /** 返回会话列表的回调 */
  onBack: () => void;
  /** 刷新会话详情的回调 */
  onRefresh: () => void;
  /** 打开原始内容弹窗的回调（传入 logId） */
  onOpenRaw: (logId: string) => void;
  /** 原始内容弹窗可见性 */
  rawModalVisible: boolean;
  /** 原始内容弹窗标题 */
  rawModalTitle: string;
  /** 原始内容弹窗内容 */
  rawModalContent: string;
  /** 关闭原始内容弹窗的回调 */
  onCloseRawModal: () => void;
}

/**
 * SessionDetailView - 会话详情视图组件
 *
 * 以轮次导航条 + 轮次卡片列表展示单个会话的完整对话内容。
 * 支持轮次滚动导航、会话级 Token 汇总、展开收起等交互。
 */
export default function SessionDetailView({
  sessionId,
  turns,
  detailLoading,
  onBack,
  onRefresh,
  onOpenRaw,
  rawModalVisible,
  rawModalTitle,
  rawModalContent,
  onCloseRawModal,
}: SessionDetailViewProps): ReactNode {
  // 当前活跃（在视口内）的轮次 ID，用于导航条高亮
  const [activeTurnId, setActiveTurnId] = useState<string>('');
  // 每个轮次卡片的 DOM 引用，用于滚动导航
  const turnRefs = useRef<Record<string, HTMLDivElement | null>>({});

  // --- 轮次滚动导航 ---

  /** 通过 IntersectionObserver 监控轮次卡片在视口中的可见性，更新 activeTurnId */
  useEffect(() => {
    const elements = Object.values(turnRefs.current).filter(Boolean) as HTMLDivElement[];
    if (elements.length === 0) return;

    const observer = new IntersectionObserver(
      (entries) => {
        let bestEntry: IntersectionObserverEntry | null = null;
        for (const entry of entries) {
          if (!bestEntry || entry.intersectionRatio > bestEntry.intersectionRatio) {
            bestEntry = entry;
          }
        }
        if (bestEntry && bestEntry.intersectionRatio > 0) {
          setActiveTurnId(bestEntry.target.getAttribute('data-turn-id') ?? '');
        }
      },
      { threshold: [0, 0.25, 0.5, 0.75, 1] },
    );

    for (const el of elements) {
      observer.observe(el);
    }

    return () => {
      observer.disconnect();
    };
  }, [turns]);

  /** 点击导航条中的轮次按钮时，滚动到对应轮次卡片 */
  const handleTurnClick = useCallback((turnId: string) => {
    const el = turnRefs.current[turnId];
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }, []);

  // --- 会话级 Token 汇总 ---

  const tokenTotals = turns.reduce(
    (acc, turn) => ({
      inputTokens: acc.inputTokens + turn.tokenSummary.inputTokens,
      outputTokens: acc.outputTokens + turn.tokenSummary.outputTokens,
      cacheCreationTokens: acc.cacheCreationTokens + turn.tokenSummary.cacheCreationTokens,
      cacheReadTokens: acc.cacheReadTokens + turn.tokenSummary.cacheReadTokens,
      thinkingTokens: acc.thinkingTokens + turn.tokenSummary.thinkingTokens,
      totalTokens: acc.totalTokens + turn.tokenSummary.totalTokens,
    }),
    {
      inputTokens: 0,
      outputTokens: 0,
      cacheCreationTokens: 0,
      cacheReadTokens: 0,
      thinkingTokens: 0,
      totalTokens: 0,
    },
  );

  return (
    <div>
      {/* ─── 顶部工具栏 ─── */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
        <Button type="tertiary" onClick={onBack}>
          &larr; 返回会话列表
        </Button>
        <Title heading={3} style={{ margin: 0 }}>
          会话详情
        </Title>
        <Button icon={<IconRefresh />} loading={detailLoading} onClick={onRefresh}>
          刷新
        </Button>
      </div>

      {/* ─── 会话概览卡片 ─── */}
      <div
        style={{
          background: 'var(--semi-color-fill-0)',
          borderRadius: 8,
          padding: 16,
          marginBottom: 16,
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <Text>
            <strong>会话 ID:</strong>{' '}
            <span style={{ fontFamily: 'monospace', fontSize: 13 }}>{sessionId}</span>
          </Text>
          <Text>
            <strong>轮次总数:</strong> {turns.length}
          </Text>
          <Text>
            <strong>事件总数:</strong> {turns.reduce((sum, t) => sum + t.blocks.length, 0)}
          </Text>
          {turns.length > 0 && (
            <>
              <Text>
                <strong>时间范围:</strong> {formatDateTime(turns[0].startTime)} ~{' '}
                {formatDateTime(turns[turns.length - 1].endTime)}
              </Text>
              {/* 会话级 Token 汇总 */}
              <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginTop: 4 }}>
                <Tag color="light-blue">总输入 &uarr;{formatNumber(tokenTotals.inputTokens)}</Tag>
                <Tag color="light-blue">总输出 &darr;{formatNumber(tokenTotals.outputTokens)}</Tag>
                {tokenTotals.cacheCreationTokens > 0 && (
                  <Tag color="teal">缓存创建 {formatNumber(tokenTotals.cacheCreationTokens)}</Tag>
                )}
                {tokenTotals.cacheReadTokens > 0 && (
                  <Tag color="teal">缓存读取 {formatNumber(tokenTotals.cacheReadTokens)}</Tag>
                )}
                {tokenTotals.thinkingTokens > 0 && (
                  <Tag color="amber">思考 {formatNumber(tokenTotals.thinkingTokens)}</Tag>
                )}
                <Tag>总计 {formatNumber(tokenTotals.totalTokens)}</Tag>
              </div>
            </>
          )}
        </div>
      </div>

      {/* ─── 轮次导航条 ─── */}
      <TurnNavigator turns={turns} activeTurnId={activeTurnId} onTurnClick={handleTurnClick} />

      {/* ─── 轮次卡片列表 ─── */}
      {detailLoading ? (
        <div style={{ textAlign: 'center', padding: 40 }}>
          <Spin />
          <Text type="secondary" style={{ display: 'block', marginTop: 8 }}>
            加载中...
          </Text>
        </div>
      ) : turns.length === 0 ? (
        <Empty description="暂无对话数据" />
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16, marginTop: 16 }}>
          {turns.map((turn) => (
            <div
              key={turn.id}
              ref={(el) => {
                turnRefs.current[turn.id] = el;
              }}
              data-turn-id={turn.id}
            >
              <TurnCard turn={turn} onOpenRaw={onOpenRaw} defaultExpanded={true} />
            </div>
          ))}
        </div>
      )}

      {/* ─── 原始内容弹窗 ─── */}
      <RawContentModal
        title={rawModalTitle}
        visible={rawModalVisible}
        content={rawModalContent}
        onClose={onCloseRawModal}
      />
    </div>
  );
}

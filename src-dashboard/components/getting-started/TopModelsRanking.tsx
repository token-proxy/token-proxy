/**
 * TopModelsRanking - 模型使用排行卡片。
 *
 * 横向条形列表（最多 8 行），按 total_tokens 归一化条宽。
 * 每行展示模型名 + 横条 + 右侧文字（紧凑词元 · 请求次数 · 占比）。
 *
 * 与 TopAccessPointsRanking 共用同一视觉范式，但不包含「已删除」降级
 * （模型名是请求体的原始字符串，不存在删除概念）。
 */

import { Card, Skeleton } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { TopModelItem } from '../../types/dashboard';
import { formatTokenCompact } from '../../utils/format';

/**
 * TopModelsRanking 组件 Props。
 */
interface TopModelsRankingProps {
  /** 排行项数组（按 total_tokens 降序，已由后端排好） */
  items: TopModelItem[];
  /** 加载态：true 时渲染 Skeleton 占位 */
  loading?: boolean;
}

/** 最多展示行数 */
const MAX_ROWS = 8;

/**
 * 模型排行卡片。
 *
 * 视觉策略：
 * - 每行高度固定 28px，行间留 8px gap，整体节奏稳定
 * - 横条宽度按 `item.total_tokens / max * 100%` 归一化，max 取榜首值
 * - 横条颜色采用 Semi 主色浅变体，避免与卡片背景冲突
 * - 右侧文字使用 tabular-nums 等宽数字，对齐美观
 *
 * 边界处理：
 * - items 为空 → 渲染空态文本
 * - loading=true → 整卡 Skeleton
 * - max=0 → 横条宽度退化为 0（仍渲染文字行）
 *
 * @example
 * <TopModelsRanking items={modelsResponse.items} />
 */
export function TopModelsRanking({ items, loading = false }: TopModelsRankingProps): ReactNode {
  // 1. 加载态
  if (loading) {
    return (
      <Card
        bordered={false}
        style={{ backgroundColor: 'var(--semi-color-bg-2)', borderRadius: 12 }}
        bodyStyle={{ padding: 20 }}
      >
        <Skeleton active placeholder={<Skeleton.Paragraph rows={5} />} loading={true} />
      </Card>
    );
  }

  // 2. 截取并计算归一化基准
  const displayItems = items.slice(0, MAX_ROWS);
  const max = displayItems.reduce((acc, item) => Math.max(acc, item.total_tokens), 0);
  const totalSum = displayItems.reduce((acc, item) => acc + item.total_tokens, 0);

  return (
    <Card
      bordered={false}
      style={{ backgroundColor: 'var(--semi-color-bg-2)', borderRadius: 12 }}
      bodyStyle={{ padding: 20 }}
    >
      {/* 标题 */}
      <div
        style={{
          fontSize: 14,
          fontWeight: 500,
          letterSpacing: '0.02em',
          color: 'var(--semi-color-text-2)',
          marginBottom: 14,
        }}
      >
        模型排行
      </div>

      {/* 空态 */}
      {displayItems.length === 0 ? (
        <div
          style={{
            padding: '24px 0',
            textAlign: 'center',
            color: 'var(--semi-color-text-2)',
            fontSize: 13,
          }}
        >
          暂无模型使用记录
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {displayItems.map((item) => {
            // 横条宽度归一化（max=0 时为 0）
            const widthPct = max > 0 ? (item.total_tokens / max) * 100 : 0;
            // 占比：基于当前展示集合的总和（让头部模型占比直观）
            const pct = totalSum > 0 ? ((item.total_tokens / totalSum) * 100).toFixed(1) : '0.0';

            return (
              <div
                key={item.model}
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 4,
                }}
              >
                {/* 上行：模型名 + 数值 */}
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    gap: 8,
                    fontSize: 12,
                  }}
                >
                  <span
                    style={{
                      color: 'var(--semi-color-text-0)',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                    title={item.model}
                  >
                    {item.model}
                  </span>
                  <span
                    style={{
                      color: 'var(--semi-color-text-2)',
                      fontVariantNumeric: 'tabular-nums',
                      flexShrink: 0,
                    }}
                  >
                    {formatTokenCompact(item.total_tokens)} · {item.request_count} 次 · {pct}%
                  </span>
                </div>

                {/* 下行：横条 */}
                <div
                  style={{
                    width: '100%',
                    height: 6,
                    borderRadius: 3,
                    backgroundColor: 'var(--semi-color-fill-1)',
                    overflow: 'hidden',
                  }}
                  role="img"
                  aria-label={`${item.model} 占比 ${pct}%`}
                >
                  <div
                    style={{
                      width: `${widthPct}%`,
                      height: '100%',
                      backgroundColor: 'var(--semi-color-primary-light-default)',
                      transition: 'width 200ms ease-out',
                    }}
                  />
                </div>
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}

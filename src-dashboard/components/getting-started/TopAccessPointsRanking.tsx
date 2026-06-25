/**
 * TopAccessPointsRanking - 接入点使用排行卡片。
 *
 * 横向条形列表（最多 5 行），按 total_tokens 归一化条宽。
 * 当接入点已被删除（name 为 null）时降级展示，附 `.getting-started-deleted` CSS class。
 */

import { Card, Skeleton } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { TopAccessPointItem } from '../../types/dashboard';
import { formatTokenCompact } from '../../utils/format';

/**
 * TopAccessPointsRanking 组件 Props。
 */
interface TopAccessPointsRankingProps {
  /** 排行项数组（按 total_tokens 降序，已由后端排好） */
  items: TopAccessPointItem[];
  /** 加载态：true 时渲染 Skeleton 占位 */
  loading?: boolean;
}

/** 最多展示行数 */
const MAX_ROWS = 5;

/**
 * 接入点排行卡片。
 *
 * 视觉策略：
 * - 与 TopModelsRanking 同构（上行名称 + 数值、下行横条），保持视觉节奏一致
 * - 删除态降级：name 为 null 时显示「已删除接入点 · <short_code 或 uuid 前 8 位>」
 *   并附 `.getting-started-deleted` CSS class（全局样式：灰色 + 等宽字体）
 *
 * 边界处理：
 * - items 为空 → 渲染空态文本
 * - loading=true → 整卡 Skeleton
 * - max=0 → 横条宽度退化为 0
 *
 * @example
 * <TopAccessPointsRanking items={accessPointsResponse.items} />
 */
export function TopAccessPointsRanking({
  items,
  loading = false,
}: TopAccessPointsRankingProps): ReactNode {
  // 1. 加载态
  if (loading) {
    return (
      <Card
        bordered={false}
        style={{ backgroundColor: 'var(--semi-color-bg-2)', borderRadius: 12 }}
        bodyStyle={{ padding: 20 }}
      >
        <Skeleton active placeholder={<Skeleton.Paragraph rows={4} />} loading={true} />
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
        接入点排行
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
          暂无接入点使用记录
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {displayItems.map((item) => {
            const widthPct = max > 0 ? (item.total_tokens / max) * 100 : 0;
            const pct = totalSum > 0 ? ((item.total_tokens / totalSum) * 100).toFixed(1) : '0.0';
            const isDeleted = item.name == null;
            // 删除态降级文案：优先使用 short_code，否则取 uuid 前 8 位
            const displayName = isDeleted
              ? `已删除接入点 · ${item.short_code ?? item.access_point_id.slice(0, 8)}`
              : item.name!;

            return (
              <div
                key={item.access_point_id}
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 4,
                }}
              >
                {/* 上行：接入点名 + 数值 */}
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
                    className={isDeleted ? 'dashboard-deleted' : undefined}
                    style={{
                      color: isDeleted ? undefined : 'var(--semi-color-text-0)',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                    title={displayName}
                  >
                    {displayName}
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
                  aria-label={`${displayName} 占比 ${pct}%`}
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

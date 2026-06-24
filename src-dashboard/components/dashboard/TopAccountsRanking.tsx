import type { ReactNode } from 'react';
import { Card, Empty, Skeleton, Tag } from '@douyinfe/semi-ui';
import type { TopAccountItem } from '../../types/dashboard.ts';
import { formatTokenCompact } from '../../utils/format.ts';
import { StackedBar, type StackedBarSegment } from './StackedBar.tsx';

/**
 * TopAccountsRanking 组件 Props。
 */
interface TopAccountsRankingProps {
  /** 账号排行项（已按 total_tokens 降序，最多 10 条） */
  items: TopAccountItem[];
  /** 加载态；为 true 时显示 Skeleton 占位 */
  loading?: boolean;
}

/**
 * 上游账号词元消耗排行 Top 10 卡片。
 *
 * 每行展示：排名 + 账号显示名（"服务商 · 账号名" 或已删除降级）+ 输入/输出/缓存读取/缓存写入
 * 四段堆叠条 + 总词元紧凑数 + 禁用原因 Tag（若有）。堆叠条宽度按当前列表内最大 total_tokens
 * 归一化（传入 StackedBar 的 maxTotal），保证多行横向对比时刻度一致。
 *
 * @example
 * <TopAccountsRanking items={data.items} loading={false} />
 */
export function TopAccountsRanking({ items, loading = false }: TopAccountsRankingProps) {
  return (
    <Card
      title="账号词元消耗排行"
      bordered={false}
      style={{
        backgroundColor: 'var(--semi-color-bg-2)',
        borderRadius: 12,
      }}
    >
      {loading ? (
        <Skeleton placeholder={<Skeleton.Paragraph rows={6} />} loading={true} active />
      ) : items.length === 0 ? (
        <Empty description="该时段暂无数据" />
      ) : (
        <RankingList items={items} />
      )}
    </Card>
  );
}

/** 内部排行列表（已确认 items 非空） */
function RankingList({ items }: { items: TopAccountItem[] }) {
  // 用列表内最大 total_tokens 做横向对齐的最大刻度
  const maxTotal = Math.max(...items.map((item) => item.total_tokens), 1);
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {items.map((item, idx) => (
        <AccountRow key={item.account_id} item={item} rank={idx + 1} maxTotal={maxTotal} />
      ))}
    </div>
  );
}

/** 单行账号排行（排名 + 名字 + 禁用 Tag + 堆叠条 + 总词元） */
function AccountRow({
  item,
  rank,
  maxTotal,
}: {
  item: TopAccountItem;
  rank: number;
  maxTotal: number;
}) {
  const displayName = renderAccountName(item);
  // 4 段顺序：输入 → 输出 → 缓存读取 → 缓存写入，颜色遵循语义（主色/成功/警告/淡填充）
  const segments: StackedBarSegment[] = [
    {
      label: '输入',
      value: item.input_tokens,
      color: 'var(--semi-color-primary)',
    },
    {
      label: '输出',
      value: item.output_tokens,
      color: 'var(--semi-color-success)',
    },
    {
      label: '缓存读取',
      value: item.cache_read_tokens,
      color: 'var(--semi-color-warning)',
    },
    {
      label: '缓存写入',
      value: item.cache_creation_tokens,
      color: 'var(--semi-color-tertiary)',
    },
  ];

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
      <span
        style={{
          width: 24,
          color: 'var(--semi-color-text-2)',
          fontSize: 12,
          textAlign: 'right',
        }}
      >
        {rank}
      </span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            marginBottom: 4,
            fontSize: 13,
            gap: 8,
            alignItems: 'center',
          }}
        >
          <span
            style={{
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              minWidth: 0,
              flex: 1,
              display: 'inline-flex',
              alignItems: 'center',
              gap: 8,
            }}
          >
            <span
              style={{
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                minWidth: 0,
              }}
            >
              {displayName}
            </span>
            {item.disabled_reason && (
              <Tag color={tagColorFor(item.disabled_reason)} size="small">
                {disabledReasonLabel(item.disabled_reason)}
              </Tag>
            )}
          </span>
          <span
            style={{
              color: 'var(--semi-color-text-2)',
              fontSize: 12,
              flexShrink: 0,
            }}
          >
            {formatTokenCompact(item.total_tokens)} 词元
          </span>
        </div>
        <StackedBar segments={segments} maxTotal={maxTotal} height={8} />
      </div>
    </div>
  );
}

/**
 * 渲染账号显示名。
 *
 * - 有 account_name 且有 provider_name → "服务商 · 账号名"（服务商名次要色）
 * - 仅有 account_name → 账号名
 * - 均为 null → 已删除降级（灰色 monospace）
 */
function renderAccountName(item: TopAccountItem): ReactNode {
  if (item.account_name && item.provider_name) {
    return (
      <span>
        <span style={{ color: 'var(--semi-color-text-2)' }}>{item.provider_name} · </span>
        {item.account_name}
      </span>
    );
  }
  if (item.account_name) {
    return <span>{item.account_name}</span>;
  }
  return <span className="dashboard-deleted">已删除账号 · {item.account_id.slice(0, 8)}</span>;
}

/** 禁用原因对应的中文标签（与 AccountManager / AccessPointDrawer 保持一致） */
function disabledReasonLabel(reason: string): string {
  switch (reason) {
    case 'rate_limited':
      return '配额耗尽';
    case 'balance_exhausted':
      return '余额耗尽';
    case 'fault':
      return '故障';
    case 'manual':
      return '手动禁用';
    default:
      return reason;
  }
}

/** 禁用原因对应的 Tag 颜色（红 = 严重不可恢复，橙 = 自动可恢复，灰 = 人为操作） */
function tagColorFor(reason: string): 'red' | 'orange' | 'grey' {
  switch (reason) {
    case 'fault':
    case 'balance_exhausted':
      return 'red';
    case 'rate_limited':
      return 'orange';
    case 'manual':
    default:
      return 'grey';
  }
}

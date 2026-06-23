import type { ReactNode } from 'react';
import { Card, Empty, Skeleton } from '@douyinfe/semi-ui';
import type { TopUserItem } from '../../types/dashboard.ts';
import { formatNumber, formatTokenCompact } from '../../utils/format.ts';

/**
 * TopUsersRanking 组件 Props。
 */
interface TopUsersRankingProps {
  /** 成员排行项（已按 request_count 降序，最多 10 条） */
  items: TopUserItem[];
  /** 加载态；为 true 时显示 Skeleton 占位 */
  loading?: boolean;
}

/**
 * 成员请求量排行 Top 10 卡片。
 *
 * 每行展示：排名 + 用户名（已删除时降级展示）+ 请求数横向条形占比 + token 总量。
 * 条形宽度按当前列表内最大 request_count 归一化，方便观察相对差距。
 *
 * @example
 * <TopUsersRanking items={data.items} loading={false} />
 */
export function TopUsersRanking({ items, loading = false }: TopUsersRankingProps) {
  return (
    <Card
      title="成员请求量排行"
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
function RankingList({ items }: { items: TopUserItem[] }) {
  // 用列表内最大值做归一化，保证最大值条形占满，其余按比例缩短
  const maxCount = Math.max(...items.map((item) => item.request_count), 1);
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {items.map((item, idx) => (
        <UserRow key={item.user_id} item={item} rank={idx + 1} maxCount={maxCount} />
      ))}
    </div>
  );
}

/** 单行成员排行（排名 + 名字 + 条形 + 数值） */
function UserRow({ item, rank, maxCount }: { item: TopUserItem; rank: number; maxCount: number }) {
  const widthPct = (item.request_count / maxCount) * 100;
  const displayName = renderUserName(item);

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
          }}
        >
          <span
            style={{
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              minWidth: 0,
              flex: 1,
            }}
          >
            {displayName}
          </span>
          <span
            style={{
              color: 'var(--semi-color-text-2)',
              fontSize: 12,
              flexShrink: 0,
            }}
          >
            {formatTokenCompact(item.total_tokens)} tokens
          </span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <div
            style={{
              flex: 1,
              height: 6,
              backgroundColor: 'var(--semi-color-fill-1)',
              borderRadius: 3,
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                width: `${widthPct}%`,
                height: '100%',
                backgroundColor: 'var(--semi-color-primary)',
              }}
            />
          </div>
          <span
            style={{
              minWidth: 56,
              textAlign: 'right',
              fontSize: 12,
              color: 'var(--semi-color-text-1)',
            }}
          >
            {formatNumber(item.request_count)}
          </span>
        </div>
      </div>
    </div>
  );
}

/**
 * 渲染用户显示名。
 *
 * 优先使用 username；缺失时回退 display_name；两者均缺失时显示已删除标记（灰色 monospace）。
 */
function renderUserName(item: TopUserItem): ReactNode {
  const name = item.username ?? item.display_name;
  if (name) return name;
  return <span className="dashboard-deleted">已删除成员 · {item.user_id.slice(0, 8)}</span>;
}

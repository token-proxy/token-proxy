import { type ReactNode } from 'react';
import { Input, Tooltip, Typography } from '@douyinfe/semi-ui';
import { IconSearch } from '@douyinfe/semi-icons';

const { Text } = Typography;

/** TrajectoryToolbar 组件 Props */
interface TrajectoryToolbarProps {
  /** 时间轴是否按内容体量加权（false = 等宽序列投影） */
  scaleBySize: boolean;
  onScaleBySizeChange: (value: boolean) => void;
  /** 轮次总数（0 时「轮次」开关不可用） */
  turnCount: number;
  /** 全部轮次是否已收起 */
  allTurnsCollapsed: boolean;
  onToggleAllTurns: () => void;
  /** 工具调用总数（0 时「调用」开关不可用） */
  callCount: number;
  /** 全部工具调用是否已收起 */
  allCallsCollapsed: boolean;
  onToggleAllCalls: () => void;
  /** 账本搜索词 */
  search: string;
  onSearchChange: (value: string) => void;
  /** 命中记录数（无搜索词时为 null） */
  matchCount: number | null;
}

/**
 * TrajectoryToolbar - 轨迹工具栏
 *
 * 三个开关与 DSH 轨迹工具栏同构，开关旁带数量，无内容可折叠时置灰：
 * - **规模**：时间轴片段宽度按内容体量加权（日志未记录逐条耗时，故不提供时长口径）
 * - **轮次**：收起 / 展开所有轮次（收起后每个轮次只留一张表头，便于扫整体走向）
 * - **调用**：收起 / 展开所有工具调用行（只隐藏工具行，助手消息与思考保留）
 *
 * 右侧为账本搜索框，命中数量实时回显。
 */
export default function TrajectoryToolbar({
  scaleBySize,
  onScaleBySizeChange,
  turnCount,
  allTurnsCollapsed,
  onToggleAllTurns,
  callCount,
  allCallsCollapsed,
  onToggleAllCalls,
  search,
  onSearchChange,
  matchCount,
}: TrajectoryToolbarProps): ReactNode {
  return (
    <div className="trajectory-toolbar">
      <Tooltip content="时间轴片段宽度按记录内容体量加权（日志未记录逐条耗时，不使用时长口径）">
        <button
          type="button"
          className="trajectory-chip"
          data-checked={scaleBySize ? 'true' : 'false'}
          onClick={() => onScaleBySizeChange(!scaleBySize)}
        >
          <span className="trajectory-chip__box" />
          规模
        </button>
      </Tooltip>

      <Tooltip
        content={
          turnCount > 0
            ? allTurnsCollapsed
              ? `展开所有轮次（共 ${turnCount} 轮）`
              : `收起所有轮次（共 ${turnCount} 轮，收起后只留表头）`
            : '当前轨迹没有轮次可折叠'
        }
      >
        <button
          type="button"
          className="trajectory-chip"
          data-checked={allTurnsCollapsed ? 'true' : 'false'}
          disabled={turnCount === 0}
          onClick={onToggleAllTurns}
        >
          <span className="trajectory-chip__box" />
          轮次
          <span className="trajectory-chip__count">{turnCount}</span>
        </button>
      </Tooltip>

      <Tooltip
        content={
          callCount > 0
            ? allCallsCollapsed
              ? `展开所有工具调用（共 ${callCount} 次）`
              : `收起所有工具调用行（共 ${callCount} 次，保留助手消息与思考）`
            : '当前轨迹没有工具调用可折叠'
        }
      >
        <button
          type="button"
          className="trajectory-chip"
          data-checked={allCallsCollapsed ? 'true' : 'false'}
          disabled={callCount === 0}
          onClick={onToggleAllCalls}
        >
          <span className="trajectory-chip__box" />
          调用
          <span className="trajectory-chip__count">{callCount}</span>
        </button>
      </Tooltip>

      <div className="trajectory-toolbar__spacer" />

      {matchCount !== null && (
        <Text size="small" type="tertiary">
          {matchCount} 条命中
        </Text>
      )}
      <Input
        size="small"
        prefix={<IconSearch />}
        placeholder="搜索轨迹"
        value={search}
        onChange={onSearchChange}
        showClear
        style={{ width: 220 }}
      />
    </div>
  );
}

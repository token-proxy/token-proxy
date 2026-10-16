import { type ReactNode, type RefObject } from 'react';
import { Tag, Tooltip, Typography } from '@douyinfe/semi-ui';
import {
  IconChevronDown,
  IconChevronRight,
  IconCode,
  IconComment,
  IconSend,
  IconServer,
  IconSetting,
  IconUser,
  IconWrench,
} from '@douyinfe/semi-icons';
import type {
  TrajectoryCell,
  TrajectoryCellKind,
  TrajectoryGroup,
  TrajectoryModel,
} from '../../../utils/trajectory.ts';

const { Text } = Typography;

/** TrajectoryLedger 组件 Props */
interface TrajectoryLedgerProps {
  /** 轨迹模型 */
  model: TrajectoryModel;
  /** 当前选中记录序号 */
  selectedIndex: number | null;
  onSelect: (index: number) => void;
  /** 搜索命中的记录集合（null = 无搜索） */
  matches: ReadonlySet<number> | null;
  /** 已收起的轮次 */
  collapsedTurns: ReadonlySet<number>;
  onToggleTurn: (index: number) => void;
  /** 已整个收起的分组（点分组表头触发） */
  collapsedGroups: ReadonlySet<string>;
  onToggleGroup: (key: string) => void;
  /** 已收起工具调用行的分组（「调用」开关触发，只隐藏工具行） */
  collapsedCalls: ReadonlySet<string>;
  onToggleCalls: (key: string) => void;
  /** 账本滚动容器 */
  containerRef: RefObject<HTMLDivElement | null>;
}

/** 记录类型 → 图标 */
const KIND_ICONS: Record<TrajectoryCellKind, ReactNode> = {
  request: <IconSend />,
  response: <IconServer />,
  system: <IconSetting />,
  user: <IconUser />,
  assistant: <IconComment />,
  tool: <IconWrench />,
  raw: <IconCode />,
};

/** 工具行使用等宽字体展示（DSH 中 `bash {"command":…}` 为等宽） */
const MONO_KINDS: ReadonlySet<TrajectoryCellKind> = new Set(['tool', 'request', 'response', 'raw']);

/**
 * TrajectoryLedger - 事件账本
 *
 * 按「轮次 → 分组（消息 / 第 k 步）→ 记录行」渲染：
 * 轮次表头 sticky，分组表头可折叠，记录行展示时间 · 类型 · 内容 · 模型 · 工具 ·
 * 输入 / 输出 / 思考等列，点击行选中并联动右侧检查器与时间轴游标。
 * 搜索时忽略折叠状态，直接展示命中行。
 */
export default function TrajectoryLedger({
  model,
  selectedIndex,
  onSelect,
  matches,
  collapsedTurns,
  onToggleTurn,
  collapsedGroups,
  onToggleGroup,
  collapsedCalls,
  onToggleCalls,
  containerRef,
}: TrajectoryLedgerProps): ReactNode {
  const searching = matches !== null;

  /** 某分组的工具行是否被「调用」开关收起（搜索时始终展开，保证命中可见） */
  const callsCollapsed = (key: string): boolean => !searching && collapsedCalls.has(key);

  const visibleTurns = model.turns
    .map((turn) => ({
      turn,
      groups: turn.groups
        .map((group) => {
          const kept = group.cells.filter(
            (cell) =>
              (!searching || matches.has(cell.index)) &&
              !(callsCollapsed(group.key) && cell.kind === 'tool'),
          );
          return { group, cells: kept, hiddenCalls: group.cells.length - kept.length };
        })
        .filter((entry) => entry.cells.length > 0),
    }))
    .filter((entry) => entry.groups.length > 0);

  if (visibleTurns.length === 0) {
    return (
      <div className="trajectory-ledger" ref={containerRef}>
        <div className="trajectory-ledger__empty">
          <Text type="tertiary">{searching ? '没有匹配的记录' : '暂无轨迹内容'}</Text>
        </div>
      </div>
    );
  }

  return (
    <div className="trajectory-ledger" ref={containerRef}>
      {visibleTurns.map(({ turn, groups }) => {
        const collapsed = !searching && collapsedTurns.has(turn.index);
        const cellCount = turn.groups.reduce((sum, group) => sum + group.cells.length, 0);

        return (
          <section className="trajectory-turn" key={turn.index}>
            <button
              type="button"
              className="trajectory-turn__header"
              onClick={() => onToggleTurn(turn.index)}
              aria-expanded={!collapsed}
            >
              <span className="trajectory-turn__chevron" data-open={collapsed ? 'false' : 'true'}>
                {collapsed ? <IconChevronRight /> : <IconChevronDown />}
              </span>
              <span className="trajectory-turn__title">{turn.title}</span>
              {turn.description && (
                <span className="trajectory-turn__description">{turn.description}</span>
              )}
              <span className="trajectory-turn__spacer" />
              <Text size="small" type="tertiary">
                {cellCount} 条记录
              </Text>
            </button>

            {!collapsed &&
              groups.map(({ group, cells, hiddenCalls }) => (
                <GroupBlock
                  key={group.key}
                  group={group}
                  cells={cells}
                  hiddenCalls={hiddenCalls}
                  selectedIndex={selectedIndex}
                  onSelect={onSelect}
                  collapsed={!searching && collapsedGroups.has(group.key)}
                  onToggle={() => onToggleGroup(group.key)}
                  callsCollapsed={callsCollapsed(group.key)}
                  onToggleCalls={() => onToggleCalls(group.key)}
                />
              ))}
          </section>
        );
      })}
    </div>
  );
}

/** 分组（消息 / 第 k 步）及其记录行 */
function GroupBlock({
  group,
  cells,
  hiddenCalls,
  selectedIndex,
  onSelect,
  collapsed,
  onToggle,
  callsCollapsed,
  onToggleCalls,
}: {
  group: TrajectoryGroup;
  cells: TrajectoryCell[];
  /** 被「调用」开关收起的工具行数量 */
  hiddenCalls: number;
  selectedIndex: number | null;
  onSelect: (index: number) => void;
  collapsed: boolean;
  onToggle: () => void;
  callsCollapsed: boolean;
  onToggleCalls: () => void;
}): ReactNode {
  const toolCount = group.cells.filter((cell) => cell.kind === 'tool').length;

  return (
    <div className="trajectory-group">
      <div className="trajectory-group__row">
        <button
          type="button"
          className="trajectory-group__header"
          onClick={onToggle}
          aria-expanded={!collapsed}
        >
          <span className="trajectory-group__chevron" data-open={collapsed ? 'false' : 'true'}>
            {collapsed ? <IconChevronRight /> : <IconChevronDown />}
          </span>
          <span className="trajectory-group__title">{group.title}</span>
          {group.description && (
            <span className="trajectory-group__description">{group.description}</span>
          )}
          <span className="trajectory-group__spacer" />
          <Text size="small" type="tertiary">
            {cells.length + hiddenCalls}
          </Text>
        </button>

        {/* 「调用」收起时给出可见的收起摘要，避免误以为记录丢失 */}
        {toolCount > 0 && (
          <button
            type="button"
            className="trajectory-group__calls"
            data-collapsed={callsCollapsed ? 'true' : 'false'}
            onClick={onToggleCalls}
            title={callsCollapsed ? '展开本步的工具调用' : '收起本步的工具调用'}
          >
            {callsCollapsed ? `已收起 ${toolCount} 次工具调用` : `收起 ${toolCount} 次工具调用`}
          </button>
        )}
      </div>

      {!collapsed &&
        cells.map((cell) => (
          <Row
            key={cell.index}
            cell={cell}
            selected={cell.index === selectedIndex}
            onSelect={onSelect}
          />
        ))}
    </div>
  );
}

/** 单条记录行 */
function Row({
  cell,
  selected,
  onSelect,
}: {
  cell: TrajectoryCell;
  selected: boolean;
  onSelect: (index: number) => void;
}): ReactNode {
  const mono = MONO_KINDS.has(cell.kind);

  return (
    <div
      className="trajectory-row"
      data-kind={cell.kind}
      data-selected={selected ? 'true' : 'false'}
      data-cell-index={cell.index}
      role="button"
      tabIndex={0}
      onClick={() => onSelect(cell.index)}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelect(cell.index);
        }
      }}
    >
      <span className="trajectory-row__time">{cell.time ?? ''}</span>

      <span className="trajectory-row__icon" data-kind={cell.kind}>
        {KIND_ICONS[cell.kind]}
      </span>

      <span className="trajectory-row__kind">{cell.kindLabel}</span>

      <span className="trajectory-row__main">
        <span className="trajectory-row__text" data-mono={mono ? 'true' : 'false'}>
          {cell.text || '（无内容）'}
        </span>
        {cell.preview && <span className="trajectory-row__preview">{cell.preview}</span>}
      </span>

      <span className="trajectory-row__meta">
        {cell.model && <span className="trajectory-row__model">{cell.model}</span>}
        <TokenColumns cell={cell} />
        {cell.timeMs !== null && <span>{cell.timeMs} ms</span>}
        {cell.contentMissing && (
          <Tag size="small" color="grey">
            无原文
          </Tag>
        )}
        <StatusTag cell={cell} />
      </span>
    </div>
  );
}

/** 输入 / 输出 / 思考三列（仅承载词元的行有值） */
function TokenColumns({ cell }: { cell: TrajectoryCell }): ReactNode {
  const input = (cell.input ?? 0) + (cell.cacheRead ?? 0) + (cell.cacheWrite ?? 0);
  const output = (cell.output ?? 0) + (cell.think ?? 0);
  if (input === 0 && output === 0) return null;

  const parts: string[] = [];
  if (input > 0) parts.push(`输入 ${input.toLocaleString('zh-CN')}`);
  if (output > 0) parts.push(`输出 ${output.toLocaleString('zh-CN')}`);
  if ((cell.think ?? 0) > 0) parts.push(`思考 ${(cell.think ?? 0).toLocaleString('zh-CN')}`);

  return (
    <Tooltip
      content={`未命中缓存 ${cell.input ?? 0} · 缓存写入 ${cell.cacheWrite ?? 0} · 缓存读取 ${cell.cacheRead ?? 0} · 输出 ${cell.output ?? 0} · 思考 ${cell.think ?? 0}`}
    >
      <span className="trajectory-row__tokens">{parts.join(' · ')}</span>
    </Tooltip>
  );
}

/** 状态标签（失败 / 等待中，已完成不占位） */
function StatusTag({ cell }: { cell: TrajectoryCell }): ReactNode {
  if (cell.status === 'failed') {
    return (
      <Tag size="small" color="red">
        失败
      </Tag>
    );
  }
  if (cell.status === 'pending') {
    return (
      <Tag size="small" color="amber">
        等待中
      </Tag>
    );
  }
  return null;
}

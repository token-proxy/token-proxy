import { type ReactNode, useCallback, useMemo, useRef, useState } from 'react';
import { Tag, Typography } from '@douyinfe/semi-ui';
import { IconAlertTriangle } from '@douyinfe/semi-icons';
import type { LogDetailFull } from '../../../types/log.ts';
import { PROTOCOL_LABELS, buildTrajectory } from '../../../utils/trajectory.ts';
import type {
  TrajectoryCell,
  TrajectoryModel,
  TrajectoryTimelineScale,
} from '../../../utils/trajectory.ts';
import TrajectoryToolbar from './TrajectoryToolbar.tsx';
import TrajectoryTimelineView from './TrajectoryTimeline.tsx';
import TrajectoryLedger from './TrajectoryLedger.tsx';
import TrajectoryInspector from './TrajectoryInspector.tsx';
import type { TrajectoryLocation } from './TrajectoryInspector.tsx';
import './TrajectoryView.css';

const { Text } = Typography;

/** TrajectoryView 组件 Props */
interface TrajectoryViewProps {
  /** 日志完整详情 */
  data: LogDetailFull;
  style?: React.CSSProperties;
}

/**
 * TrajectoryView - 运行轨迹视图
 *
 * 参照 DSH 轨迹视图的构成：工具栏（规模 / 轮次 / 调用开关 + 搜索）+ 三泳道时间轴概览
 * + 按轮次分组的事件账本 + 右侧记录检查器。
 *
 * 账本内容取决于请求实际携带的内容：Anthropic 与 OpenAI Chat Completions 的请求体
 * 自带历史消息，轨迹覆盖整段对话；OpenAI Responses 只有本轮输入项时轨迹只有本轮。
 * 请求头、响应头与请求选项内嵌在「请求」「响应」记录行的检查器页签中。
 *
 * 由父组件以 `key={日志 ID}` 挂载，切换日志时整体重建选中与折叠状态。
 */
export default function TrajectoryView({ data, style }: TrajectoryViewProps): ReactNode {
  const model = useMemo(() => buildTrajectory(data), [data]);
  const protocol = model.protocol;

  const [selectedIndex, setSelectedIndex] = useState<number | null>(() => defaultSelection(model));
  const [search, setSearch] = useState('');
  const [scale, setScale] = useState<TrajectoryTimelineScale>('sequence');
  const [collapsedTurns, setCollapsedTurns] = useState<ReadonlySet<number>>(() => new Set());
  const [collapsedGroups, setCollapsedGroups] = useState<ReadonlySet<string>>(() => new Set());
  const [collapsedCalls, setCollapsedCalls] = useState<ReadonlySet<string>>(() => new Set());
  const [allTurnsCollapsed, setAllTurnsCollapsed] = useState(false);
  const [allCallsCollapsed, setAllCallsCollapsed] = useState(false);
  const ledgerRef = useRef<HTMLDivElement | null>(null);

  // ─── 搜索命中 ───

  const matches = useMemo(() => {
    const keyword = search.trim().toLowerCase();
    if (!keyword) return null;
    const hit = new Set<number>();
    for (const cell of model.cells) {
      if (matchesCell(cell, keyword)) hit.add(cell.index);
    }
    return hit;
  }, [model, search]);

  // ─── 选中与滚动联动 ───

  const scrollToCell = useCallback((index: number) => {
    const container = ledgerRef.current;
    const target = container?.querySelector(`[data-cell-index="${index}"]`);
    target?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }, []);

  /** 行点击：再次点击同一条记录收起检查器 */
  const handleRowSelect = useCallback((index: number) => {
    setSelectedIndex((previous) => (previous === index ? null : index));
  }, []);

  /** 时间轴片段点击：强制选中并滚动到对应记录 */
  const handleTimelineSelect = useCallback(
    (index: number) => {
      setSelectedIndex(index);
      scrollToCell(index);
    },
    [scrollToCell],
  );

  // ─── 折叠 ───

  /** 单个轮次：收起后只留表头 */
  const toggleTurn = useCallback((index: number) => {
    setCollapsedTurns((previous) => toggleSet(previous, index));
  }, []);

  /** 单个分组（第 k 步）：收起整个分组的记录行 */
  const toggleGroup = useCallback((key: string) => {
    setCollapsedGroups((previous) => toggleSet(previous, key));
  }, []);

  /** 单个分组的工具调用行：只收起工具行，保留助手消息 */
  const toggleCalls = useCallback((key: string) => {
    setCollapsedCalls((previous) => toggleSet(previous, key));
  }, []);

  /** 含工具调用的分组（「调用」开关的作用范围） */
  const stepGroupKeys = useMemo(
    () =>
      model.turns.flatMap((turn) =>
        turn.groups
          .filter((group) => group.cells.some((cell) => cell.kind === 'tool'))
          .map((group) => group.key),
      ),
    [model],
  );

  const handleToggleAllTurns = useCallback(() => {
    setCollapsedTurns(allTurnsCollapsed ? new Set() : new Set(model.turns.map((t) => t.index)));
    setAllTurnsCollapsed(!allTurnsCollapsed);
  }, [allTurnsCollapsed, model]);

  const handleToggleAllCalls = useCallback(() => {
    setCollapsedCalls(allCallsCollapsed ? new Set() : new Set(stepGroupKeys));
    setAllCallsCollapsed(!allCallsCollapsed);
  }, [allCallsCollapsed, stepGroupKeys]);

  // ─── 选中记录的检查器数据 ───

  const selected = useMemo(
    () => model.cells.find((cell) => cell.index === selectedIndex) ?? null,
    [model, selectedIndex],
  );
  const location = useMemo(
    () => (selected ? locate(model, selected.index) : null),
    [model, selected],
  );

  return (
    <div className="trajectory-view" style={style}>
      {/* ─── 视图标题 ─── */}
      <div className="trajectory-view__head">
        <span className="trajectory-view__title">运行轨迹</span>
        <Tag size="small" color="blue">
          {PROTOCOL_LABELS[protocol] ?? protocol}
        </Tag>
        <Text size="small" type="tertiary">
          {model.stats.turns} 轮 · {model.stats.steps} 步 · {model.stats.cells} 条记录 ·{' '}
          {model.stats.toolCalls} 次工具调用
        </Text>
        <span className="trajectory-view__spacer" />
        <Text size="small" type="tertiary">
          {protocol === 'openai_response'
            ? '按请求携带的输入项渲染（可能只含本轮）'
            : '含请求携带的历史对话与本次响应'}
        </Text>
      </div>

      {/* ─── 内容可用性提示（原文未落库时明确区分「数据缺失」与「解析失败」）─── */}
      {model.warnings.length > 0 && (
        <div className="trajectory-warning">
          <IconAlertTriangle size="small" />
          <span>{model.warnings.join('；')}</span>
        </div>
      )}

      {/* ─── 工具栏 ─── */}
      <TrajectoryToolbar
        scaleBySize={scale === 'size'}
        onScaleBySizeChange={(value) => setScale(value ? 'size' : 'sequence')}
        turnCount={model.stats.turns}
        allTurnsCollapsed={allTurnsCollapsed}
        onToggleAllTurns={handleToggleAllTurns}
        callCount={model.stats.toolCalls}
        allCallsCollapsed={allCallsCollapsed}
        onToggleAllCalls={handleToggleAllCalls}
        search={search}
        onSearchChange={setSearch}
        matchCount={matches ? matches.size : null}
      />

      {/* ─── 时间轴概览 ─── */}
      <TrajectoryTimelineView
        timeline={model.timeline}
        scale={scale}
        selectedIndex={selectedIndex}
        onSelect={handleTimelineSelect}
        matches={matches}
      />

      {/* ─── 账本 + 检查器 ─── */}
      <div className="trajectory-view__body">
        <TrajectoryLedger
          model={model}
          selectedIndex={selectedIndex}
          onSelect={handleRowSelect}
          matches={matches}
          collapsedTurns={collapsedTurns}
          onToggleTurn={toggleTurn}
          collapsedGroups={collapsedGroups}
          onToggleGroup={toggleGroup}
          collapsedCalls={collapsedCalls}
          onToggleCalls={toggleCalls}
          containerRef={ledgerRef}
        />

        {selected && location ? (
          <TrajectoryInspector
            cell={selected}
            location={location}
            onClose={() => setSelectedIndex(null)}
          />
        ) : (
          <aside className="trajectory-inspector trajectory-inspector--empty">
            <Text type="tertiary">选择左侧记录查看详情</Text>
          </aside>
        )}
      </div>
    </div>
  );
}

/** 默认选中：本次请求的响应记录（无则取最后一条记录） */
function defaultSelection(model: TrajectoryModel): number | null {
  const cells = model.cells;
  for (let i = cells.length - 1; i >= 0; i--) {
    const cell = cells[i];
    if (cell.kind === 'assistant' && (cell.output ?? 0) + (cell.think ?? 0) > 0) return cell.index;
  }
  for (let i = cells.length - 1; i >= 0; i--) {
    if (cells[i].kind === 'assistant' || cells[i].kind === 'response') return cells[i].index;
  }
  return cells.length > 0 ? cells[cells.length - 1].index : null;
}

/** 记录是否命中搜索词（正文、预览与检查器内容一起匹配） */
function matchesCell(cell: TrajectoryCell, keyword: string): boolean {
  const haystack = [
    cell.text,
    cell.preview ?? '',
    cell.kindLabel,
    cell.toolName ?? '',
    cell.model ?? '',
    ...cell.tabs.map((tab) => tab.value ?? ''),
    ...cell.tabs.flatMap((tab) => (tab.items ?? []).map((item) => `${item.label} ${item.value}`)),
  ]
    .join('\n')
    .toLowerCase();
  return haystack.includes(keyword);
}

/** 定位记录所在的轮次与分组 */
function locate(model: TrajectoryModel, index: number): TrajectoryLocation {
  for (const turn of model.turns) {
    for (const group of turn.groups) {
      if (group.cells.some((cell) => cell.index === index)) {
        return { turnTitle: turn.title, groupTitle: group.title };
      }
    }
  }
  return { turnTitle: '—', groupTitle: '—' };
}

/** 集合内元素的增删（返回新集合） */
function toggleSet<T>(set: ReadonlySet<T>, value: T): ReadonlySet<T> {
  const next = new Set(set);
  if (next.has(value)) next.delete(value);
  else next.add(value);
  return next;
}

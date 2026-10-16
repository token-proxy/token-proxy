import { type ReactNode, useCallback, useEffect, useRef, useState } from 'react';
import { Tooltip } from '@douyinfe/semi-ui';
import { LANE_LABELS, trajectorySpanBox } from '../../../utils/trajectory.ts';
import type { TrajectoryLane, TrajectoryTimeline } from '../../../utils/trajectory.ts';

/** 泳道显示顺序 */
const LANES: TrajectoryLane[] = ['input', 'model', 'tool'];

/** 缩放范围与步长（相对基准宽度的倍数） */
const MIN_ZOOM = 1;
const MAX_ZOOM = 12;
const ZOOM_STEP = 1.25;

/** TrajectoryTimelineView 组件 Props */
interface TrajectoryTimelineViewProps {
  /** 时间轴投影模型（等宽槽位） */
  timeline: TrajectoryTimeline;
  /** 当前选中的记录序号 */
  selectedIndex: number | null;
  /** 选中记录 */
  onSelect: (index: number) => void;
  /** 搜索命中的记录集合（null = 无搜索） */
  matches: ReadonlySet<number> | null;
  /**
   * 需要保持可见的记录序号（滚动联动）
   *
   * 账本滚动时由父组件传入「当前首条可见记录」，时间轴据此横向滚动，
   * 使片段与下方列表保持对应。
   */
  focusIndex: number | null;
}

/**
 * TrajectoryTimelineView - 三泳道时间轴概览
 *
 * 投影口径对齐 DSH 的 `sequence` 模式：**每条记录恰好占一个等宽槽位**，
 * 泳道只决定纵向行（输入 / 模型 / 工具），横向位置由槽位序号决定 ——
 * 因此不同泳道的片段在结构上不可能重叠（日志没有逐条耗时，不做时间轴布点，
 * 也不按内容体量加权，避免一条极宽一条极窄）。
 *
 * 交互：滚轮缩放（1×~12×）、横向滚动、点击片段跳转选中；
 * 账本滚动时按 `focusIndex` 自动横向跟随。
 */
export default function TrajectoryTimelineView({
  timeline,
  selectedIndex,
  onSelect,
  matches,
  focusIndex,
}: TrajectoryTimelineViewProps): ReactNode {
  const [zoom, setZoom] = useState(MIN_ZOOM);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const selected = timeline.spans.find((span) => span.cellIndex === selectedIndex);
  const total = Math.max(1, timeline.total);

  /** 把某个槽位滚动到可视区（保留左侧一点余量，便于看清前一条） */
  const scrollToSlot = useCallback(
    (slot: number) => {
      const container = scrollRef.current;
      if (!container) return;
      const contentWidth = container.scrollWidth;
      const center = ((slot + 0.5) / total) * contentWidth;
      const left = Math.max(0, center - container.clientWidth / 2);
      container.scrollTo({ left, behavior: 'smooth' });
    },
    [total],
  );

  // 账本滚动联动：跟随当前首条可见记录
  useEffect(() => {
    const span = timeline.spans.find((item) => item.cellIndex === focusIndex);
    if (span) scrollToSlot(span.slot);
  }, [focusIndex, timeline, scrollToSlot]);

  // 选中记录时同样保证可见
  useEffect(() => {
    if (selected) scrollToSlot(selected.slot);
  }, [selected, scrollToSlot]);

  return (
    <div className="trajectory-timeline">
      <div className="trajectory-timeline__head">
        <span className="trajectory-timeline__hint">
          每次记录一个等宽槽位（无逐条耗时数据，故不按时间布轴）
        </span>
        <span className="trajectory-timeline__zoom">
          <button
            type="button"
            className="trajectory-timeline__zoom-btn"
            onClick={() => setZoom((z) => Math.max(MIN_ZOOM, z / ZOOM_STEP))}
            disabled={zoom <= MIN_ZOOM}
            title="缩小（滚轮同样可缩放）"
          >
            −
          </button>
          <span className="trajectory-timeline__zoom-value">{zoom.toFixed(1)}×</span>
          <button
            type="button"
            className="trajectory-timeline__zoom-btn"
            onClick={() => setZoom((z) => Math.min(MAX_ZOOM, z * ZOOM_STEP))}
            disabled={zoom >= MAX_ZOOM}
            title="放大（滚轮同样可缩放）"
          >
            +
          </button>
        </span>
      </div>

      {/* 滚轮缩放：指针在时间轴上时直接缩放（页面本身不滚动，不抢占滚动语义） */}
      <div
        className="trajectory-timeline__scroll"
        ref={scrollRef}
        onWheel={(event) => {
          event.preventDefault();
          setZoom((z) =>
            Math.min(
              MAX_ZOOM,
              Math.max(MIN_ZOOM, event.deltaY < 0 ? z * ZOOM_STEP : z / ZOOM_STEP),
            ),
          );
        }}
      >
        <div
          className="trajectory-timeline__track"
          style={{ width: `${zoom * 100}%`, minWidth: '100%' }}
        >
          {LANES.map((lane) => (
            <div className="trajectory-lane" key={lane}>
              <span className="trajectory-lane__label">{LANE_LABELS[lane]}</span>
              <div className="trajectory-lane__track">
                {timeline.spans
                  .filter((span) => span.lane === lane)
                  .map((span) => {
                    const box = trajectorySpanBox(span, timeline);
                    const isMatch = matches?.has(span.cellIndex) ?? false;
                    return (
                      <Tooltip key={span.cellIndex} content={`${span.label}（点击跳转）`}>
                        <button
                          type="button"
                          className="trajectory-span"
                          data-kind={span.kind}
                          data-selected={span.cellIndex === selectedIndex ? 'true' : 'false'}
                          data-match={isMatch ? 'true' : 'false'}
                          style={{
                            left: `${box.left * 100}%`,
                            width: `${box.width * 100}%`,
                          }}
                          onClick={() => onSelect(span.cellIndex)}
                        />
                      </Tooltip>
                    );
                  })}
                {timeline.boundaries.map((boundary) => (
                  <span
                    key={`boundary-${boundary.turnIndex}`}
                    className="trajectory-boundary"
                    style={{ left: `${(boundary.slot / total) * 100}%` }}
                    title={`第 ${boundary.turnIndex} 轮开始`}
                  />
                ))}
              </div>
            </div>
          ))}
          {selected && (
            <span
              className="trajectory-cursor"
              style={{ left: `${((selected.slot + 0.5) / total) * 100}%` }}
            />
          )}
        </div>
      </div>
    </div>
  );
}

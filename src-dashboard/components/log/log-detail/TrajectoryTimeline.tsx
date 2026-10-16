import { type ReactNode } from 'react';
import { Tooltip } from '@douyinfe/semi-ui';
import {
  LANE_LABELS,
  trajectorySpanStart,
  trajectorySpanWidth,
} from '../../../utils/trajectory.ts';
import type {
  TrajectoryLane,
  TrajectoryTimeline,
  TrajectoryTimelineScale,
} from '../../../utils/trajectory.ts';

/** 泳道显示顺序 */
const LANES: TrajectoryLane[] = ['input', 'model', 'tool'];

/** TrajectoryTimelineView 组件 Props */
interface TrajectoryTimelineViewProps {
  /** 时间轴投影模型 */
  timeline: TrajectoryTimeline;
  /** 宽度口径：序列（等宽）/ 规模（按体量加权） */
  scale: TrajectoryTimelineScale;
  /** 当前选中的记录序号 */
  selectedIndex: number | null;
  /** 选中记录（用于时间轴游标定位） */
  onSelect: (index: number) => void;
  /** 搜索命中的记录集合（null = 无搜索） */
  matches: ReadonlySet<number> | null;
}

/**
 * TrajectoryTimelineView - 三泳道时间轴概览
 *
 * 与 DSH 轨迹的 Overview 同构：按「输入 / 模型 / 工具」三条泳道把每条记录投影到
 * 以记录顺序为轴的横条上，轮次边界画分隔线，选中记录处画游标；
 * 点击片段可跳转选中对应记录。
 */
export default function TrajectoryTimelineView({
  timeline,
  scale,
  selectedIndex,
  onSelect,
  matches,
}: TrajectoryTimelineViewProps): ReactNode {
  const selected = timeline.spans.find((span) => span.cellIndex === selectedIndex);

  return (
    <div className="trajectory-timeline">
      {LANES.map((lane) => (
        <div className="trajectory-lane" key={lane}>
          <span className="trajectory-lane__label">{LANE_LABELS[lane]}</span>
          <div className="trajectory-lane__track">
            {/* 轮次边界 */}
            {timeline.boundaries.map((boundary) => (
              <span
                key={`boundary-${boundary.turnIndex}`}
                className="trajectory-lane__boundary"
                style={{ left: `${boundary.position * 100}%` }}
              />
            ))}

            {/* 记录片段 */}
            {timeline.spans
              .filter((span) => span.lane === lane)
              .map((span) => {
                const dimmed = matches !== null && !matches.has(span.cellIndex);
                return (
                  <Tooltip key={span.cellIndex} content={span.label}>
                    <button
                      type="button"
                      className="trajectory-lane__span"
                      data-kind={span.kind}
                      data-selected={span.cellIndex === selectedIndex ? 'true' : 'false'}
                      data-dimmed={dimmed ? 'true' : 'false'}
                      style={{
                        left: `${trajectorySpanStart(span, timeline, scale) * 100}%`,
                        width: `${Math.max(trajectorySpanWidth(span, timeline, scale) * 100, 0.6)}%`,
                      }}
                      onClick={() => onSelect(span.cellIndex)}
                    />
                  </Tooltip>
                );
              })}

            {/* 选中游标（与账本选中行对齐） */}
            {selected && (
              <span
                className="trajectory-lane__cursor"
                style={{ left: `${selected.center * 100}%` }}
              />
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

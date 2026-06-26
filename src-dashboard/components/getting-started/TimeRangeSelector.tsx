/**
 * Dashboard 时间范围切换器组件。
 *
 * 整合预设范围（今日 / 7 天 / 30 天）、自定义日期区间和刷新按钮，
 * 挂载在「数据指标」和「用量趋势」卡片 `headerExtraContent` 内。
 *
 * 设计要点：
 * - 组件对外只输出 `TimeRangeValue { start: Date, end: Date }`，不再区分预设/自定义
 * - 按钮高亮由 `detectPreset` 从日期边界反向推导，是纯视图层行为
 * - 预设切换到 today/last7/last30 时自动计算日期范围并立即通知父组件
 * - "自定义"激活时弹出 Popover 内嵌 DatePicker，仅用户选择日期后才通知父组件，
 *   关闭 popover 未选择则回退到原高亮状态，无副作用
 * - 右侧刷新按钮（IconRefresh + loading spinner）
 * - 整体 flexWrap: wrap，窄屏自动换行
 */

import { useState } from 'react';
import { Button, ButtonGroup, DatePicker, Popover } from '@douyinfe/semi-ui';
import { IconCalendar, IconRefresh } from '@douyinfe/semi-icons';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import type { TimeRangePreset, TimeRangeValue } from '../../types/dashboard';
import { last30Range, last7Range, todayRange } from '../../types/dashboard';

/**
 * TimeRangeSelector 组件属性。
 */
export interface TimeRangeSelectorProps {
  /** 当前选择的时间范围 */
  value: TimeRangeValue;
  /** 时间范围切换回调（用户切换预设或选择自定义区间时触发） */
  onChange: (next: TimeRangeValue) => void;
  /** 刷新按钮点击回调 */
  onRefresh: () => void;
  /** 数据加载中（控制刷新按钮 spinner），默认 false */
  loading?: boolean;
  /** 允许展示的预设范围，默认展示全部预设 */
  allowedPresets?: TimeRangePreset[];
}

/** 预设范围的中文标签，按显示顺序排列 */
const PRESET_LABELS: Record<TimeRangePreset, string> = {
  today: '今日',
  last7: '7 天',
  last30: '30 天',
  custom: '自定义',
};

// ─── 预设反推 ────────────────────────────────────────────

/** 容差（毫秒），容忍 now 的自然漂移（约 60 秒） */
const TOLERANCE_MS = 60_000;

/** 判断两个 Date 是否在容差范围内相等 */
function isWithinTolerance(a: Date, b: Date): boolean {
  return Math.abs(a.getTime() - b.getTime()) <= TOLERANCE_MS;
}

/**
 * 从日期边界反向推导预设。
 *
 * 推导优先级：today > last7 > last30 > custom
 * - today：start 是今天 00:00、end 接近 now
 * - last7：start 是 7 天前、end 接近 now
 * - last30：start 是 30 天前、end 接近 now
 * - custom：其余
 */
function detectPreset(range: TimeRangeValue): TimeRangePreset {
  const now = new Date();

  // today：start 是今天 00:00
  const todayStart = new Date(now);
  todayStart.setHours(0, 0, 0, 0);
  if (isWithinTolerance(range.start, todayStart) && isWithinTolerance(range.end, now)) {
    return 'today';
  }

  // last7：start 是 7 天前
  const last7Start = new Date(now);
  last7Start.setDate(last7Start.getDate() - 7);
  if (isWithinTolerance(range.start, last7Start) && isWithinTolerance(range.end, now)) {
    return 'last7';
  }

  // last30：start 是 30 天前
  const last30Start = new Date(now);
  last30Start.setDate(last30Start.getDate() - 30);
  if (isWithinTolerance(range.start, last30Start) && isWithinTolerance(range.end, now)) {
    return 'last30';
  }

  return 'custom';
}

/**
 * 时间范围切换器 + 刷新按钮。
 *
 * 这是一个纯粹的时间范围输出组件：对外只输出 `{ start: Date, end: Date }`。
 * 预设按钮高亮由 `detectPreset` 从日期边界反向推导，是纯视图层逻辑。
 *
 * 交互逻辑：
 * - 切换到 today / last7 / last30 时，自动计算对应日期范围并通过 onChange 通知父组件
 * - 切换到 custom 时，打开 DatePicker 供用户调整，**不**触发 onChange
 * - 用户选完日期后，通过 onChange 上报 `{ start, end }`
 * - 关闭 Popover 而未选择日期时，无副作用
 *
 * @example
 * ```tsx
 * const [range, setRange] = useState<TimeRangeValue>(last7Range);
 * <TimeRangeSelector
 *   value={range}
 *   onChange={setRange}
 *   onRefresh={() => refetch()}
 *   loading={loading}
 * />
 * ```
 */
export function TimeRangeSelector({
  value,
  onChange,
  onRefresh,
  loading = false,
  allowedPresets = ['today', 'last7', 'last30', 'custom'],
}: TimeRangeSelectorProps) {
  const [customPopoverVisible, setCustomPopoverVisible] = useState(false);

  // DatePicker 的受控值：直接使用 value.start/end 作为初始范围
  const customDates: [Date, Date] = [new Date(value.start), new Date(value.end)];

  /** 处理预设切换 */
  const handlePresetChange = (next: TimeRangePreset) => {
    if (next === 'today') {
      onChange(todayRange());
      setCustomPopoverVisible(false);
    } else if (next === 'last7') {
      onChange(last7Range());
      setCustomPopoverVisible(false);
    } else if (next === 'last30') {
      onChange(last30Range());
      setCustomPopoverVisible(false);
    } else {
      // custom：打开 DatePicker，不调 onChange
      setCustomPopoverVisible(true);
    }
  };

  /** DatePicker 选择回调：将 [Date, Date] 上报给父组件 */
  const handleCustomDateChange: DatePickerProps['onChange'] = (dates) => {
    if (Array.isArray(dates) && dates.length === 2 && dates[0] && dates[1]) {
      onChange({ start: new Date(dates[0]), end: new Date(dates[1]) });
      setCustomPopoverVisible(false);
    }
  };

  // 推导当前高亮按钮
  const activePreset = customPopoverVisible ? 'custom' : detectPreset(value);

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        flexWrap: 'wrap',
      }}
    >
      <ButtonGroup size="small" aria-label="时间范围">
        {allowedPresets.map((preset) => (
          <Button
            key={preset}
            theme={activePreset === preset ? 'solid' : 'light'}
            type="primary"
            onClick={() => handlePresetChange(preset)}
          >
            {PRESET_LABELS[preset]}
          </Button>
        ))}
      </ButtonGroup>

      {/* 自定义日期选择器：始终存在 DOM，仅通过 visible 控制显隐 */}
      {allowedPresets.includes('custom') && (
        <Popover
          visible={customPopoverVisible}
          onVisibleChange={(v) => {
            if (!v) {
              // 关闭 popover（未选择日期），无副作用，高亮由 detectPreset 推导
              setCustomPopoverVisible(false);
            }
          }}
          trigger="click"
          position="bottomLeft"
          content={
            <div style={{ padding: 12 }}>
              <DatePicker
                type="dateRange"
                value={customDates}
                onChange={handleCustomDateChange}
                density="compact"
              />
            </div>
          }
        >
          <Button icon={<IconCalendar />} size="small">
            {activePreset === 'custom'
              ? `${customDates[0].toLocaleDateString('zh-CN')} - ${customDates[1].toLocaleDateString('zh-CN')}`
              : '选择日期'}
          </Button>
        </Popover>
      )}

      {/* 刷新按钮 */}
      <Button
        icon={<IconRefresh />}
        loading={loading}
        onClick={onRefresh}
        type="tertiary"
        size="small"
      >
        刷新
      </Button>
    </div>
  );
}

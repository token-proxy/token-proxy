/**
 * Dashboard 时间范围切换器组件。
 *
 * 整合预设范围（今日 / 7 天 / 30 天）、自定义日期区间和刷新按钮，
 * 挂载在「数据指标」和「用量趋势」卡片 `headerExtraContent` 内。
 *
 * 设计要点：
 * - 组件内部维护独立的 startDate / endDate 状态（均归一化到 00:00），
 *   与 DatePicker 的值解耦
 * - 预设按钮（今日 / 7 天 / 30 天）直接设置内部状态，customMode = false
 * - 自定义模式下打开 DatePicker，用户选择后更新内部状态
 * - 对外 onChange 输出的 end 为内部 end + 1 天，
 *   匹配后端 [start, end) 半开区间语义
 * - customMode 标志控制按钮高亮：customMode = true 时始终显示"自定义"，
 *   即使日期范围与预设一致也不跳回预设
 * - 自定义模式下按钮文字显示日期范围，预设不显示
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

// ─── 辅助函数 ────────────────────────────────────────────

/** 将 Date 归一化到当天 00:00 */
function toMidnight(d: Date): Date {
  const copy = new Date(d);
  copy.setHours(0, 0, 0, 0);
  return copy;
}

/** 将 Date 推迟一天 */
function addOneDay(d: Date): Date {
  const next = new Date(d);
  next.setDate(next.getDate() + 1);
  return next;
}

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
 * - today：start 是今天 00:00、end 是今天 00:00
 * - last7：start 是 7 天前 00:00、end 是今天 00:00
 * - last30：start 是 30 天前 00:00、end 是今天 00:00
 * - custom：其余
 */
function detectPreset(start: Date, end: Date): TimeRangePreset {
  const now = new Date();
  const todayMidnight = toMidnight(now);

  // today：start 是今天 00:00，end 也是今天 00:00
  if (isWithinTolerance(start, todayMidnight) && isWithinTolerance(end, todayMidnight)) {
    return 'today';
  }

  // last7：start 是 7 天前 00:00，end 是今天 00:00
  const last7Start = new Date(todayMidnight);
  last7Start.setDate(last7Start.getDate() - 7);
  if (isWithinTolerance(start, last7Start) && isWithinTolerance(end, todayMidnight)) {
    return 'last7';
  }

  // last30：start 是 30 天前 00:00，end 是今天 00:00
  const last30Start = new Date(todayMidnight);
  last30Start.setDate(last30Start.getDate() - 30);
  if (isWithinTolerance(start, last30Start) && isWithinTolerance(end, todayMidnight)) {
    return 'last30';
  }

  return 'custom';
}

/** 格式化日期为 YYYY/MM/DD */
function formatDate(d: Date): string {
  return `${d.getFullYear()}/${String(d.getMonth() + 1).padStart(2, '0')}/${String(d.getDate()).padStart(2, '0')}`;
}

/**
 * 时间范围切换器 + 刷新按钮。
 *
 * 组件内部维护独立的 startDate/endDate（归一化到 00:00）和 customMode 标志。
 * 对外 onChange 输出时，end 自动推迟一天以匹配后端 [start, end) 语义。
 *
 * 交互逻辑：
 * - 切换到 today / last7 / last30 时，自动计算对应日期范围并通过 onChange 通知父组件
 * - 切换到 custom 时，打开 DatePicker 供用户调整，**不**触发 onChange
 * - 用户选完日期后，通过 onChange 上报 `{ start, end: addOneDay(end) }`
 * - 关闭 Popover 时不清除 customMode，用户可通过预设按钮退出自定义模式
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
  // ─── 内部独立状态 ─────────────────────────────────────
  // 从外部 value 初始化，但归一化到 00:00
  const [startDate, setStartDate] = useState<Date>(() => toMidnight(value.start));
  const [endDate, setEndDate] = useState<Date>(() => toMidnight(value.end));
  // 是否处于自定义模式
  const [customMode, setCustomMode] = useState(
    () => detectPreset(value.start, value.end) === 'custom',
  );
  const [customPopoverVisible, setCustomPopoverVisible] = useState(false);

  // 推导当前高亮按钮
  const activePreset = customMode ? 'custom' : detectPreset(startDate, endDate);

  /** 处理预设切换 */
  const handlePresetChange = (next: TimeRangePreset) => {
    if (next === 'today') {
      const range = todayRange();
      setStartDate(range.start);
      setEndDate(range.end);
      setCustomMode(false);
      setCustomPopoverVisible(false);
      onChange({ start: range.start, end: addOneDay(range.end) });
    } else if (next === 'last7') {
      const range = last7Range();
      setStartDate(range.start);
      setEndDate(range.end);
      setCustomMode(false);
      setCustomPopoverVisible(false);
      onChange({ start: range.start, end: addOneDay(range.end) });
    } else if (next === 'last30') {
      const range = last30Range();
      setStartDate(range.start);
      setEndDate(range.end);
      setCustomMode(false);
      setCustomPopoverVisible(false);
      onChange({ start: range.start, end: addOneDay(range.end) });
    } else {
      // custom：打开 DatePicker，切换到自定义模式，不调 onChange
      setCustomMode(true);
      setCustomPopoverVisible(true);
    }
  };

  /** DatePicker 选择回调：更新内部状态并对外输出 end + 1 天 */
  const handleCustomDateChange: DatePickerProps['onChange'] = (dates) => {
    if (Array.isArray(dates) && dates.length === 2 && dates[0] && dates[1]) {
      const s = toMidnight(new Date(dates[0]));
      const e = toMidnight(new Date(dates[1]));
      setStartDate(s);
      setEndDate(e);
      setCustomPopoverVisible(false);
      onChange({ start: s, end: addOneDay(e) });
    }
  };

  // DatePicker 的受控值：使用内部状态的 start/end
  const customDates: [Date, Date] = [startDate, endDate];

  // 自定义按钮文字：自定义模式下显示日期范围，否则显示"选择日期"
  const customButtonText = customMode
    ? `${formatDate(startDate)} ~ ${formatDate(endDate)}`
    : '选择日期';

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
              // 关闭 popover：不清除 customMode，用户可通过预设按钮退出自定义模式
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
            {customButtonText}
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

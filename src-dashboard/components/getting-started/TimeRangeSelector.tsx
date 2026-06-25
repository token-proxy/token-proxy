/**
 * Dashboard 时间范围切换器组件。
 *
 * 整合预设范围（今日 / 7 天 / 30 天）、自定义日期区间和刷新按钮，
 * 挂载在「数据指标」卡片 `headerExtraContent` 内，使用小尺寸控件避免撑开卡片标题。
 *
 * 设计要点：
 * - 主体使用 Semi ButtonGroup 按钮组样式，4 个预设视觉上等宽紧凑，size=small
 * - "自定义"激活时弹出 Popover 内嵌 DatePicker（dateTimeRange 模式），
 *   避免首屏拥挤；触发按钮上回显已选区间
 * - 右侧刷新按钮（IconRefresh + loading spinner）
 * - 整体 flexWrap: wrap，窄屏自动换行
 */

import { useState } from 'react';
import { Button, ButtonGroup, DatePicker, Popover } from '@douyinfe/semi-ui';
import { IconCalendar, IconRefresh } from '@douyinfe/semi-icons';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import type { TimeRangePreset, TimeRangeQuery } from '../../types/dashboard';
import { toIsoString } from '../../utils/query';

/**
 * TimeRangeSelector 组件属性。
 */
export interface TimeRangeSelectorProps {
  /** 当前选择的时间范围 */
  value: TimeRangeQuery;
  /** 时间范围切换回调（用户切换预设或选择自定义区间时触发） */
  onChange: (next: TimeRangeQuery) => void;
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

/** 根据当前预设推导自定义日期范围，保证切换到自定义时继承用户刚才看到的时间窗。 */
function buildCustomRangeFromPreset(value: TimeRangeQuery): { start: string; end: string } {
  if (value.range === 'custom' && value.start && value.end) {
    return { start: value.start, end: value.end };
  }

  const end = new Date();
  const start = new Date(end);

  switch (value.range) {
    case 'today':
      start.setHours(0, 0, 0, 0);
      break;
    case 'last30':
      start.setDate(start.getDate() - 30);
      break;
    case 'last7':
    case 'custom':
      start.setDate(start.getDate() - 7);
      break;
  }

  return { start: start.toISOString(), end: end.toISOString() };
}

/**
 * 时间范围切换器 + 刷新按钮。
 *
 * 交互逻辑：
 * - 切换到 today / last7 / last30 时，onChange 传入 `{ range }` 不带 start/end
 * - 切换到 custom 时，只初始化为当前预设对应的同等时间窗并展开 Popover，不立即触发 onChange
 * - 自定义日期选完两端后，转换为 ISO 8601 字符串通过 onChange 上报
 *
 * @example
 * ```tsx
 * const [range, setRange] = useState<TimeRangeQuery>({ range: 'last7' });
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
  // 自定义日期 Popover 显隐控制（仅 custom 模式下使用）
  const [customPopoverVisible, setCustomPopoverVisible] = useState(false);
  const [customDraftDates, setCustomDraftDates] = useState<[Date, Date] | undefined>();

  /** 处理预设切换：custom 继承当前时间窗但不立即提交，其它清空 start/end */
  const handlePresetChange = (next: TimeRangePreset) => {
    if (next === 'custom') {
      const customRange = buildCustomRangeFromPreset(value);
      setCustomDraftDates([new Date(customRange.start), new Date(customRange.end)]);
      setCustomPopoverVisible(true);
    } else {
      setCustomDraftDates(undefined);
      onChange({ range: next });
      setCustomPopoverVisible(false);
    }
  };

  /** DatePicker 选择回调：将 [Date, Date] 转换为 ISO 字符串后上报 */
  const handleCustomDateChange: DatePickerProps['onChange'] = (dates) => {
    if (Array.isArray(dates) && dates.length === 2 && dates[0] && dates[1]) {
      const nextDates: [Date, Date] = [new Date(dates[0]), new Date(dates[1])];
      setCustomDraftDates(nextDates);
      onChange({
        range: 'custom',
        start: toIsoString(nextDates[0]),
        end: toIsoString(nextDates[1]),
      });
    }
  };

  // 转换 ISO 字符串为 Date 对象（DatePicker 受控 value 接受 Date）
  const customDates: [Date, Date] | undefined =
    value.range === 'custom' && value.start && value.end
      ? [new Date(value.start), new Date(value.end)]
      : customDraftDates;

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
            theme={value.range === preset ? 'solid' : 'light'}
            type="primary"
            onClick={() => handlePresetChange(preset)}
          >
            {PRESET_LABELS[preset]}
          </Button>
        ))}
      </ButtonGroup>

      {/* 自定义模式：日期选择器折叠在 Popover 内，避免首屏拥挤 */}
      {value.range === 'custom' && (
        <Popover
          visible={customPopoverVisible}
          onVisibleChange={setCustomPopoverVisible}
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
            {customDates
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

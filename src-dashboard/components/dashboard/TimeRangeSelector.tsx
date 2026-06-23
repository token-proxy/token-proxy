/**
 * Dashboard 时间范围切换器组件。
 *
 * 整合预设范围（今日 / 7 天 / 30 天）、自定义日期区间和刷新按钮，
 * 是 DashboardPage 顶部唯一的时间控制入口。
 *
 * 设计要点：
 * - 主体使用 Semi RadioGroup 按钮组样式，4 个预设视觉上等宽紧凑
 * - "自定义"激活时弹出 Popover 内嵌 DatePicker（dateTimeRange 模式），
 *   避免首屏拥挤；触发按钮上回显已选区间
 * - 右侧刷新按钮（IconRefresh + loading spinner）通过 flex: 1 占位推到末端
 * - 整体 flexWrap: wrap，窄屏自动换行
 */

import { useState } from 'react';
import { Button, DatePicker, Popover, RadioGroup, Radio } from '@douyinfe/semi-ui';
import { IconRefresh, IconCalendar } from '@douyinfe/semi-icons';
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
}

/** 预设范围的中文标签，按显示顺序排列 */
const PRESET_LABELS: Record<TimeRangePreset, string> = {
  today: '今日',
  last7: '7 天',
  last30: '30 天',
  custom: '自定义',
};

/** 自定义模式默认窗口（7 天）的毫秒数 */
const DEFAULT_CUSTOM_WINDOW_MS = 7 * 24 * 60 * 60 * 1000;

/**
 * 时间范围切换器 + 刷新按钮。
 *
 * 交互逻辑：
 * - 切换到 today / last7 / last30 时，onChange 传入 `{ range }` 不带 start/end
 * - 切换到 custom 时，自动初始化为最近 7 天（now - 7d ~ now）并展开 Popover
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
}: TimeRangeSelectorProps) {
  // 自定义日期 Popover 显隐控制（仅 custom 模式下使用）
  const [customPopoverVisible, setCustomPopoverVisible] = useState(false);

  /** 处理预设切换：custom 需要初始化默认区间并展开 Popover，其它清空 start/end */
  const handlePresetChange = (next: TimeRangePreset) => {
    if (next === 'custom') {
      const now = new Date();
      const sevenDaysAgo = new Date(now.getTime() - DEFAULT_CUSTOM_WINDOW_MS);
      onChange({
        range: 'custom',
        start: sevenDaysAgo.toISOString(),
        end: now.toISOString(),
      });
      setCustomPopoverVisible(true);
    } else {
      onChange({ range: next });
      setCustomPopoverVisible(false);
    }
  };

  /** DatePicker 选择回调：将 [Date, Date] 转换为 ISO 字符串后上报 */
  const handleCustomDateChange: DatePickerProps['onChange'] = (dates) => {
    if (Array.isArray(dates) && dates.length === 2 && dates[0] && dates[1]) {
      onChange({
        range: 'custom',
        start: toIsoString(dates[0]),
        end: toIsoString(dates[1]),
      });
    }
  };

  // 转换 ISO 字符串为 Date 对象（DatePicker 受控 value 接受 Date）
  const customDates: [Date, Date] | undefined =
    value.range === 'custom' && value.start && value.end
      ? [new Date(value.start), new Date(value.end)]
      : undefined;

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        flexWrap: 'wrap',
      }}
    >
      <RadioGroup
        type="button"
        value={value.range}
        onChange={(e) => handlePresetChange(e.target.value as TimeRangePreset)}
      >
        {(Object.keys(PRESET_LABELS) as TimeRangePreset[]).map((preset) => (
          <Radio key={preset} value={preset}>
            {PRESET_LABELS[preset]}
          </Radio>
        ))}
      </RadioGroup>

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
                type="dateTimeRange"
                value={customDates}
                onChange={handleCustomDateChange}
                density="compact"
              />
            </div>
          }
        >
          <Button icon={<IconCalendar />} size="default">
            {customDates
              ? `${customDates[0].toLocaleDateString('zh-CN')} - ${customDates[1].toLocaleDateString('zh-CN')}`
              : '选择日期'}
          </Button>
        </Popover>
      )}

      {/* 弹性占位：将刷新按钮推到行末 */}
      <div style={{ flex: 1 }} />

      <Button icon={<IconRefresh />} loading={loading} onClick={onRefresh} type="tertiary">
        刷新
      </Button>
    </div>
  );
}

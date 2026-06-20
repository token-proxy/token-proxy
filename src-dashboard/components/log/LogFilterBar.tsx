import { Button, DatePicker, Select, Typography } from '@douyinfe/semi-ui';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import type { ReactNode } from 'react';

const {Text} = Typography;

/** 筛选选项 */
interface FilterOption {
  id: string;
  label: string;
}

/** LogFilterBar 组件 Props */
interface LogFilterBarProps {
  users: FilterOption[];
  accessPoints: FilterOption[];
  userId?: string;
  accessPointId?: string;
  /** 隐藏内置的用户筛选下拉框（父组件自行作为 children 传入时可设为 true） */
  hideUserSelect?: boolean;
  /** 隐藏内置的接入点筛选下拉框 */
  hideAccessPointSelect?: boolean;
  /** 重置按钮之前插入的内容（如刷新按钮） */
  beforeReset?: ReactNode;
  datePickerWidth?: number;
  selectWidth?: number;
  onDateChange: DatePickerProps['onChange'];
  onUserChange: (value?: string) => void;
  onAccessPointChange: (value?: string) => void;
  onReset: () => void;
  children?: ReactNode;
}

/**
 * LogFilterBar - 日志/会话筛选栏组件
 *
 * 提供时间范围、用户、接入点等公共筛选条件，支持自定义子筛选控件。
 */
export default function LogFilterBar({
  users,
  accessPoints,
  userId,
  accessPointId,
  hideUserSelect = false,
  hideAccessPointSelect = false,
  beforeReset,
  datePickerWidth = 340,
  selectWidth = 140,
  onDateChange,
  onUserChange,
  onAccessPointChange,
  onReset,
  children,
}: LogFilterBarProps): ReactNode {
  return (
    <div
      style={{
        display: 'flex',
        gap: 12,
        marginBottom: 16,
        flexWrap: 'wrap',
        alignItems: 'flex-end',
      }}
    >
      <div>
        <Text style={{display: 'block', marginBottom: 4, fontSize: 13}}>时间范围</Text>
        <DatePicker
          type="dateTimeRange"
          onChange={onDateChange}
          style={{width: datePickerWidth}}
        />
      </div>
      {children}
      {!hideUserSelect && (
        <div>
          <Text style={{display: 'block', marginBottom: 4, fontSize: 13}}>用户</Text>
          <Select
            placeholder="选择用户"
            value={userId}
            onChange={(value) => onUserChange(value == null ? undefined : String(value))}
            style={{width: selectWidth}}
            showClear
          >
            {users.map((user) => (
              <Select.Option key={user.id} value={user.id}>{user.label}</Select.Option>
            ))}
          </Select>
        </div>
      )}
      {!hideAccessPointSelect && (
        <div>
          <Text style={{display: 'block', marginBottom: 4, fontSize: 13}}>接入点</Text>
          <Select
            placeholder="选择接入点"
            value={accessPointId}
            onChange={(value) => onAccessPointChange(value == null ? undefined : String(value))}
            style={{width: selectWidth}}
            showClear
          >
            {accessPoints.map((accessPoint) => (
              <Select.Option key={accessPoint.id} value={accessPoint.id}>{accessPoint.label}</Select.Option>
            ))}
          </Select>
        </div>
      )}
      {beforeReset}
      <Button type="tertiary" onClick={onReset}>重置</Button>
    </div>
  );
}

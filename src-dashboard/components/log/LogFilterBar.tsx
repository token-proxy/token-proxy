import { Button, DatePicker, Select, Typography } from '@douyinfe/semi-ui';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import type { ReactNode } from 'react';

const {Text} = Typography;

interface FilterOption {
  id: string;
  label: string;
}

interface LogFilterBarProps {
  users: FilterOption[];
  accessPoints: FilterOption[];
  userId?: string;
  accessPointId?: string;
  datePickerWidth?: number;
  selectWidth?: number;
  onDateChange: DatePickerProps['onChange'];
  onUserChange: (value?: string) => void;
  onAccessPointChange: (value?: string) => void;
  onReset: () => void;
  children?: ReactNode;
}

export default function LogFilterBar({
  users,
  accessPoints,
  userId,
  accessPointId,
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
      <Button onClick={onReset}>重置</Button>
    </div>
  );
}

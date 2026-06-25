/**
 * AuditLogPage - 审计日志查看页面
 *
 * 提供审计日志的分页浏览、多条件筛选（时间范围、操作类型、实体类型、操作者），
 * 以及展开查看操作详情的功能。
 */

import { type ReactNode, useEffect, useMemo, useState } from 'react';
import { Button, DatePicker, Select, Table, Typography } from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import { useFetch } from '../hooks/useFetch.ts';
import CopyableIdText from '@components/common/CopyableIdText';
import { auditLogApi } from '../api.ts';
import api from '../api.ts';
import { formatDateTime } from '../utils/format.ts';
import { toIsoString } from '../utils/query.ts';
import {
  ACTION_LABELS,
  ENTITY_TYPE_LABELS,
  ACTION_OPTIONS,
  ENTITY_TYPE_OPTIONS,
} from '../utils/auditLog.ts';
import type { AuditLogItem, AuditLogFilters } from '../types/auditLog.ts';
import type { UserItem } from '../types/log.ts';

const { Title, Text } = Typography;

// ─── 表格列定义 ───

/** 审计日志表格列配置 */
const COLUMNS = [
  {
    title: '时间',
    dataIndex: 'timestamp' satisfies keyof AuditLogItem,
    render: (_: unknown, record: AuditLogItem) => formatDateTime(record.timestamp),
  },
  {
    title: '操作者',
    dataIndex: 'operator_name' satisfies keyof AuditLogItem,
    render: (_: unknown, record: AuditLogItem) =>
      record.operator_name ?? <Text type="tertiary">系统</Text>,
  },
  {
    title: '实体类型',
    dataIndex: 'entity_type' satisfies keyof AuditLogItem,
    render: (_: unknown, record: AuditLogItem) =>
      ENTITY_TYPE_LABELS[record.entity_type] ?? record.entity_type,
  },
  {
    title: '操作类型',
    dataIndex: 'action' satisfies keyof AuditLogItem,
    render: (_: unknown, record: AuditLogItem) => ACTION_LABELS[record.action] ?? record.action,
  },
  {
    title: '实体 ID',
    dataIndex: 'entity_id' satisfies keyof AuditLogItem,
    render: (_: unknown, record: AuditLogItem) =>
      record.entity_id ? (
        <CopyableIdText value={record.entity_id} />
      ) : (
        <Text type="tertiary">—</Text>
      ),
  },
];

// ─── 组件 ───

/**
 * AuditLogPage - 审计日志查看页面
 *
 * 提供审计日志的分页浏览、多条件筛选（时间范围、操作类型、实体类型、操作者），
 * 以及展开查看操作详情的功能。
 */
export default function AuditLogPage(): ReactNode {
  // 参考数据
  const [users, setUsers] = useState<UserItem[]>([]);

  // 列表状态
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const [filters, setFilters] = useState<AuditLogFilters>({});

  // ─── 加载参考数据 ───

  useEffect(() => {
    api
      .get<UserItem[]>('/api/users')
      .then(setUsers)
      .catch(() => console.warn('[AuditLogPage] 加载用户参考数据失败'));
  }, []);

  // ─── 用户选项映射 ───

  const userOptions = useMemo(
    () => users.map((u) => ({ value: u.id, label: u.display_name })),
    [users],
  );

  // ─── 加载审计日志 ───

  const {
    data: logsData,
    loading,
    refetch: fetchLogs,
  } = useFetch(
    () => auditLogApi.list(page, pageSize, filters),
    [page, pageSize, JSON.stringify(filters)],
  );
  const logs = logsData?.items ?? [];
  const total = logsData?.total ?? 0;

  // ─── 筛选处理 ───

  const handleDateChange: DatePickerProps['onChange'] = (value) => {
    if (Array.isArray(value) && value.length === 2) {
      setFilters((prev) => ({
        ...prev,
        startTime: value[0] ? toIsoString(value[0]) : undefined,
        endTime: value[1] ? toIsoString(value[1]) : undefined,
      }));
    } else {
      setFilters((prev) => ({ ...prev, startTime: undefined, endTime: undefined }));
    }
  };

  const handleActionsChange = (values: string | string[] | undefined) => {
    const arr = Array.isArray(values) ? (values as string[]) : values ? [values as string] : [];
    setFilters((prev) => ({ ...prev, actions: arr.length > 0 ? arr : undefined }));
  };

  const handleEntityTypesChange = (values: string | string[] | undefined) => {
    const arr = Array.isArray(values) ? (values as string[]) : values ? [values as string] : [];
    setFilters((prev) => ({ ...prev, entityTypes: arr.length > 0 ? arr : undefined }));
  };

  const handleOperatorChange = (value: string | string[] | undefined) => {
    const val = value == null || Array.isArray(value) ? undefined : String(value);
    if (val === '__system__') {
      // 系统操作：按 operator_type 筛选，清除 operatorId
      setFilters((prev) => ({ ...prev, operatorId: undefined, operatorType: 'system' }));
    } else {
      // 具体用户：按 operator_id 筛选，清除 operatorType
      setFilters((prev) => ({ ...prev, operatorId: val, operatorType: undefined }));
    }
  };

  const handleReset = () => {
    setFilters({});
    setPage(1);
  };

  // 筛选条件变化时重置到第 1 页
  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  // ─── 展开行渲染 ───

  /** 审计日志详情 JSON 展开渲染 */
  function expandedRowRender(record: AuditLogItem | undefined): ReactNode {
    if (!record) return null;
    if (!record.details || Object.keys(record.details).length === 0) {
      return (
        <div style={{ padding: 16 }}>
          <Text type="tertiary">无详情</Text>
        </div>
      );
    }
    return (
      <pre
        style={{
          whiteSpace: 'pre-wrap',
          fontSize: 12,
          padding: 16,
          margin: 0,
          background: 'var(--semi-color-fill-0)',
          borderRadius: 4,
        }}
      >
        {JSON.stringify(record.details, null, 2)}
      </pre>
    );
  }

  // ─── 渲染 ───

  return (
    <div>
      {/* 标题区域 */}
      <div style={{ marginBottom: 16 }}>
        <Title heading={3} style={{ margin: 0 }}>
          审计日志
        </Title>
      </div>

      {/* 筛选栏 */}
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
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>时间范围</Text>
          <DatePicker type="dateTimeRange" onChange={handleDateChange} style={{ width: 340 }} />
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>操作者</Text>
          <Select
            filter
            placeholder="选择操作者"
            value={filters.operatorType === 'system' ? '__system__' : filters.operatorId}
            onChange={handleOperatorChange}
            style={{ width: 140 }}
            showClear
          >
            <Select.Option value="__system__">系统</Select.Option>
            {userOptions.map((u) => (
              <Select.Option key={u.value} value={u.value}>
                {u.label}
              </Select.Option>
            ))}
          </Select>
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>实体类型</Text>
          <Select
            multiple
            placeholder="选择实体类型"
            value={filters.entityTypes}
            onChange={handleEntityTypesChange}
            style={{ width: 160 }}
            showClear
          >
            {ENTITY_TYPE_OPTIONS.map((opt) => (
              <Select.Option key={opt.value} value={opt.value}>
                {opt.label}
              </Select.Option>
            ))}
          </Select>
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>操作类型</Text>
          <Select
            multiple
            placeholder="选择操作类型"
            value={filters.actions}
            onChange={handleActionsChange}
            style={{ width: 180 }}
            showClear
          >
            {ACTION_OPTIONS.map((opt) => (
              <Select.Option key={opt.value} value={opt.value}>
                {opt.label}
              </Select.Option>
            ))}
          </Select>
        </div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'flex-end', paddingBottom: 0 }}>
          <Button icon={<IconRefresh />} loading={loading} onClick={() => fetchLogs()}>
            刷新
          </Button>
          <Button type="tertiary" onClick={handleReset}>
            重置
          </Button>
        </div>
      </div>

      {/* 数据表格 */}
      <Table<AuditLogItem>
        columns={COLUMNS}
        dataSource={logs}
        rowKey="id"
        loading={loading}
        expandedRowRender={expandedRowRender}
        pagination={{
          currentPage: page,
          pageSize: pageSize,
          total: total,
          onPageChange: handlePageChange,
        }}
      />
    </div>
  );
}

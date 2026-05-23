import { Button, Popconfirm, Space, Table } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import StatusToggle from './StatusToggle.tsx';
import type { AccessPoint } from '../types/accessPoint.ts';

interface AccessPointTableProps {
  accessPoints: AccessPoint[];
  loading: boolean;
  operatingIds: string[];
  copyingUrl: boolean;
  onCopyUrl: (shortCode: string) => void;
  onEdit: (accessPoint: AccessPoint) => void;
  onDelete: (id: string) => void;
  onToggleEnabled: (accessPoint: AccessPoint) => void;
}

export default function AccessPointTable({
  accessPoints,
  loading,
  operatingIds,
  copyingUrl,
  onCopyUrl,
  onEdit,
  onDelete,
  onToggleEnabled,
}: AccessPointTableProps): ReactNode {
  const columns = [
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: 'Short Code', dataIndex: 'short_code', key: 'short_code', width: 160 },
    {
      title: '映射规则数',
      key: 'mapping_count',
      width: 120,
      render: (_: unknown, record: AccessPoint) =>
        record.model_mappings?.length ?? 0,
    },
    { title: 'API 类型', dataIndex: 'api_type', key: 'api_type', width: 100 },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (_: string, record: AccessPoint) => (
        <StatusToggle
          enabled={record.status === 'enabled'}
          loading={operatingIds.includes(record.id)}
          onToggle={() => onToggleEnabled(record)}
        />
      ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 220,
      render: (_: unknown, record: AccessPoint) => (
        <Space>
          <Button size="small" onClick={() => onCopyUrl(record.short_code)} loading={copyingUrl}>
            复制 URL
          </Button>
          <Button size="small" onClick={() => onEdit(record)}>编辑</Button>
          <Popconfirm
            title="确认删除此接入点?"
            onConfirm={() => onDelete(record.id)}
            position="bottomRight"
          >
            <Button size="small" type="danger" loading={operatingIds.includes(record.id)}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <Table
      columns={columns}
      dataSource={accessPoints}
      loading={loading}
      rowKey="id"
      scroll={{ x: 'max-content' }}
      pagination={{ pageSize: 20 }}
    />
  );
}

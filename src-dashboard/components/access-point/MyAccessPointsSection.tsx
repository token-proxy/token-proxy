/**
 * MyAccessPointsSection -「开始使用」页中"我的接入点"区块。
 *
 * 仅展示当前用户创建的接入点，以卡片网格呈现。
 * 新建 / 编辑复用 `AccessPointDrawer`（与接入点管理页共享同一表单组件）。
 */

import { Button, Card, Popconfirm, Tag, Typography } from '@douyinfe/semi-ui';
import { IconCopy, IconDelete, IconEdit, IconPlus } from '@douyinfe/semi-icons';
import { useState, type ReactNode } from 'react';
import useAccessPoints from '../../hooks/useAccessPoints';
import AccessPointDrawer from './AccessPointDrawer';
import type { AccessPoint } from '../../types/accessPoint';

const { Text } = Typography;

/** MyAccessPointsSection 组件 Props */
export interface MyAccessPointsSectionProps {
  /** 卡片标题右侧额外操作按钮（如跳转 API Key 配置） */
  extraHeaderContent?: ReactNode;
}

/**
 * 「我的接入点」区块组件。
 */
export function MyAccessPointsSection({
  extraHeaderContent,
}: MyAccessPointsSectionProps): ReactNode {
  const {
    accessPoints,
    loading,
    providers,
    accounts,
    accountsLoading,
    operatingIds,
    copyingUrl,
    emptyForm,
    saveAccessPoint,
    deleteAccessPoint,
    copyAccessUrl,
  } = useAccessPoints('mine');

  const [drawerVisible, setDrawerVisible] = useState(false);
  const [editingAccessPoint, setEditingAccessPoint] = useState<AccessPoint | null>(null);
  const [saving, setSaving] = useState(false);
  const [formData, setFormData] = useState(emptyForm);

  const handleEdit = (ap: AccessPoint) => {
    setFormData({
      name: ap.name,
      short_code: ap.short_code,
      api_type: ap.api_type,
      accounts:
        ap.accounts?.map((a) => ({
          account_id: a.account_id,
          weight: a.weight ?? 0,
          priority: a.priority ?? 0,
        })) ?? [],
      routing_strategy: ap.routing_strategy,
      model_routing_grid: ap.model_routing_grid ?? { provider_ids: [], rows: [] },
    });
    setEditingAccessPoint(ap);
    setDrawerVisible(true);
  };

  const handleCreate = () => {
    setFormData(emptyForm);
    setEditingAccessPoint(null);
    setDrawerVisible(true);
  };

  const handleClose = () => {
    setDrawerVisible(false);
    setEditingAccessPoint(null);
    setFormData(emptyForm);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const ok = await saveAccessPoint(formData, editingAccessPoint);
      if (ok) {
        handleClose();
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card
      title="我的接入点"
      headerExtraContent={
        <div style={{ display: 'flex', gap: 8 }}>
          {extraHeaderContent}
          <Button size="small" icon={<IconPlus />} onClick={handleCreate}>
            新建接入点
          </Button>
        </div>
      }
    >
      {!loading && accessPoints.length === 0 && (
        <Card
          bordered={false}
          style={{
            backgroundColor: 'var(--semi-color-bg-2)',
            borderRadius: 12,
            textAlign: 'center',
            padding: 40,
          }}
        >
          <Text type="secondary">你还没有创建任何接入点</Text>
          <div style={{ marginTop: 12 }}>
            <Button type="primary" icon={<IconPlus />} onClick={handleCreate}>
              创建你的第一个接入点
            </Button>
          </div>
        </Card>
      )}

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
          gap: 16,
        }}
      >
        {accessPoints.map((ap) => (
          <Card
            key={ap.id}
            style={{
              backgroundColor: 'var(--semi-color-bg-2)',
              borderRadius: 12,
            }}
            bodyStyle={{ padding: 16 }}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              <div
                style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}
              >
                <Text strong>{ap.name}</Text>
                <Tag size="small" color={ap.status === 'enabled' ? 'green' : 'grey'} shape="circle">
                  {ap.status === 'enabled' ? '启用' : '禁用'}
                </Tag>
              </div>

              <Text
                size="small"
                type="tertiary"
                style={{ fontFamily: 'monospace' }}
                ellipsis={{ showTooltip: true }}
              >
                /ap/{ap.short_code}
              </Text>

              <div style={{ display: 'flex', gap: 8 }}>
                <Tag size="small" color="blue" shape="circle">
                  {ap.api_type}
                </Tag>
                <Tag size="small" color="cyan" shape="circle">
                  {ap.routing_strategy}
                </Tag>
              </div>

              <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
                <Button
                  size="small"
                  icon={<IconCopy />}
                  loading={copyingUrl}
                  onClick={() => copyAccessUrl(ap.short_code)}
                >
                  复制链接
                </Button>
                <Button size="small" icon={<IconEdit />} onClick={() => handleEdit(ap)}>
                  编辑
                </Button>
                <Popconfirm title="确定删除此接入点？" onConfirm={() => deleteAccessPoint(ap.id)}>
                  <Button
                    size="small"
                    type="danger"
                    icon={<IconDelete />}
                    loading={operatingIds.includes(ap.id)}
                  >
                    删除
                  </Button>
                </Popconfirm>
              </div>
            </div>
          </Card>
        ))}
      </div>

      <AccessPointDrawer
        visible={drawerVisible}
        editingAccessPoint={editingAccessPoint}
        saving={saving}
        formData={formData}
        providers={providers}
        accounts={accounts}
        accountsLoading={accountsLoading}
        onClose={handleClose}
        onFormChange={setFormData}
        onSave={handleSave}
      />
    </Card>
  );
}

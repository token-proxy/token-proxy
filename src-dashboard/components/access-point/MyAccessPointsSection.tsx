/**
 * MyAccessPointsSection -「开始使用」页中"我的接入点"区块。
 *
 * 仅展示当前用户创建的接入点，以卡片网格呈现。
 * 新建 / 编辑复用 `AccessPointDrawer`（与接入点管理页共享同一表单组件）。
 */

import { Button, Card, Dropdown, Modal, Tag, Typography } from '@douyinfe/semi-ui';
import AutoColoredTag from '@components/common/AutoColoredTag';
import {
  IconCopy,
  IconDelete,
  IconEdit,
  IconPlus,
  IconChevronDown,
  IconTerminal,
  IconRefresh,
} from '@douyinfe/semi-icons';
import { useState, useMemo, type ReactNode } from 'react';
import useAccessPoints from '../../hooks/useAccessPoints';
import AccessPointDrawer from './AccessPointDrawer';
import SplitButtonGroup from '@douyinfe/semi-ui/lib/es/button/splitButtonGroup';
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
    copyingUrl,
    emptyForm,
    saveAccessPoint,
    deleteAccessPoint,
    copyAccessUrl,
    copyClaudeCodeCommand,
    refetch,
  } = useAccessPoints('mine');

  // 按创建时间升序，旧的在前、新的在后
  const sortedAccessPoints = useMemo(
    () =>
      [...accessPoints].sort(
        (a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
      ),
    [accessPoints],
  );

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
          <Button
            icon={<IconRefresh />}
            loading={loading}
            onClick={refetch}
            type="tertiary"
            size="small"
          >
            刷新
          </Button>
        </div>
      }
    >
      {!loading && sortedAccessPoints.length === 0 && (
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
        {sortedAccessPoints.map((ap) => {
          // 统计服务商数、账号总数、可用账号数
          const providerIds = new Set(ap.accounts?.map((a) => a.provider_id).filter(Boolean));
          const accountCount = ap.accounts?.length ?? 0;
          const availableCount = ap.accounts?.filter((a) => a.status === 'enabled').length ?? 0;

          const handleDeleteClick = () => {
            Modal.confirm({
              title: '确定删除此接入点？',
              content: '删除后不可恢复',
              onOk: () => deleteAccessPoint(ap.id),
            });
          };

          return (
            <Card
              key={ap.id}
              style={{
                backgroundColor: 'var(--semi-color-bg-2)',
                borderRadius: 12,
              }}
              bodyStyle={{ padding: 16 }}
            >
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {/* 第一行：名称 + 状态标签 */}
                <div
                  style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}
                >
                  <Text strong>{ap.name}</Text>
                  <Tag
                    size="small"
                    color={ap.status === 'enabled' ? 'green' : 'grey'}
                    shape="circle"
                  >
                    {ap.status === 'enabled' ? '启用' : '禁用'}
                  </Tag>
                </div>

                {/* 短码（去除 /ap/ 前缀，紧贴名称） */}
                <Text
                  size="small"
                  type="tertiary"
                  style={{ fontFamily: 'monospace', marginTop: -4 }}
                  ellipsis={{ showTooltip: true }}
                >
                  {ap.short_code}
                </Text>

                {/* 统计信息行 */}
                <Text style={{ marginTop: 4 }}>
                  {providerIds.size} 服务商 · {accountCount} 账号 · {availableCount} 可用
                </Text>

                {/* 底部：类型标签 + 操作按钮 */}
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    marginTop: 4,
                  }}
                >
                  <AutoColoredTag size="small" shape="circle">
                    {ap.api_type}
                  </AutoColoredTag>

                  <SplitButtonGroup>
                    <Button
                      size="small"
                      icon={<IconCopy />}
                      loading={copyingUrl}
                      onClick={() => copyAccessUrl(ap.short_code)}
                    >
                      复制链接
                    </Button>
                    <Dropdown
                      menu={[
                        {
                          node: 'item',
                          name: '编辑',
                          icon: <IconEdit />,
                          onClick: () => handleEdit(ap),
                        },
                        {
                          node: 'item',
                          name: '复制命令',
                          icon: <IconTerminal />,
                          onClick: () => copyClaudeCodeCommand(ap.short_code),
                        },
                        {
                          node: 'item',
                          name: '删除',
                          icon: <IconDelete />,
                          type: 'danger' as const,
                          onClick: handleDeleteClick,
                        },
                      ]}
                    >
                      <Button size="small" icon={<IconChevronDown />} />
                    </Dropdown>
                  </SplitButtonGroup>
                </div>
              </div>
            </Card>
          );
        })}
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

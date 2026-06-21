import { type ReactNode, useState } from 'react';
import { Button, Typography } from '@douyinfe/semi-ui';
import AccessPointDrawer from '@components/access-point/AccessPointDrawer';
import AccessPointTable from '@components/access-point/AccessPointTable';
import useAccessPoints from '../hooks/useAccessPoints.ts';
import { type AccessPoint } from '../types/accessPoint.ts';

const { Title } = Typography;

/**
 * AccessPointManagement - 接入点管理页面
 *
 * 接入点的列表展示、创建、编辑、删除、状态切换等操作入口。
 */
export default function AccessPointManagement(): ReactNode {
  const {
    accessPoints,
    loading,
    providers,
    accounts,
    accountsLoading,
    operatingIds,
    copyingUrl,
    emptyForm,
    clearAccounts,
    saveAccessPoint,
    deleteAccessPoint,
    toggleAccessPoint,
    copyAccessUrl,
    copyClaudeCodeCommand,
  } = useAccessPoints();

  const [drawerVisible, setDrawerVisible] = useState(false);
  const [editingAccessPoint, setEditingAccessPoint] = useState<AccessPoint | null>(null);
  const [saving, setSaving] = useState(false);
  const [formData, setFormData] = useState(emptyForm);

  const openCreateDrawer = () => {
    setEditingAccessPoint(null);
    setFormData({ ...emptyForm });
    clearAccounts();
    setDrawerVisible(true);
  };

  const openEditDrawer = (accessPoint: AccessPoint) => {
    setEditingAccessPoint(accessPoint);
    setFormData({
      name: accessPoint.name,
      short_code: accessPoint.short_code,
      api_type: accessPoint.api_type,
      accounts: accessPoint.accounts ?? [],
      routing_strategy: accessPoint.routing_strategy ?? 'weighted',
      model_routing_grid: accessPoint.model_routing_grid ?? { provider_ids: [], rows: [] },
    });
    setDrawerVisible(true);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const saved = await saveAccessPoint(formData, editingAccessPoint);
      if (saved) {
        setDrawerVisible(false);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <div>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 16,
        }}
      >
        <Title heading={3}>接入点管理</Title>
        <Button type="primary" onClick={openCreateDrawer}>
          创建接入点
        </Button>
      </div>

      <AccessPointTable
        accessPoints={accessPoints}
        loading={loading}
        operatingIds={operatingIds}
        copyingUrl={copyingUrl}
        onCopyUrl={copyAccessUrl}
        onCopyClaudeCodeCommand={copyClaudeCodeCommand}
        onEdit={openEditDrawer}
        onDelete={deleteAccessPoint}
        onToggleEnabled={toggleAccessPoint}
      />

      <AccessPointDrawer
        visible={drawerVisible}
        editingAccessPoint={editingAccessPoint}
        saving={saving}
        formData={formData}
        providers={providers}
        accounts={accounts}
        accountsLoading={accountsLoading}
        onClose={() => setDrawerVisible(false)}
        onFormChange={setFormData}
        onSave={handleSave}
      />
    </div>
  );
}

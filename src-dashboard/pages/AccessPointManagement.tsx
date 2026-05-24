import { useState, type ReactNode } from 'react';
import { Button, Typography } from '@douyinfe/semi-ui';
import AccessPointDrawer from '../components/AccessPointDrawer.tsx';
import AccessPointTable from '../components/AccessPointTable.tsx';
import useAccessPoints from '../hooks/useAccessPoints.ts';
import { DEFAULT_MODEL, UNMATCHED_MODEL, type AccessPoint, type AccessPointFormData, type ModelMapping } from '../types/accessPoint.ts';

const { Title } = Typography;

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
    loadProviderById,
    loadAccountsByProvider,
    clearAccounts,
    saveAccessPoint,
    deleteAccessPoint,
    toggleAccessPoint,
    copyAccessUrl,
  } = useAccessPoints();

  const [drawerVisible, setDrawerVisible] = useState(false);
  const [editingAccessPoint, setEditingAccessPoint] = useState<AccessPoint | null>(null);
  const [saving, setSaving] = useState(false);
  const [formData, setFormData] = useState<AccessPointFormData>(emptyForm);
  const [mappings, setMappings] = useState<ModelMapping[]>([]);

  const openCreateDrawer = () => {
    setEditingAccessPoint(null);
    setFormData(emptyForm);
    setMappings([]);
    clearAccounts();
    setDrawerVisible(true);
  };

  const openEditDrawer = (accessPoint: AccessPoint) => {
    setEditingAccessPoint(accessPoint);
    setFormData({
      name: accessPoint.name,
      short_code: accessPoint.short_code,
      provider_id: accessPoint.provider_id,
      account_id: accessPoint.account_id,
      api_type: accessPoint.api_type,
    });
    setMappings(accessPoint.model_mappings ?? []);
    if (accessPoint.provider_id) {
      loadAccountsByProvider(accessPoint.provider_id);
    }
    setDrawerVisible(true);
  };

  const handleProviderChange = async (providerId: string) => {
    const currentProvider = providers.find((item) => item.id === providerId);
    setFormData({ ...formData, provider_id: providerId, account_id: undefined });
    loadAccountsByProvider(providerId);

    let provider = currentProvider;
    try {
      provider = await loadProviderById(providerId);
    } catch {
      provider = currentProvider;
    }

    if (!editingAccessPoint && provider?.default_model) {
      setMappings([
        {
          source_model: UNMATCHED_MODEL,
          target_model: DEFAULT_MODEL,
          match_type: 'prefix',
        },
      ]);
    } else if (!editingAccessPoint) {
      setMappings([]);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const saved = await saveAccessPoint(formData, mappings, editingAccessPoint);
      if (saved) {
        setDrawerVisible(false);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <Title heading={3}>接入点管理</Title>
        <Button type="primary" onClick={openCreateDrawer}>创建接入点</Button>
      </div>

      <AccessPointTable
        accessPoints={accessPoints}
        loading={loading}
        operatingIds={operatingIds}
        copyingUrl={copyingUrl}
        onCopyUrl={copyAccessUrl}
        onEdit={openEditDrawer}
        onDelete={deleteAccessPoint}
        onToggleEnabled={toggleAccessPoint}
      />

      <AccessPointDrawer
        visible={drawerVisible}
        editingAccessPoint={editingAccessPoint}
        saving={saving}
        formData={formData}
        mappings={mappings}
        providers={providers}
        accounts={accounts}
        accountsLoading={accountsLoading}
        onClose={() => setDrawerVisible(false)}
        onFormChange={setFormData}
        onProviderChange={handleProviderChange}
        onMappingsChange={setMappings}
        onSave={handleSave}
      />
    </div>
  );
}

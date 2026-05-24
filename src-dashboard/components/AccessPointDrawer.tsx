import { Button, Input, Select, SideSheet } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import ModelMappingEditor, { matchTypeForSource } from './ModelMappingEditor.tsx';
import type {
  AccessPoint,
  AccessPointFormData,
  AccountOption,
  ModelMapping,
  ProviderOption,
} from '../types/accessPoint.ts';

interface AccessPointDrawerProps {
  visible: boolean;
  editingAccessPoint: AccessPoint | null;
  saving: boolean;
  formData: AccessPointFormData;
  mappings: ModelMapping[];
  providers: ProviderOption[];
  accounts: AccountOption[];
  accountsLoading: boolean;
  onClose: () => void;
  onFormChange: (formData: AccessPointFormData) => void;
  onProviderChange: (providerId: string) => void;
  onMappingsChange: (mappings: ModelMapping[]) => void;
  onSave: () => void;
}

export default function AccessPointDrawer({
  visible,
  editingAccessPoint,
  saving,
  formData,
  mappings,
  providers,
  accounts,
  accountsLoading,
  onClose,
  onFormChange,
  onProviderChange,
  onMappingsChange,
  onSave,
}: AccessPointDrawerProps): ReactNode {
  const handleAddMapping = () => {
    onMappingsChange([...mappings, { source_model: '', target_model: '', match_type: 'exact' }]);
  };

  const selectedProvider = providers.find((provider) => provider.id === formData.provider_id);
  const modelOptions = selectedProvider?.models ?? [];

  const handleRemoveMapping = (index: number) => {
    onMappingsChange(mappings.filter((_, i) => i !== index));
  };

  const handleMappingChange = (index: number, field: keyof ModelMapping, value: string) => {
    const next = [...mappings];
    const currentMapping = next[index];
    if (field === 'source_model') {
      next[index] = { ...currentMapping, source_model: value, match_type: matchTypeForSource(value) };
    } else {
      next[index] = { ...currentMapping, [field]: value };
    }
    onMappingsChange(next);
  };

  return (
    <SideSheet
      title={editingAccessPoint ? '编辑接入点' : '创建接入点'}
      visible={visible}
      onCancel={onClose}
      width={600}
      maskClosable={false}
    >
      <div style={{ padding: '0 4px' }}>
        <div>
          <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>名称</div>
          <Input
            value={formData.name}
            onChange={(value: string) => onFormChange({ ...formData, name: value })}
            placeholder="接入点名称"
          />
        </div>
        <div style={{ marginTop: 16 }}>
          <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>Short Code</div>
          <Input
            value={formData.short_code}
            onChange={(value: string) => onFormChange({ ...formData, short_code: value })}
            placeholder="留空则自动生成"
          />
        </div>
        <div style={{ marginTop: 16 }}>
          <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>Provider</div>
          <Select
            placeholder="选择 Provider"
            value={formData.provider_id}
            onChange={(value) => onProviderChange(value as string)}
            style={{ width: '100%' }}
          >
            {providers.map((provider) => (
              <Select.Option key={provider.id} value={provider.id}>{provider.name}</Select.Option>
            ))}
          </Select>
        </div>
        {selectedProvider && (
          <div style={{ marginTop: 8, color: 'var(--semi-color-text-2)', fontSize: 13 }}>
            默认模型: {selectedProvider.default_model || '未设置'}
          </div>
        )}
        <div style={{ marginTop: 16 }}>
          <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>Account</div>
          <Select
            placeholder="选择 Account"
            value={formData.account_id}
            onChange={(value) => onFormChange({ ...formData, account_id: value as string })}
            loading={accountsLoading}
            style={{ width: '100%' }}
          >
            {accounts.map((account) => (
              <Select.Option key={account.id} value={account.id}>
                {account.name} (******{account.api_key_suffix})
              </Select.Option>
            ))}
          </Select>
        </div>
        <div style={{ marginTop: 16 }}>
          <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>API 类型</div>
          <Select
            value={formData.api_type}
            onChange={(value) => onFormChange({ ...formData, api_type: value as string })}
            style={{ width: '100%' }}
          >
            <Select.Option value="anthropic">Anthropic</Select.Option>
          </Select>
        </div>

        <ModelMappingEditor
          mappings={mappings}
          apiType={formData.api_type}
          modelOptions={modelOptions}
          defaultModel={selectedProvider?.default_model}
          onAdd={handleAddMapping}
          onRemove={handleRemoveMapping}
          onChange={handleMappingChange}
        />

        <Button
          type="primary"
          onClick={onSave}
          loading={saving}
          block
          style={{ marginTop: 24 }}
        >
          {editingAccessPoint ? '更新' : '创建'}
        </Button>
      </div>
    </SideSheet>
  );
}

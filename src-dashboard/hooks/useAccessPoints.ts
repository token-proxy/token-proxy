import { useCallback, useEffect, useRef, useState } from 'react';
import { Toast } from '@douyinfe/semi-ui';
import api from '../api.ts';
import {
  type AccessPoint,
  type AccessPointFormData,
  type AccountOption,
  type ModelMapping,
  type ProviderOption,
} from '../types/accessPoint.ts';

const EMPTY_FORM: AccessPointFormData = {
  name: '',
  short_code: '',
  provider_id: undefined,
  account_id: undefined,
  api_type: 'anthropic',
};

export default function useAccessPoints() {
  const [accessPoints, setAccessPoints] = useState<AccessPoint[]>([]);
  const [loading, setLoading] = useState(false);
  const [providers, setProviders] = useState<ProviderOption[]>([]);
  const [accounts, setAccounts] = useState<AccountOption[]>([]);
  const [accountsLoading, setAccountsLoading] = useState(false);
  const [operatingIds, setOperatingIds] = useState<string[]>([]);
  const [copyingUrl, setCopyingUrl] = useState(false);
  const operatingIdsRef = useRef<Set<string>>(new Set());

  const setOperation = (key: string, operating: boolean) => {
    const next = new Set(operatingIdsRef.current);
    if (operating) {
      next.add(key);
    } else {
      next.delete(key);
    }
    operatingIdsRef.current = next;
    setOperatingIds([...next]);
  };

  const loadAccessPoints = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.get<AccessPoint[]>('/api/access-points');
      setAccessPoints(data);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '获取接入点列表失败');
    } finally {
      setLoading(false);
    }
  }, []);

  const loadProviders = useCallback(async () => {
    try {
      const data = await api.get<ProviderOption[]>('/api/providers');
      setProviders(data);
    } catch {
      // providers 数据可能尚未就绪
    }
  }, []);

  const loadProviderById = useCallback(async (providerId: string) => {
    const provider = await api.get<ProviderOption>(`/api/providers/${providerId}`);
    setProviders((current) => current.map((item) => (item.id === provider.id ? provider : item)));
    return provider;
  }, []);

  const loadAccountsByProvider = useCallback(async (providerId: string) => {
    setAccountsLoading(true);
    try {
      const data = await api.get<AccountOption[]>(`/api/providers/${providerId}/accounts`);
      setAccounts(data);
    } catch {
      setAccounts([]);
    } finally {
      setAccountsLoading(false);
    }
  }, []);

  const clearAccounts = useCallback(() => {
    setAccounts([]);
  }, []);

  useEffect(() => {
    loadAccessPoints();
    loadProviders();
  }, [loadAccessPoints, loadProviders]);

  const saveAccessPoint = async (
    formData: AccessPointFormData,
    mappings: ModelMapping[],
    defaultModel: string | undefined,
    editingAccessPoint: AccessPoint | null,
  ) => {
    if (!formData.name) {
      Toast.error('请输入接入点名称');
      return false;
    }
    if (!formData.provider_id) {
      Toast.error('请选择 Provider');
      return false;
    }
    if (!formData.account_id) {
      Toast.error('请选择 Account');
      return false;
    }

    const provider = providers.find((item) => item.id === formData.provider_id);
    const allowedTargetModels = new Set(provider?.models ?? []);
    const validMappings = mappings.filter(
      (mapping) => mapping.source_model && mapping.target_model && allowedTargetModels.has(mapping.target_model),
    );

    if (editingAccessPoint) {
      await api.put(`/api/access-points/${editingAccessPoint.id}`, {
        name: formData.name,
        provider_id: formData.provider_id,
        account_id: formData.account_id,
        api_type: formData.api_type,
        model_mappings: validMappings.length > 0 ? validMappings : undefined,
        default_model: defaultModel || '',
      });
      Toast.success('接入点已更新');
    } else {
      await api.post('/api/access-points', {
        name: formData.name,
        provider_id: formData.provider_id,
        account_id: formData.account_id,
        api_type: formData.api_type,
        short_code: formData.short_code || undefined,
        model_mappings: validMappings.length > 0 ? validMappings : undefined,
        default_model: defaultModel || '',
      });
      Toast.success('接入点已创建');
    }

    loadAccessPoints();
    return true;
  };

  const deleteAccessPoint = async (id: string) => {
    if (operatingIdsRef.current.has(id)) return;
    setOperation(id, true);
    try {
      await api.delete(`/api/access-points/${id}`);
      Toast.success('接入点已删除');
      loadAccessPoints();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '删除失败');
    } finally {
      setOperation(id, false);
    }
  };

  const toggleAccessPoint = async (accessPoint: AccessPoint) => {
    if (operatingIdsRef.current.has(accessPoint.id)) return;
    setOperation(accessPoint.id, true);
    const nextStatus = accessPoint.status === 'enabled' ? 'disabled' : 'enabled';
    try {
      await api.put(`/api/access-points/${accessPoint.id}`, {status: nextStatus});
      Toast.success(`接入点已${nextStatus === 'enabled' ? '启用' : '禁用'}`);
      loadAccessPoints();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '操作失败');
    } finally {
      setOperation(accessPoint.id, false);
    }
  };

  const copyAccessUrl = async (shortCode: string) => {
    if (copyingUrl) return;
    setCopyingUrl(true);
    const baseUrl = `${window.location.protocol}//${window.location.host}`;
    const url = `${baseUrl}/ap/${shortCode}`;
    try {
      await navigator.clipboard.writeText(url);
      Toast.success('接入链接已复制');
    } catch {
      Toast.error('复制失败');
    } finally {
      setCopyingUrl(false);
    }
  };

  const copyClaudeCodeCommand = async (shortCode: string) => {
    if (copyingUrl) return;
    setCopyingUrl(true);
    const baseUrl = `${window.location.protocol}//${window.location.host}`;
    const command = `ANTHROPIC_BASE_URL=${baseUrl}/ap/${shortCode} claude`;
    try {
      await navigator.clipboard.writeText(command);
      Toast.success('Claude Code 启动命令已复制');
    } catch {
      Toast.error('复制失败');
    } finally {
      setCopyingUrl(false);
    }
  };

  return {
    accessPoints,
    loading,
    providers,
    accounts,
    accountsLoading,
    operatingIds,
    copyingUrl,
    emptyForm: EMPTY_FORM,
    loadProviderById,
    loadAccountsByProvider,
    clearAccounts,
    saveAccessPoint,
    deleteAccessPoint,
    toggleAccessPoint,
    copyAccessUrl,
    copyClaudeCodeCommand,
  };
}

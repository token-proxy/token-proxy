import { useCallback, useEffect, useRef, useState } from 'react';
import { Toast } from '@douyinfe/semi-ui';
import api from '../api.ts';
import {
  type AccessPoint,
  type AccessPointFormData,
  type AccountOption,
  type ProviderOption,
  UNMATCHED_MODEL,
} from '../types/accessPoint.ts';

const EMPTY_FORM: AccessPointFormData = {
  name: '',
  short_code: '',
  api_type: 'anthropic',
  accounts: [],
  routing_strategy: 'weighted',
  model_routing_grid: { provider_ids: [], rows: [] },
};

/**
 * useAccessPoints - 接入点数据管理 Hook
 *
 * 封装接入点的增删改查、状态切换、服务商/账号加载等功能。
 * 提供接入点列表、服务商树、账号池等数据。
 *
 * @returns 接入点列表、加载状态、服务商/账号数据、CRUD 操作方法、复制 URL 等工具方法
 */
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
      console.warn('[useAccessPoints] 加载服务商列表失败，数据可能尚未就绪');
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
      console.warn('[useAccessPoints] 加载账号列表失败');
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
    editingAccessPoint: AccessPoint | null,
  ) => {
    if (!formData.name) {
      Toast.error('请输入接入点名称');
      return false;
    }
    // 账号池验证
    if (formData.accounts.length === 0) {
      Toast.error('请至少添加一个账号');
      return false;
    }
    for (const b of formData.accounts) {
      if (!b.account_id) {
        Toast.error('账号池中存在未选择账号的行');
        return false;
      }
      if (formData.routing_strategy === 'weighted' && (b.weight == null || b.weight <= 0)) {
        Toast.error('权重必须大于 0');
        return false;
      }
    }
    // 模型路由网格验证
    const grid = formData.model_routing_grid;
    for (const row of grid.rows) {
      if (!row.source_model) {
        Toast.error('模型路由表中存在空的原始模型，请填写或删除该行');
        return false;
      }
    }
    // 未匹配行所有列不能为空
    const unmatched = grid.rows.find((r) => r.source_model === UNMATCHED_MODEL);
    if (unmatched) {
      for (const pid of grid.provider_ids) {
        if (!unmatched.targets[pid]) {
          Toast.error('未匹配行的所有服务商列都必须填写目标模型');
          return false;
        }
      }
    }

    if (editingAccessPoint) {
      await api.put(`/api/access-points/${editingAccessPoint.id}`, {
        name: formData.name,
        short_code: formData.short_code,
        api_type: formData.api_type,
        accounts: formData.accounts,
        routing_strategy: formData.routing_strategy,
        model_routing_grid: formData.model_routing_grid,
      });
      Toast.success('接入点已更新');
    } else {
      await api.post('/api/access-points', {
        name: formData.name,
        short_code: formData.short_code || undefined,
        api_type: formData.api_type,
        accounts: formData.accounts,
        routing_strategy: formData.routing_strategy,
        model_routing_grid: formData.model_routing_grid,
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
      await api.put(`/api/access-points/${accessPoint.id}`, { status: nextStatus });
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

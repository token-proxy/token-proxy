import { useState, useEffect, useCallback, useRef, type ReactNode } from 'react';
import {
  Table, Button, Tag, Space, Popconfirm, SideSheet,
  Toast, Typography, Select, Input,
} from '@douyinfe/semi-ui';
import api from '../api.ts';

const { Title } = Typography;

interface Provider {
  id: string;
  name: string;
}

interface Account {
  id: string;
  provider_id: string;
  name: string;
  api_key_suffix: string;
  status: string;
}

interface ModelMapping {
  source_model: string;
  target_model: string;
}

interface AccessPoint {
  id: string;
  name: string;
  short_code: string;
  provider_id: string;
  account_id: string;
  api_type: string;
  model_mappings: ModelMapping[];
  access_url: string;
  status: string;
  created_at: string;
  updated_at: string;
}

interface AccessPointFormData {
  name: string;
  short_code: string;
  provider_id: string | undefined;
  account_id: string | undefined;
  api_type: string;
}

export default function AccessPointManagement(): ReactNode {
  const [accessPoints, setAccessPoints] = useState<AccessPoint[]>([]);
  const [loading, setLoading] = useState(false);
  const [drawerVisible, setDrawerVisible] = useState(false);
  const [editingAp, setEditingAp] = useState<AccessPoint | null>(null);
  const [saving, setSaving] = useState(false);
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

  // Form state
  const [formData, setFormData] = useState<AccessPointFormData>({
    name: '',
    short_code: '',
    provider_id: undefined,
    account_id: undefined,
    api_type: 'default',
  });
  const [mappings, setMappings] = useState<ModelMapping[]>([]);

  // Cascading select data
  const [providers, setProviders] = useState<Provider[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [accountsLoading, setAccountsLoading] = useState(false);

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
      const data = await api.get<Provider[]>('/api/providers');
      setProviders(data);
    } catch {
      // providers may not be available yet
    }
  }, []);

  useEffect(() => {
    loadAccessPoints();
    loadProviders();
  }, [loadAccessPoints, loadProviders]);

  const loadAccountsByProvider = async (providerId: string) => {
    setAccountsLoading(true);
    try {
      const data = await api.get<Account[]>(`/api/providers/${providerId}/accounts`);
      setAccounts(data);
    } catch {
      setAccounts([]);
    } finally {
      setAccountsLoading(false);
    }
  };

  const openCreateDrawer = () => {
    setEditingAp(null);
    setFormData({
      name: '',
      short_code: '',
      provider_id: undefined,
      account_id: undefined,
      api_type: 'default',
    });
    setMappings([]);
    setAccounts([]);
    setDrawerVisible(true);
  };

  const openEditDrawer = (ap: AccessPoint) => {
    setEditingAp(ap);
    setFormData({
      name: ap.name,
      short_code: ap.short_code,
      provider_id: ap.provider_id,
      account_id: ap.account_id,
      api_type: ap.api_type,
    });
    setMappings(ap.model_mappings ?? []);

    if (ap.provider_id) {
      loadAccountsByProvider(ap.provider_id);
    }
    setDrawerVisible(true);
  };

  const handleProviderChange = (value: string) => {
    setFormData({ ...formData, provider_id: value, account_id: undefined });
    loadAccountsByProvider(value);
  };

  const handleAddMapping = () => {
    setMappings([...mappings, { source_model: '', target_model: '' }]);
  };

  const handleRemoveMapping = (index: number) => {
    const next = mappings.filter((_, i) => i !== index);
    setMappings(next);
  };

  const handleMappingChange = (index: number, field: keyof ModelMapping, value: string) => {
    const next = [...mappings];
    next[index] = { ...next[index], [field]: value };
    setMappings(next);
  };

  const handleSave = async () => {
    if (!formData.name) {
      Toast.error('请输入接入点名称');
      return;
    }
    if (!formData.provider_id) {
      Toast.error('请选择 Provider');
      return;
    }
    if (!formData.account_id) {
      Toast.error('请选择 Account');
      return;
    }

    setSaving(true);
    try {
      const validMappings = mappings.filter((m) => m.source_model && m.target_model);

      const body = {
        name: formData.name,
        provider_id: formData.provider_id,
        account_id: formData.account_id,
        short_code: formData.short_code || undefined,
        model_mappings: validMappings.length > 0 ? validMappings : undefined,
      };

      if (editingAp) {
        await api.put(`/api/access-points/${editingAp.id}`, {
          name: formData.name,
          provider_id: formData.provider_id,
          account_id: formData.account_id,
          model_mappings: validMappings.length > 0 ? validMappings : undefined,
        });
        Toast.success('接入点已更新');
      } else {
        await api.post('/api/access-points', body);
        Toast.success('接入点已创建');
      }
      setDrawerVisible(false);
      loadAccessPoints();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
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

  const handleToggleEnabled = async (ap: AccessPoint) => {
    if (operatingIdsRef.current.has(ap.id)) return;
    setOperation(ap.id, true);
    const nextStatus = ap.status === 'enabled' ? 'disabled' : 'enabled';
    try {
      await api.put(`/api/access-points/${ap.id}`, { status: nextStatus });
      Toast.success(`接入点已${nextStatus === 'enabled' ? '启用' : '禁用'}`);
      loadAccessPoints();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '操作失败');
    } finally {
      setOperation(ap.id, false);
    }
  };

  const copyAccessUrl = async (shortCode: string) => {
    if (copyingUrl) return;
    setCopyingUrl(true);
    const baseUrl = `${window.location.protocol}//${window.location.host}`;
    const url = `${baseUrl}/ap/${shortCode}`;
    try {
      await navigator.clipboard.writeText(url);
      Toast.success('接入 URL 已复制');
    } catch {
      Toast.error('复制失败');
    } finally {
      setCopyingUrl(false);
    }
  };

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
      render: (_: string, record: AccessPoint) => {
        const enabled = record.status === 'enabled';
        const operating = operatingIds.includes(record.id);
        const tag = (
          <Tag
            color={enabled ? 'green' : 'red'}
            style={{ cursor: operating ? 'not-allowed' : 'pointer', opacity: operating ? 0.5 : 1 }}
          >
            {enabled ? '启用' : '禁用'}
          </Tag>
        );
        if (operating) return tag;
        return (
          <Popconfirm
            title={`确认${enabled ? '禁用' : '启用'}?`}
            onConfirm={() => handleToggleEnabled(record)}
            position="bottomRight"
          >
            {tag}
          </Popconfirm>
        );
      },
    },
    {
      title: '操作',
      key: 'actions',
      width: 220,
      render: (_: unknown, record: AccessPoint) => (
        <Space>
          <Button size="small" onClick={() => copyAccessUrl(record.short_code)} loading={copyingUrl}>
            复制 URL
          </Button>
          <Button size="small" onClick={() => openEditDrawer(record)}>编辑</Button>
          <Popconfirm
            title="确认删除此接入点?"
            onConfirm={() => handleDelete(record.id)}
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
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <Title heading={3}>接入点管理</Title>
        <Button type="primary" onClick={openCreateDrawer}>创建接入点</Button>
      </div>

      <Table
        columns={columns}
        dataSource={accessPoints}
        loading={loading}
        rowKey="id"
        scroll={{ x: 'max-content' }}
        pagination={{ pageSize: 20 }}
      />

      <SideSheet
        title={editingAp ? '编辑接入点' : '创建接入点'}
        visible={drawerVisible}
        onCancel={() => setDrawerVisible(false)}
        width={600}
        maskClosable={false}
      >
        <div style={{ padding: '0 4px' }}>
          <div>
            <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>名称</div>
            <Input
              value={formData.name}
              onChange={(v: string) => setFormData({ ...formData, name: v })}
              placeholder="接入点名称"
            />
          </div>
          <div style={{ marginTop: 16 }}>
            <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>Short Code</div>
            <Input
              value={formData.short_code}
              onChange={(v: string) => setFormData({ ...formData, short_code: v })}
              placeholder="留空则自动生成"
            />
          </div>
          <div style={{ marginTop: 16 }}>
            <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>Provider</div>
            <Select
              placeholder="选择 Provider"
              value={formData.provider_id}
              onChange={(v) => handleProviderChange(v as string)}
              style={{ width: '100%' }}
            >
              {providers.map((p) => (
                <Select.Option key={p.id} value={p.id}>{p.name}</Select.Option>
              ))}
            </Select>
          </div>
          <div style={{ marginTop: 16 }}>
            <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>Account</div>
            <Select
              placeholder="选择 Account"
              value={formData.account_id}
              onChange={(v) => setFormData({ ...formData, account_id: v as string })}
              loading={accountsLoading}
              style={{ width: '100%' }}
            >
              {accounts.map((a) => (
                <Select.Option key={a.id} value={a.id}>
                  {a.name} (******{a.api_key_suffix})
                </Select.Option>
              ))}
            </Select>
          </div>

          <div style={{ marginTop: 24 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
              <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--semi-color-text-0)' }}>模型映射</span>
              <Button size="small" onClick={handleAddMapping}>添加映射</Button>
            </div>
            {mappings.length === 0 && (
              <div style={{ color: 'var(--semi-color-text-2)', fontSize: 13, padding: '8px 0' }}>
                暂无映射规则，点击"添加映射"新增
              </div>
            )}
            {mappings.map((m, i) => (
              <div key={i} style={{ display: 'flex', gap: 8, marginBottom: 8, alignItems: 'center' }}>
                <Input
                  value={m.source_model}
                  onChange={(v: string) => handleMappingChange(i, 'source_model', v)}
                  placeholder="源模型"
                />
                <span style={{ color: 'var(--semi-color-text-2)' }}>→</span>
                <Input
                  value={m.target_model}
                  onChange={(v: string) => handleMappingChange(i, 'target_model', v)}
                  placeholder="目标模型"
                />
                <Button type="danger" icon={null} onClick={() => handleRemoveMapping(i)} size="small">删除</Button>
              </div>
            ))}
          </div>

          <Button
            type="primary"
            onClick={handleSave}
            loading={saving}
            block
            style={{ marginTop: 24 }}
          >
            {editingAp ? '更新' : '创建'}
          </Button>
        </div>
      </SideSheet>
    </div>
  );
}

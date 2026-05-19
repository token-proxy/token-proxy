import { useState, useEffect, useCallback, type ReactNode } from 'react';
import {
  Table, Button, Tag, Space, Popconfirm, SideSheet, Form,
  Toast, Typography, Select,
} from '@douyinfe/semi-ui';
import api from '../api.ts';

const { Title } = Typography;

interface Provider {
  id: number;
  name: string;
}

interface Account {
  id: number;
  provider_id: number;
  name: string;
  model: string;
}

interface ModelMapping {
  source: string;
  target: string;
}

interface AccessPoint {
  id: number;
  name: string;
  short_code: string;
  provider_id: number;
  provider_name?: string;
  account_id: number;
  account_name?: string;
  api_type: string;
  model_mapping: Record<string, string>;
  enabled: boolean;
}

interface AccessPointFormData {
  name: string;
  short_code: string;
  provider_id: number | undefined;
  account_id: number | undefined;
  api_type: string;
  model_mapping: Record<string, string>;
}

export default function AccessPointManagement(): ReactNode {
  const [accessPoints, setAccessPoints] = useState<AccessPoint[]>([]);
  const [loading, setLoading] = useState(false);
  const [drawerVisible, setDrawerVisible] = useState(false);
  const [editingAp, setEditingAp] = useState<AccessPoint | null>(null);
  const [saving, setSaving] = useState(false);

  // Form state
  const [formData, setFormData] = useState<AccessPointFormData>({
    name: '',
    short_code: '',
    provider_id: undefined,
    account_id: undefined,
    api_type: 'default',
    model_mapping: {},
  });
  const [mappings, setMappings] = useState<ModelMapping[]>([]);

  // Cascading select data
  const [providers, setProviders] = useState<Provider[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [accountsLoading, setAccountsLoading] = useState(false);

  const baseUrl = `${window.location.protocol}//${window.location.host}`;

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

  const loadAccountsByProvider = async (providerId: number) => {
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
      model_mapping: {},
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
      model_mapping: ap.model_mapping,
    });
    const entries = Object.entries(ap.model_mapping || {});
    setMappings(entries.map(([source, target]) => ({ source, target })));

    if (ap.provider_id) {
      loadAccountsByProvider(ap.provider_id);
    }
    setDrawerVisible(true);
  };

  const handleProviderChange = (value: number) => {
    setFormData({ ...formData, provider_id: value, account_id: undefined });
    loadAccountsByProvider(value);
  };

  const handleAddMapping = () => {
    setMappings([...mappings, { source: '', target: '' }]);
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

    setSaving(true);
    try {
      const modelMapping: Record<string, string> = {};
      mappings.forEach((m) => {
        if (m.source && m.target) {
          modelMapping[m.source] = m.target;
        }
      });

      const body = {
        ...formData,
        model_mapping: modelMapping,
      };

      if (editingAp) {
        await api.put(`/api/access-points/${editingAp.id}`, body);
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

  const handleDelete = async (id: number) => {
    try {
      await api.delete(`/api/access-points/${id}`);
      Toast.success('接入点已删除');
      loadAccessPoints();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '删除失败');
    }
  };

  const handleToggleEnabled = async (ap: AccessPoint) => {
    try {
      await api.put(`/api/access-points/${ap.id}`, {
        name: ap.name,
        short_code: ap.short_code,
        provider_id: ap.provider_id,
        account_id: ap.account_id,
        api_type: ap.api_type,
        model_mapping: ap.model_mapping,
        enabled: !ap.enabled,
      });
      Toast.success(`接入点已${ap.enabled ? '禁用' : '启用'}`);
      loadAccessPoints();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '操作失败');
    }
  };

  const copyAccessUrl = (shortCode: string) => {
    const url = `${baseUrl}/ap/${shortCode}`;
    navigator.clipboard.writeText(url).then(() => {
      Toast.success('接入 URL 已复制');
    }).catch(() => {
      Toast.error('复制失败');
    });
  };

  const columns = [
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: 'Short Code', dataIndex: 'short_code', key: 'short_code' },
    { title: 'Provider', dataIndex: 'provider_name', key: 'provider', render: (text?: string) => text || '-' },
    {
      title: '映射规则数',
      key: 'mapping_count',
      render: (_: unknown, record: AccessPoint) =>
        Object.keys(record.model_mapping || {}).length,
    },
    { title: 'API 类型', dataIndex: 'api_type', key: 'api_type' },
    {
      title: '状态',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (_: boolean, record: AccessPoint) => (
        <Popconfirm
          title={`确认${record.enabled ? '禁用' : '启用'}?`}
          onConfirm={() => handleToggleEnabled(record)}
          position="bottomRight"
        >
          <Tag color={record.enabled ? 'green' : 'red'} style={{ cursor: 'pointer' }}>
            {record.enabled ? '启用' : '禁用'}
          </Tag>
        </Popconfirm>
      ),
    },
    {
      title: '操作',
      key: 'actions',
      render: (_: unknown, record: AccessPoint) => (
        <Space>
          <Button size="small" onClick={() => copyAccessUrl(record.short_code)}>复制 URL</Button>
          <Button size="small" onClick={() => openEditDrawer(record)}>编辑</Button>
          <Popconfirm
            title="确认删除此接入点?"
            onConfirm={() => handleDelete(record.id)}
            position="bottomRight"
          >
            <Button size="small" type="danger">删除</Button>
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
          <Form.Input
            label="名称"
            value={formData.name}
            onChange={(v: string) => setFormData({ ...formData, name: v })}
            placeholder="接入点名称"
          />
          <div style={{ marginTop: 16 }}>
            <Form.Input
              label="Short Code"
              value={formData.short_code}
              onChange={(v: string) => setFormData({ ...formData, short_code: v })}
              placeholder="留空则自动生成"
            />
          </div>
          <div style={{ marginTop: 16 }}>
            <label style={{ display: 'block', marginBottom: 8, fontSize: 14, fontWeight: 500, color: 'var(--semi-color-text-0)' }}>Provider</label>
            <Select
              placeholder="选择 Provider"
              value={formData.provider_id}
              onChange={handleProviderChange}
              style={{ width: '100%' }}
            >
              {providers.map((p) => (
                <Select.Option key={p.id} value={p.id}>{p.name}</Select.Option>
              ))}
            </Select>
          </div>
          <div style={{ marginTop: 16 }}>
            <label style={{ display: 'block', marginBottom: 8, fontSize: 14, fontWeight: 500, color: 'var(--semi-color-text-0)' }}>Account</label>
            <Select
              placeholder="选择 Account"
              value={formData.account_id}
              onChange={(v: number) => setFormData({ ...formData, account_id: v })}
              loading={accountsLoading}
              style={{ width: '100%' }}
            >
              {accounts.map((a) => (
                <Select.Option key={a.id} value={a.id}>{a.name} ({a.model})</Select.Option>
              ))}
            </Select>
          </div>
          <div style={{ marginTop: 16 }}>
            <Form.Input
              label="API 类型"
              value={formData.api_type}
              onChange={(v: string) => setFormData({ ...formData, api_type: v })}
              placeholder="default"
            />
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
                <Form.Input
                  value={m.source}
                  onChange={(v: string) => handleMappingChange(i, 'source', v)}
                  placeholder="源模型"
                />
                <span style={{ color: 'var(--semi-color-text-2)' }}>→</span>
                <Form.Input
                  value={m.target}
                  onChange={(v: string) => handleMappingChange(i, 'target', v)}
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
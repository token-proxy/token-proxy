import { useState, useEffect, useCallback, type ReactNode } from 'react';
import {
  Table, Button, Tag, Space, Popconfirm, SideSheet, Form,
  Toast, Typography, Switch,
} from '@douyinfe/semi-ui';
import api from '../api.ts';

const { Title } = Typography;

interface Provider {
  id: number;
  name: string;
  openai_base_url?: string;
  anthropic_base_url?: string;
  enabled: boolean;
  models?: string[];
}

interface Account {
  id: number;
  provider_id: number;
  name: string;
  api_key: string;
  model: string;
  enabled: boolean;
}

interface ProviderFormData {
  name: string;
  openai_base_url?: string;
  anthropic_base_url?: string;
}

export default function ProviderManagement(): ReactNode {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(false);
  const [drawerVisible, setDrawerVisible] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [saving, setSaving] = useState(false);

  // Account management state
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [accountsLoading, setAccountsLoading] = useState(false);
  const [selectedProviderId, setSelectedProviderId] = useState<number | null>(null);
  const [accountFormVisible, setAccountFormVisible] = useState(false);
  const [editingAccount, setEditingAccount] = useState<Account | null>(null);
  const [accountForm, setAccountForm] = useState({ name: '', api_key: '', model: '' });
  const [accountSaving, setAccountSaving] = useState(false);

  const loadProviders = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.get<Provider[]>('/api/providers');
      setProviders(data);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '获取 Provider 列表失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadProviders();
  }, [loadProviders]);

  const openCreateDrawer = () => {
    setEditingProvider(null);
    setAccounts([]);
    setSelectedProviderId(null);
    setDrawerVisible(true);
  };

  const openEditDrawer = async (provider: Provider) => {
    setEditingProvider(provider);
    setSelectedProviderId(provider.id);
    setDrawerVisible(true);
    loadAccounts(provider.id);
  };

  const loadAccounts = async (providerId: number) => {
    setAccountsLoading(true);
    try {
      const data = await api.get<Account[]>(`/api/providers/${providerId}/accounts`);
      setAccounts(data);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '获取 Account 列表失败');
      setAccounts([]);
    } finally {
      setAccountsLoading(false);
    }
  };

  const handleSaveProvider = async (values: ProviderFormData) => {
    setSaving(true);
    try {
      if (editingProvider) {
        await api.put(`/api/providers/${editingProvider.id}`, values);
        Toast.success('Provider 已更新');
      } else {
        await api.post('/api/providers', values);
        Toast.success('Provider 已创建');
      }
      setDrawerVisible(false);
      loadProviders();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await api.delete(`/api/providers/${id}`);
      Toast.success('Provider 已删除');
      loadProviders();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '删除失败');
    }
  };

  const handleToggleEnabled = async (provider: Provider) => {
    try {
      await api.put(`/api/providers/${provider.id}`, {
        ...provider,
        enabled: !provider.enabled,
      });
      Toast.success(`Provider 已${provider.enabled ? '禁用' : '启用'}`);
      loadProviders();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '操作失败');
    }
  };

  // Account operations
  const handleOpenAccountForm = (account?: Account) => {
    if (account) {
      setEditingAccount(account);
      setAccountForm({ name: account.name, api_key: account.api_key, model: account.model });
    } else {
      setEditingAccount(null);
      setAccountForm({ name: '', api_key: '', model: '' });
    }
    setAccountFormVisible(true);
  };

  const handleSaveAccount = async () => {
    if (!selectedProviderId) return;
    setAccountSaving(true);
    try {
      const body = { ...accountForm, provider_id: selectedProviderId };
      if (editingAccount) {
        await api.put(`/api/accounts/${editingAccount.id}`, body);
        Toast.success('Account 已更新');
      } else {
        await api.post('/api/accounts', body);
        Toast.success('Account 已创建');
      }
      setAccountFormVisible(false);
      loadAccounts(selectedProviderId);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '保存 Account 失败');
    } finally {
      setAccountSaving(false);
    }
  };

  const handleDeleteAccount = async (id: number) => {
    try {
      await api.delete(`/api/accounts/${id}`);
      Toast.success('Account 已删除');
      if (selectedProviderId) loadAccounts(selectedProviderId);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '删除 Account 失败');
    }
  };

  const columns = [
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: 'OpenAI 端点', dataIndex: 'openai_base_url', key: 'openai', render: (text?: string) => text || '-' },
    { title: 'Anthropic 端点', dataIndex: 'anthropic_base_url', key: 'anthropic', render: (text?: string) => text || '-' },
    {
      title: '模型',
      dataIndex: 'models',
      key: 'models',
      render: (models?: string[]) => (
        <Space wrap>
          {models && models.length > 0
            ? models.slice(0, 5).map((m) => <Tag key={m} size="small">{m}</Tag>)
            : <span style={{ color: 'var(--semi-color-text-2)' }}>暂无</span>}
          {models && models.length > 5 && <Tag size="small">+{models.length - 5}</Tag>}
        </Space>
      ),
    },
    {
      title: '状态',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (_: boolean, record: Provider) => (
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
      render: (_: unknown, record: Provider) => (
        <Space>
          <Button size="small" onClick={() => openEditDrawer(record)}>编辑</Button>
          <Popconfirm
            title="确认删除此 Provider?"
            onConfirm={() => handleDelete(record.id)}
            position="bottomRight"
          >
            <Button size="small" type="danger">删除</Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const accountColumns = [
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: 'API Key', dataIndex: 'api_key', key: 'api_key', render: (text: string) => text ? `${text.slice(0, 8)}...` : '-' },
    { title: '模型', dataIndex: 'model', key: 'model' },
    {
      title: '操作',
      key: 'actions',
      render: (_: unknown, record: Account) => (
        <Space>
          <Button size="small" onClick={() => handleOpenAccountForm(record)}>编辑</Button>
          <Popconfirm
            title="确认删除此 Account?"
            onConfirm={() => handleDeleteAccount(record.id)}
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
        <Title heading={3}>Provider 管理</Title>
        <Button type="primary" onClick={openCreateDrawer}>创建 Provider</Button>
      </div>

      <Table
        columns={columns}
        dataSource={providers}
        loading={loading}
        rowKey="id"
        pagination={{ pageSize: 20 }}
      />

      <SideSheet
        title={editingProvider ? '编辑 Provider' : '创建 Provider'}
        visible={drawerVisible}
        onCancel={() => setDrawerVisible(false)}
        width={600}
        maskClosable={false}
      >
        <Form
          onSubmit={handleSaveProvider}
          initValues={editingProvider || undefined}
          style={{ padding: '0 4px' }}
        >
          <Form.Input
            field="name"
            label="名称"
            placeholder="Provider 名称"
            rules={[{ required: true, message: '请输入名称' }]}
          />
          <Form.Input
            field="openai_base_url"
            label="OpenAI 端点"
            placeholder="https://api.openai.com/v1"
          />
          <Form.Input
            field="anthropic_base_url"
            label="Anthropic 端点"
            placeholder="https://api.anthropic.com"
          />
          <Button type="primary" htmlType="submit" loading={saving} block style={{ marginTop: 16 }}>
            {editingProvider ? '更新' : '创建'}
          </Button>
        </Form>

        {editingProvider && (
          <div style={{ marginTop: 32, borderTop: '1px solid var(--semi-color-border)', paddingTop: 24 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
              <Title heading={6}>Account 管理</Title>
              <Button size="small" onClick={() => handleOpenAccountForm()}>添加 Account</Button>
            </div>
            <Table
              columns={accountColumns}
              dataSource={accounts}
              loading={accountsLoading}
              rowKey="id"
              size="small"
              pagination={false}
            />

            <SideSheet
              title={editingAccount ? '编辑 Account' : '添加 Account'}
              visible={accountFormVisible}
              onCancel={() => setAccountFormVisible(false)}
              width={400}
              maskClosable={false}
            >
              <div style={{ padding: '0 4px' }}>
                <Form.Input
                  label="名称"
                  value={accountForm.name}
                  onChange={(v: string) => setAccountForm({ ...accountForm, name: v })}
                  placeholder="Account 名称"
                />
                <div style={{ marginTop: 16 }}>
                  <Form.Input
                    label="API Key"
                    value={accountForm.api_key}
                    onChange={(v: string) => setAccountForm({ ...accountForm, api_key: v })}
                    placeholder="API Key"
                  />
                </div>
                <div style={{ marginTop: 16 }}>
                  <Form.Input
                    label="模型"
                    value={accountForm.model}
                    onChange={(v: string) => setAccountForm({ ...accountForm, model: v })}
                    placeholder="模型名称"
                  />
                </div>
                <Button
                  type="primary"
                  onClick={handleSaveAccount}
                  loading={accountSaving}
                  block
                  style={{ marginTop: 16 }}
                >
                  {editingAccount ? '更新' : '添加'}
                </Button>
              </div>
            </SideSheet>
          </div>
        )}
      </SideSheet>
    </div>
  );
}
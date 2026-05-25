import { useState, useEffect, useCallback, useRef, type ReactNode } from 'react';
import {
  Table, Button, Tag, Space, Popconfirm, SideSheet, Form,
  Toast, Typography, TagInput, Input, Tooltip, Select,
} from '@douyinfe/semi-ui';
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form';
import { IconEyeOpened, IconEyeClosedSolid } from '@douyinfe/semi-icons';
import api from '../api.ts';
import StatusToggle from '../components/StatusToggle.tsx';

const { Title, Text } = Typography;

interface Provider {
  id: string;
  name: string;
  openai_base_url?: string;
  anthropic_base_url?: string;
  default_model?: string;
  status: string;
  models?: string[];
  account_count?: number;
}

interface Account {
  id: string;
  provider_id: string;
  name: string;
  api_key_suffix: string;
  status: string;
  created_at: string;
  updated_at: string;
}

interface ProviderFormData {
  name: string;
  openai_base_url?: string;
  anthropic_base_url?: string;
  default_model?: string;
}

export default function ProviderManagement(): ReactNode {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(false);
  const [drawerVisible, setDrawerVisible] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [saving, setSaving] = useState(false);
  const [operatingIds, setOperatingIds] = useState<string[]>([]);
  const formRef = useRef<FormApi>(null);
  const operatingIdsRef = useRef<Set<string>>(new Set());

  // 模型管理
  const [providerModels, setProviderModels] = useState<string[]>([]);
  const [providerDefaultModel, setProviderDefaultModel] = useState<string | undefined>();
  const [discovering, setDiscovering] = useState(false);

  // Account 管理
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [accountsLoading, setAccountsLoading] = useState(false);
  const [selectedProviderId, setSelectedProviderId] = useState<string | null>(null);
  const [accountFormVisible, setAccountFormVisible] = useState(false);
  const [editingAccount, setEditingAccount] = useState<Account | null>(null);
  const [accountForm, setAccountForm] = useState({ name: '', api_key: '' });
  const [accountSaving, setAccountSaving] = useState(false);
  const [apiKeyVisible, setApiKeyVisible] = useState(false);

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
    setProviderModels([]);
    setProviderDefaultModel(undefined);
    setDrawerVisible(true);
  };

  const openEditDrawer = (provider: Provider) => {
    setEditingProvider(provider);
    setSelectedProviderId(provider.id);
    setProviderModels(provider.models ?? []);
    setProviderDefaultModel(provider.default_model);
    setDrawerVisible(true);
    loadAccounts(provider.id);
  };

  const loadAccounts = async (providerId: string) => {
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
      const defaultModel = providerDefaultModel && providerModels.includes(providerDefaultModel)
        ? providerDefaultModel
        : '';
      if (editingProvider) {
        // 编辑态：将基础字段 + 模型列表合并为一次 PUT
        await api.put(`/api/providers/${editingProvider.id}`, {
          ...values,
          default_model: defaultModel,
          models: providerModels,
        });
        Toast.success('Provider 已更新');
      } else {
        await api.post('/api/providers', { ...values, default_model: defaultModel });
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

  const handleDelete = async (id: string) => {
    const operationKey = `provider:${id}`;
    if (operatingIdsRef.current.has(operationKey)) return;
    setOperation(operationKey, true);
    try {
      await api.delete(`/api/providers/${id}`);
      Toast.success('Provider 已删除');
      loadProviders();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '删除失败');
    } finally {
      setOperation(operationKey, false);
    }
  };

  const handleToggleEnabled = async (provider: Provider) => {
    const operationKey = `provider:${provider.id}`;
    if (operatingIdsRef.current.has(operationKey)) return;
    setOperation(operationKey, true);
    const nextStatus = provider.status === 'enabled' ? 'disabled' : 'enabled';
    try {
      await api.put(`/api/providers/${provider.id}`, { status: nextStatus });
      Toast.success(`Provider 已${nextStatus === 'enabled' ? '启用' : '禁用'}`);
      loadProviders();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '操作失败');
    } finally {
      setOperation(operationKey, false);
    }
  };

  // 模型列表管理
  const handleDiscoverModels = async () => {
    if (!editingProvider) return;
    setDiscovering(true);
    try {
      const resp = await api.post<{ models: string[] }>(
        `/api/providers/${editingProvider.id}/discover-models`,
        {},
      );
      const models = resp.models ?? [];
      setProviderModels(models);
      if (providerDefaultModel && !models.includes(providerDefaultModel)) {
        setProviderDefaultModel(undefined);
      }
      Toast.success(`自动发现完成，共 ${models.length} 个模型`);
      loadProviders();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '自动发现模型失败');
    } finally {
      setDiscovering(false);
    }
  };

  const handleProviderModelsChange = (models: string[]) => {
    setProviderModels(models);
    if (providerDefaultModel && !models.includes(providerDefaultModel)) {
      setProviderDefaultModel(undefined);
    }
  };

  // Account 操作
  const handleOpenAccountForm = (account?: Account) => {
    setApiKeyVisible(false);
    if (account) {
      setEditingAccount(account);
      setAccountForm({ name: account.name, api_key: '' });
    } else {
      setEditingAccount(null);
      setAccountForm({ name: '', api_key: '' });
    }
    setAccountFormVisible(true);
  };

  const handleSaveAccount = async () => {
    if (!selectedProviderId) return;
    if (!editingAccount && !accountForm.api_key.trim()) {
      Toast.error('请填写 API Key');
      return;
    }
    setAccountSaving(true);
    try {
      if (editingAccount) {
        // 更新时 api_key 为空则不传，避免覆盖
        const body: Record<string, string> = { name: accountForm.name };
        if (accountForm.api_key.trim()) body.api_key = accountForm.api_key.trim();
        await api.put(
          `/api/providers/${selectedProviderId}/accounts/${editingAccount.id}`,
          body,
        );
        Toast.success('Account 已更新');
      } else {
        await api.post(
          `/api/providers/${selectedProviderId}/accounts`,
          {
            name: accountForm.name.trim() || undefined,
            api_key: accountForm.api_key.trim(),
          },
        );
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

  const handleDeleteAccount = async (id: string) => {
    if (!selectedProviderId) return;
    const operationKey = `account:${id}`;
    if (operatingIdsRef.current.has(operationKey)) return;
    setOperation(operationKey, true);
    try {
      await api.delete(`/api/providers/${selectedProviderId}/accounts/${id}`);
      Toast.success('Account 已删除');
      loadAccounts(selectedProviderId);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '删除 Account 失败');
    } finally {
      setOperation(operationKey, false);
    }
  };

  const columns = [
    { title: '名称', dataIndex: 'name', key: 'name', width: 140 },
    { title: 'OpenAI 端点', dataIndex: 'openai_base_url', key: 'openai', width: 200, render: (text?: string) => text || '-' },
    { title: 'Anthropic 端点', dataIndex: 'anthropic_base_url', key: 'anthropic', width: 200, render: (text?: string) => text || '-' },
    {
      title: '默认模型',
      dataIndex: 'default_model',
      key: 'default_model',
      width: 180,
      render: (text?: string) => text ? <Tag color="blue" size="small">{text}</Tag> : '-',
    },
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
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (_: string, record: Provider) => (
        <StatusToggle
          enabled={record.status === 'enabled'}
          loading={operatingIds.includes(`provider:${record.id}`)}
          onToggle={() => handleToggleEnabled(record)}
        />
      ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 160,
      render: (_: unknown, record: Provider) => (
        <Space>
          <Button size="small" onClick={() => openEditDrawer(record)}>编辑</Button>
          <Popconfirm
            title="确认删除此 Provider?"
            onConfirm={() => handleDelete(record.id)}
            position="bottomRight"
          >
            <Button size="small" type="danger" loading={operatingIds.includes(`provider:${record.id}`)}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const accountColumns = [
    { title: '名称', dataIndex: 'name', key: 'name' },
    {
      title: 'API Key',
      dataIndex: 'api_key_suffix',
      key: 'api_key_suffix',
      width: 140,
      render: (suffix: string) => suffix ? `******${suffix}` : '-',
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 80,
      render: (status: string) => (
        <Tag color={status === 'enabled' ? 'green' : 'red'} size="small">
          {status === 'enabled' ? '启用' : '禁用'}
        </Tag>
      ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 160,
      render: (_: unknown, record: Account) => (
        <Space>
          <Button size="small" onClick={() => handleOpenAccountForm(record)}>编辑</Button>
          <Popconfirm
            title="确认删除此 Account?"
            onConfirm={() => handleDeleteAccount(record.id)}
            position="bottomRight"
          >
            <Button size="small" type="danger" loading={operatingIds.includes(`account:${record.id}`)}>
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
        <Title heading={3}>Provider 管理</Title>
        <Button type="primary" onClick={openCreateDrawer}>创建 Provider</Button>
      </div>

      <Table
        columns={columns}
        dataSource={providers}
        loading={loading}
        rowKey="id"
        scroll={{ x: 'max-content' }}
        pagination={{ pageSize: 20 }}
      />

      <SideSheet
        title={editingProvider ? '编辑 Provider' : '创建 Provider'}
        visible={drawerVisible}
        onCancel={() => setDrawerVisible(false)}
        size="large"
        maskClosable
      >
        <div style={{ padding: '0 4px' }}>
        <Form
          onSubmit={handleSaveProvider}
          initValues={editingProvider || undefined}
          getFormApi={(api) => { formRef.current = api; }}
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
        </Form>


        {editingProvider && (
          <>
            <div style={{ marginTop: 32, borderTop: '1px solid var(--semi-color-border)', paddingTop: 24 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
                <Title heading={6}>模型列表</Title>
                {accounts.length === 0 ? (
                  <Tooltip content="缺少可用的 API Key，请先在下方添加 Account">
                    {/* Tooltip 不能直接包裹 disabled Button，需用 span 中转 */}
                    <span style={{ display: 'inline-block' }}>
                      <Button size="small" disabled>
                        自动发现
                      </Button>
                    </span>
                  </Tooltip>
                ) : (
                  <Button
                    size="small"
                    onClick={handleDiscoverModels}
                    loading={discovering}
                  >
                    自动发现
                  </Button>
                )}
              </div>
              <TagInput
                value={providerModels}
                onChange={(v) => handleProviderModelsChange(v as string[])}
                placeholder="输入模型名后回车，或点击自动发现"
                allowDuplicates={false}
                style={{ width: '100%' }}
              />
              <Text type="tertiary" size="small" style={{ marginTop: 6, display: 'block' }}>
                {accounts.length === 0
                  ? '缺少可用的 API Key，请先添加一个 Account 后再使用自动发现；模型列表会随底部提交按钮一并提交'
                  : `将使用 Account "${accounts[0].name}" 的 API Key 调用上游 /v1/models 接口；模型列表会随底部提交按钮一并提交`}
              </Text>
              <div style={{ marginTop: 16 }}>
                <Text strong size="small" style={{ marginBottom: 8, display: 'block' }}>默认模型</Text>
                <Select
                  value={providerDefaultModel}
                  placeholder={providerModels.length === 0 ? '模型列表为空，无法选择默认模型' : '从已有模型中选择'}
                  showClear
                  disabled={providerModels.length === 0}
                  optionList={providerModels.map((m) => ({ value: m, label: m }))}
                  onChange={(value) => setProviderDefaultModel(value as string | undefined)}
                  style={{ width: '100%' }}
                />
                <Text type="tertiary" size="small" style={{ marginTop: 6, display: 'block' }}>
                  默认模型必须来自模型列表；从模型列表移除当前默认模型时会立即清空。
                </Text>
              </div>
            </div>

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
                scroll={{ x: 'max-content' }}
                pagination={false}
              />

              <SideSheet
                title={editingAccount ? '编辑 Account' : '添加 Account'}
                visible={accountFormVisible}
                onCancel={() => setAccountFormVisible(false)}
                width={560}
                maskClosable
              >
                <div style={{ padding: '0 4px' }}>
                    <div>
                      <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>名称</div>
                      <Input
                        value={accountForm.name}
                        onChange={(v: string) => setAccountForm({ ...accountForm, name: v })}
                        placeholder="留空将自动以 API Key 后缀生成"
                        autoComplete="off"
                      />
                    </div>
                    <div style={{ marginTop: 16 }}>
                      <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>
                        {editingAccount ? 'API Key (留空表示不修改)' : 'API Key'}
                      </div>
                      {/*
                        故意不使用 type="password"，避免触发浏览器密码管理器的"保存账号密码"弹窗。
                        改为普通 text input + CSS 视觉遮罩（webkit-text-security）+ 自定义眼睛按钮。
                      */}
                      <Input
                        value={accountForm.api_key}
                        onChange={(v: string) => setAccountForm({ ...accountForm, api_key: v })}
                        placeholder={editingAccount ? '仅在需要替换时填写' : '上游 API Key'}
                        autoComplete="off"
                        data-1p-ignore="true"
                        data-lpignore="true"
                        spellCheck={false}
                        style={!apiKeyVisible && accountForm.api_key
                          ? ({
                              WebkitTextSecurity: 'disc',
                              textSecurity: 'disc',
                              fontFamily: 'text-security-disc, monospace',
                            } as React.CSSProperties)
                          : undefined}
                        suffix={
                          <Button
                            theme="borderless"
                            icon={apiKeyVisible ? <IconEyeClosedSolid /> : <IconEyeOpened />}
                            size="small"
                            onClick={() => setApiKeyVisible(!apiKeyVisible)}
                            aria-label={apiKeyVisible ? '隐藏' : '显示'}
                          />
                        }
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

          </>
        )}

        <Button
          type="primary"
          loading={saving}
          onClick={() => formRef.current?.submitForm()}
          block
          style={{ marginTop: 24 }}
        >
          {editingProvider ? '更新' : '创建'}
        </Button>
        </div>
      </SideSheet>
    </div>
  );
}

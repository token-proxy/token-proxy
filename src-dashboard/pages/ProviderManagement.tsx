import { type ReactNode, useRef, useState } from 'react';
import { useFetch } from '../hooks/useFetch.ts';
import {
  Button,
  Collapse,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Radio,
  RadioGroup,
  Select,
  SideSheet,
  Space,
  Table,
  Tag,
  TagInput,
  Toast,
  Tooltip,
  Typography,
} from '@douyinfe/semi-ui';
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form';
import api from '../api.ts';
import AccountManager, { type Account } from '@components/provider/AccountManager';
import AutoColoredTag from '@components/common/AutoColoredTag';
import type { FaultConfigJson } from '../types/accessPoint.ts';

const { Title, Text } = Typography;

// ── 类型定义 ──

/** 服务商信息 */
interface Provider {
  id: string;
  name: string;
  openai_base_url?: string;
  anthropic_base_url?: string;
  status: string;
  models?: string[];
  account_count?: number;
  available_account_count?: number;
  rate_limit_config?: FaultConfigJson;
  balance_exhausted_config?: FaultConfigJson;
}

/** 服务商表单数据 */
interface ProviderFormData {
  name: string;
  openai_base_url?: string;
  anthropic_base_url?: string;
}

/** 故障配置的本地状态表示 */
interface FaultConfigState {
  status_codes: string[];
  recover_type: 'manual' | 'scheduled' | 'extract';
  // scheduled
  delay_value: number;
  delay_unit: 'seconds' | 'minutes' | 'hours' | 'days';
  // extract
  extract_source: 'header' | 'body';
  extract_source_path: string;
  extract_regex_pattern: string;
  extract_kind: 'duration' | 'timestamp';
  extract_duration_unit: 'seconds' | 'minutes' | 'hours' | 'days';
  extract_on_failed: '' | 'fallback_scheduled' | 'fallback_manual';
  // fallback_scheduled 的延迟配置
  extract_fallback_delay_value: number;
  extract_fallback_delay_unit: 'seconds' | 'minutes' | 'hours' | 'days';
}

// ── 默认值 ──

/** 限流故障配置默认值 */

const DEFAULT_RATE_LIMIT_CONFIG: FaultConfigState = {
  status_codes: ['429'],
  recover_type: 'manual',
  delay_value: 60,
  delay_unit: 'seconds',
  extract_source: 'header',
  extract_source_path: '',
  extract_regex_pattern: '(\\d+)',
  extract_kind: 'duration',
  extract_duration_unit: 'seconds',
  extract_on_failed: '',
  extract_fallback_delay_value: 60,
  extract_fallback_delay_unit: 'seconds',
};

/** 余额耗尽故障配置默认值 */
const DEFAULT_BALANCE_CONFIG: FaultConfigState = {
  ...DEFAULT_RATE_LIMIT_CONFIG,
  status_codes: ['402'],
};

// ── 序列化/反序列化 ──

/** 将 FaultConfigState 序列化为 FaultConfigJson（与后端 Rust 结构对应） */
function buildFaultConfig(state: FaultConfigState): FaultConfigJson | undefined {
  if (!state.status_codes.length) return undefined;

  const result: FaultConfigJson = {
    status_codes: state.status_codes,
    type: state.recover_type,
  };

  if (state.recover_type === 'scheduled') {
    result.delay = { value: state.delay_value, unit: state.delay_unit };
  } else if (state.recover_type === 'extract') {
    result.config = {
      source: state.extract_source,
      source_path: state.extract_source_path,
      regex_pattern: state.extract_regex_pattern,
      kind:
        state.extract_kind === 'duration'
          ? { type: 'duration', unit: state.extract_duration_unit }
          : { type: 'timestamp' },
    };
    if (state.extract_on_failed === 'fallback_scheduled') {
      result.config.on_extract_failed = {
        type: 'fallback_scheduled',
        delay: {
          value: state.extract_fallback_delay_value,
          unit: state.extract_fallback_delay_unit,
        },
      };
    } else if (state.extract_on_failed === 'fallback_manual') {
      result.config.on_extract_failed = { type: 'fallback_manual' };
    }
  }

  return result;
}

/** 将后端返回的 FaultConfigJson 反序列化为本地 FaultConfigState */
function parseFaultConfig(
  json: FaultConfigJson | undefined,
  defaults: FaultConfigState,
): FaultConfigState {
  if (!json) return { ...defaults };
  return {
    status_codes: json.status_codes ?? defaults.status_codes,
    recover_type: (json.type as FaultConfigState['recover_type']) ?? defaults.recover_type,
    delay_value: json.delay?.value ?? defaults.delay_value,
    delay_unit: (json.delay?.unit as FaultConfigState['delay_unit']) ?? defaults.delay_unit,
    extract_source:
      (json.config?.source as FaultConfigState['extract_source']) ?? defaults.extract_source,
    extract_source_path: json.config?.source_path ?? defaults.extract_source_path,
    extract_regex_pattern: json.config?.regex_pattern ?? defaults.extract_regex_pattern,
    extract_kind:
      (json.config?.kind?.type as FaultConfigState['extract_kind']) ?? defaults.extract_kind,
    extract_duration_unit:
      (json.config?.kind?.unit as FaultConfigState['extract_duration_unit']) ??
      defaults.extract_duration_unit,
    extract_on_failed:
      (json.config?.on_extract_failed?.type as FaultConfigState['extract_on_failed']) ?? '',
    extract_fallback_delay_value:
      json.config?.on_extract_failed?.delay?.value ?? defaults.extract_fallback_delay_value,
    extract_fallback_delay_unit:
      (json.config?.on_extract_failed?.delay
        ?.unit as FaultConfigState['extract_fallback_delay_unit']) ??
      defaults.extract_fallback_delay_unit,
  };
}

// ── 子组件：单板块编辑器 ──

/** 故障配置编辑子组件，支持手动/定时/提取三种恢复方式 */
function FaultConfigEditor({
  state,
  onChange,
  allowedRecoverTypes,
}: {
  state: FaultConfigState;
  onChange: (s: FaultConfigState) => void;
  allowedRecoverTypes?: FaultConfigState['recover_type'][];
}): ReactNode {
  const update = (patch: Partial<FaultConfigState>) => onChange({ ...state, ...patch });

  const recoverOptions = (
    [
      { value: 'manual' as const, label: '手动恢复 (manual)' },
      { value: 'scheduled' as const, label: '定时恢复 (scheduled)' },
      { value: 'extract' as const, label: '从响应提取恢复时间 (extract)' },
    ] as { value: FaultConfigState['recover_type']; label: string }[]
  ).filter((opt) => !allowedRecoverTypes || allowedRecoverTypes.includes(opt.value));

  return (
    <div>
      <div style={{ marginBottom: 12 }}>
        <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 13 }}>
          触发状态码
        </div>
        <TagInput
          value={state.status_codes}
          onChange={(v) => update({ status_codes: v as string[] })}
          placeholder="输入状态码后回车"
        />
      </div>

      <div style={{ marginBottom: 12 }}>
        <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 13 }}>
          恢复方式
        </div>
        <Select
          value={state.recover_type}
          onChange={(v) => update({ recover_type: v as FaultConfigState['recover_type'] })}
          style={{ width: '100%' }}
        >
          {recoverOptions.map((opt) => (
            <Select.Option key={opt.value} value={opt.value}>
              {opt.label}
            </Select.Option>
          ))}
        </Select>
      </div>

      {state.recover_type === 'scheduled' && (
        <div style={{ marginBottom: 12 }}>
          <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 13 }}>
            延迟时长
          </div>
          <Space>
            <InputNumber
              value={state.delay_value}
              onChange={(v) => update({ delay_value: (v as number) || 60 })}
              min={1}
              style={{ width: 120 }}
            />
            <Select
              value={state.delay_unit}
              onChange={(v) => update({ delay_unit: v as FaultConfigState['delay_unit'] })}
              style={{ width: 100 }}
            >
              <Select.Option value="seconds">秒</Select.Option>
              <Select.Option value="minutes">分</Select.Option>
              <Select.Option value="hours">时</Select.Option>
              <Select.Option value="days">天</Select.Option>
            </Select>
          </Space>
        </div>
      )}

      {state.recover_type === 'extract' && (
        <>
          <div style={{ marginBottom: 12 }}>
            <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 13 }}>
              提取来源
            </div>
            <Space>
              <Select
                value={state.extract_source}
                onChange={(v) =>
                  update({ extract_source: v as FaultConfigState['extract_source'] })
                }
                style={{ width: 110 }}
              >
                <Select.Option value="header">响应头</Select.Option>
                <Select.Option value="body">响应体</Select.Option>
              </Select>
              <Input
                value={state.extract_source_path}
                onChange={(v) => update({ extract_source_path: v })}
                placeholder={
                  state.extract_source === 'header' ? '如 Retry-After' : '如 $.error.reset_time'
                }
                style={{ flex: 1 }}
              />
            </Space>
          </div>

          <div style={{ marginBottom: 12 }}>
            <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 13 }}>
              正则表达式
            </div>
            <Input
              value={state.extract_regex_pattern}
              onChange={(v) => update({ extract_regex_pattern: v })}
              placeholder="含一个捕获组，如 (\\d+)"
            />
          </div>

          <div style={{ marginBottom: 12 }}>
            <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 13 }}>
              结果语义
            </div>
            <RadioGroup
              value={state.extract_kind}
              onChange={(e) =>
                update({ extract_kind: e.target.value as FaultConfigState['extract_kind'] })
              }
            >
              <Radio value="duration">时间间隔</Radio>
              <Radio value="timestamp">时刻</Radio>
            </RadioGroup>
          </div>

          {state.extract_kind === 'duration' && (
            <div style={{ marginBottom: 12 }}>
              <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 13 }}>
                时间单位
              </div>
              <Select
                value={state.extract_duration_unit}
                onChange={(v) =>
                  update({
                    extract_duration_unit: v as FaultConfigState['extract_duration_unit'],
                  })
                }
                style={{ width: '100%' }}
              >
                <Select.Option value="seconds">秒</Select.Option>
                <Select.Option value="minutes">分</Select.Option>
                <Select.Option value="hours">时</Select.Option>
                <Select.Option value="days">天</Select.Option>
              </Select>
            </div>
          )}

          <div style={{ marginBottom: 12 }}>
            <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 13 }}>
              提取失败处理
            </div>
            <Select
              value={state.extract_on_failed}
              onChange={(v) =>
                update({ extract_on_failed: v as FaultConfigState['extract_on_failed'] })
              }
              style={{ width: '100%' }}
            >
              <Select.Option value="fallback_scheduled">降级为定时恢复</Select.Option>
              <Select.Option value="fallback_manual">降级为手动恢复</Select.Option>
            </Select>
          </div>

          {state.extract_on_failed === 'fallback_scheduled' && (
            <div style={{ marginBottom: 12 }}>
              <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 13 }}>
                降级延迟时长
              </div>
              <Space>
                <InputNumber
                  value={state.extract_fallback_delay_value}
                  onChange={(v) => update({ extract_fallback_delay_value: (v as number) || 60 })}
                  min={1}
                  style={{ width: 120 }}
                />
                <Select
                  value={state.extract_fallback_delay_unit}
                  onChange={(v) =>
                    update({
                      extract_fallback_delay_unit:
                        v as FaultConfigState['extract_fallback_delay_unit'],
                    })
                  }
                  style={{ width: 100 }}
                >
                  <Select.Option value="seconds">秒</Select.Option>
                  <Select.Option value="minutes">分</Select.Option>
                  <Select.Option value="hours">时</Select.Option>
                  <Select.Option value="days">天</Select.Option>
                </Select>
              </Space>
            </div>
          )}
        </>
      )}
    </div>
  );
}

// ── 主组件 ──

/**
 * ProviderManagement - 服务商管理页面
 *
 * 服务商的增删改查、模型管理（自动发现 + 手动编辑）、账号管理、账户异常处置配置。
 */
export default function ProviderManagement(): ReactNode {
  const {
    data: providers,
    loading,
    refetch: loadProviders,
  } = useFetch(() => api.get<Provider[]>('/api/providers'), []);
  const providersList = providers ?? [];
  const [drawerVisible, setDrawerVisible] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [saving, setSaving] = useState(false);
  const [operatingIds, setOperatingIds] = useState<string[]>([]);
  const formRef = useRef<FormApi>(null);
  const operatingIdsRef = useRef<Set<string>>(new Set());

  // 模型管理
  const [providerModels, setProviderModels] = useState<string[]>([]);
  const [discovering, setDiscovering] = useState(false);

  // 账号管理
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [accountsLoading, setAccountsLoading] = useState(false);
  const [selectedProviderId, setSelectedProviderId] = useState<string | null>(null);

  // 账户异常处置
  const [rateLimitConfig, setRateLimitConfig] =
    useState<FaultConfigState>(DEFAULT_RATE_LIMIT_CONFIG);
  const [balanceConfig, setBalanceConfig] = useState<FaultConfigState>(DEFAULT_BALANCE_CONFIG);

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

  const openCreateDrawer = () => {
    setEditingProvider(null);
    setAccounts([]);
    setSelectedProviderId(null);
    setProviderModels([]);
    setRateLimitConfig({ ...DEFAULT_RATE_LIMIT_CONFIG });
    setBalanceConfig({ ...DEFAULT_BALANCE_CONFIG });
    setDrawerVisible(true);
  };

  const openEditDrawer = async (provider: Provider) => {
    setEditingProvider(provider);
    setSelectedProviderId(provider.id);
    setProviderModels(provider.models ?? []);
    setRateLimitConfig(parseFaultConfig(provider.rate_limit_config, DEFAULT_RATE_LIMIT_CONFIG));
    setBalanceConfig(parseFaultConfig(provider.balance_exhausted_config, DEFAULT_BALANCE_CONFIG));
    setDrawerVisible(true);
    loadAccounts(provider.id);
  };

  const loadAccounts = async (providerId: string) => {
    setAccountsLoading(true);
    try {
      const data = await api.get<Account[]>(`/api/providers/${providerId}/accounts`);
      setAccounts(data);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '获取账号列表失败');
      setAccounts([]);
    } finally {
      setAccountsLoading(false);
    }
  };

  const handleSaveProvider = async (values: ProviderFormData) => {
    setSaving(true);
    try {
      const rateLimit = buildFaultConfig(rateLimitConfig);
      const balance = buildFaultConfig(balanceConfig);

      if (editingProvider) {
        const body: Record<string, unknown> = {
          ...values,
          models: providerModels,
          rate_limit_config: rateLimit ?? null,
          balance_exhausted_config: balance ?? null,
        };
        await api.put(`/api/providers/${editingProvider.id}`, body);
        Toast.success('服务商已更新');
      } else {
        await api.post('/api/providers', {
          ...values,
          rate_limit_config: rateLimit ?? null,
          balance_exhausted_config: balance ?? null,
        });
        Toast.success('服务商已创建');
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
    const operationKey = `delete:${id}`;
    if (operatingIdsRef.current.has(operationKey)) return;
    setOperation(operationKey, true);
    try {
      await api.delete(`/api/providers/${id}`);
      Toast.success('服务商已删除');
      loadProviders();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '删除失败');
    } finally {
      setOperation(operationKey, false);
    }
  };

  const handleToggleEnabled = async (provider: Provider) => {
    const operationKey = `toggle:${provider.id}`;
    if (operatingIdsRef.current.has(operationKey)) return;
    setOperation(operationKey, true);
    const nextStatus = provider.status === 'enabled' ? 'disabled' : 'enabled';
    try {
      await api.put(`/api/providers/${provider.id}`, { status: nextStatus });
      Toast.success(`服务商已${nextStatus === 'enabled' ? '启用' : '禁用'}`);
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
        `/api/providers/${editingProvider.id}/models:discover`,
        {},
      );
      const models = resp.models ?? [];
      setProviderModels(models);
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
  };

  const columns = [
    { title: '名称', dataIndex: 'name', key: 'name', width: 140 },
    {
      title: '端点',
      key: 'endpoint',
      width: 160,
      render: (_: unknown, record: Provider) => {
        const tags: ReactNode[] = [];
        if (record.openai_base_url) {
          tags.push(
            <AutoColoredTag key="openai" size="small">
              OpenAI
            </AutoColoredTag>,
          );
        }
        if (record.anthropic_base_url) {
          tags.push(
            <AutoColoredTag key="anthropic" size="small">
              Anthropic
            </AutoColoredTag>,
          );
        }
        return tags.length > 0 ? (
          <Space>{tags}</Space>
        ) : (
          <span style={{ color: 'var(--semi-color-text-2)' }}>-</span>
        );
      },
    },
    {
      title: '可用账号',
      key: 'available_accounts',
      width: 120,
      render: (_: unknown, record: Provider) => {
        const available = record.available_account_count ?? 0;
        const total = record.account_count ?? 0;
        let color: 'red' | 'green' | 'yellow' = 'yellow';
        if (total === 0) color = 'red';
        else if (available === total) color = 'green';
        else if (available === 0) color = 'red';
        return (
          <Tag color={color} size="small">
            {available}/{total}
          </Tag>
        );
      },
    },
    {
      title: '模型',
      dataIndex: 'models',
      key: 'models',
      width: 100,
      render: (models?: string[]) => (
        <Space wrap>
          {models && models.length > 0 ? (
            models.slice(0, 5).map((m) => (
              <Tag key={m} size="small">
                {m}
              </Tag>
            ))
          ) : (
            <span style={{ color: 'var(--semi-color-text-2)' }}>暂无</span>
          )}
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
        <Tag color={record.status === 'enabled' ? 'green' : 'grey'} size="small">
          {record.status === 'enabled' ? '已启用' : '已禁用'}
        </Tag>
      ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 220,
      render: (_: unknown, record: Provider) => (
        <Space>
          <Button size="small" onClick={() => openEditDrawer(record)}>
            编辑
          </Button>
          <Button
            size="small"
            type="danger"
            loading={operatingIds.includes(`toggle:${record.id}`)}
            onClick={() => handleToggleEnabled(record)}
          >
            {record.status === 'enabled' ? '禁用' : '启用'}
          </Button>
          <Popconfirm
            title="确认删除此服务商?"
            onConfirm={() => handleDelete(record.id)}
            position="bottomRight"
          >
            <Button
              size="small"
              type="danger"
              loading={operatingIds.includes(`delete:${record.id}`)}
            >
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

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
        <Title heading={3}>服务商管理</Title>
        <Space>
          <Button onClick={loadProviders} loading={loading}>
            刷新
          </Button>
          <Button type="primary" onClick={openCreateDrawer}>
            创建服务商
          </Button>
        </Space>
      </div>

      <Table
        columns={columns}
        dataSource={providersList}
        loading={loading}
        rowKey="id"
        scroll={{ x: 'max-content' }}
        pagination={{ pageSize: 20 }}
      />

      <SideSheet
        title={editingProvider ? '编辑服务商' : '创建服务商'}
        visible={drawerVisible}
        onCancel={() => {
          setDrawerVisible(false);
          loadProviders();
        }}
        size="large"
        maskClosable
      >
        <div style={{ padding: '0 4px' }}>
          <Form
            onSubmit={handleSaveProvider}
            initValues={editingProvider || undefined}
            getFormApi={(api) => {
              formRef.current = api;
            }}
          >
            <Form.Input
              field="name"
              label="名称"
              placeholder="服务商名称"
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

          {/* 账户异常处置 — 始终可见，创建和编辑时均有默认值 */}
          <div style={{ marginTop: 20 }}>
            <div
              style={{
                marginBottom: 12,
                color: 'var(--semi-color-text-0)',
                fontWeight: 500,
                fontSize: 14,
              }}
            >
              账户异常处置
            </div>
            <Collapse>
              <Collapse.Panel header="配额耗尽" itemKey="rate_limit">
                <FaultConfigEditor state={rateLimitConfig} onChange={setRateLimitConfig} />
              </Collapse.Panel>
              <Collapse.Panel header="余额耗尽" itemKey="balance_exhausted">
                <FaultConfigEditor
                  state={balanceConfig}
                  onChange={setBalanceConfig}
                  allowedRecoverTypes={['manual']}
                />
              </Collapse.Panel>
            </Collapse>
          </div>

          {editingProvider && (
            <>
              <div
                style={{
                  marginTop: 32,
                  borderTop: '1px solid var(--semi-color-border)',
                  paddingTop: 24,
                }}
              >
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    marginBottom: 12,
                  }}
                >
                  <Title heading={6}>模型列表</Title>
                  {accounts.length === 0 ? (
                    <Tooltip content="缺少可用的 API Key，请先在下方添加账号">
                      <span style={{ display: 'inline-block' }}>
                        <Button size="small" disabled>
                          自动发现
                        </Button>
                      </span>
                    </Tooltip>
                  ) : (
                    <Button size="small" onClick={handleDiscoverModels} loading={discovering}>
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
                  renderTagItem={(value, _index, onClose) => (
                    <Tag
                      key={value as string}
                      color="blue"
                      size="small"
                      closable
                      onClose={onClose}
                      style={{ marginRight: 4 }}
                    >
                      {value as string}
                    </Tag>
                  )}
                />
                <Text type="tertiary" size="small" style={{ marginTop: 6, display: 'block' }}>
                  {accounts.length === 0
                    ? '缺少可用的 API Key，请先添加一个账号后再使用自动发现；模型列表会随底部提交按钮一并提交'
                    : `将使用账号 "${accounts[0]?.name}" 的 API Key 调用上游 /v1/models 接口；模型列表会随底部提交按钮一并提交`}
                </Text>
              </div>

              <div
                style={{
                  marginTop: 32,
                  borderTop: '1px solid var(--semi-color-border)',
                  paddingTop: 24,
                }}
              >
                <AccountManager
                  providerId={selectedProviderId!}
                  accounts={accounts}
                  loading={accountsLoading}
                  onAccountsChanged={() => selectedProviderId && loadAccounts(selectedProviderId)}
                />
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

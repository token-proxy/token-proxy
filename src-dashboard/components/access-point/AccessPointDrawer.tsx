import {
  Button,
  Input,
  InputNumber,
  Select,
  SideSheet,
  Table,
  Tag,
  Tooltip,
} from '@douyinfe/semi-ui';
import { type ReactNode, useState, useRef, useMemo, useEffect, useCallback } from 'react';
import type {
  AccessPoint,
  AccessPointFormData,
  AccountEntry,
  ModelRoutingRow,
  ProviderOption,
  AccountOption,
} from '../../types/accessPoint.ts';
import { UNMATCHED_MODEL } from '../../types/accessPoint.ts';
import api from '../../api.ts';

/** AccessPointDrawer 组件 Props */
interface AccessPointDrawerProps {
  visible: boolean;
  editingAccessPoint: AccessPoint | null;
  saving: boolean;
  formData: AccessPointFormData;
  providers: ProviderOption[];
  accounts: AccountOption[];
  accountsLoading: boolean;
  onClose: () => void;
  onFormChange: (formData: AccessPointFormData) => void;
  onSave: () => void;
}

// ---- helpers ----

/** 根据 disabled_reason 返回中文标签 */
function disabledReasonLabel(reason: string): string {
  switch (reason) {
    case 'rate_limited':
      return '配额耗尽';
    case 'balance_exhausted':
      return '余额耗尽';
    case 'fault':
      return '故障';
    case 'manual':
      return '手动';
    default:
      return reason;
  }
}

function providerIdsFromAccounts(
  accounts: AccountEntry[],
  accountOptions: AccountOption[],
): string[] {
  const ids = new Set<string>();
  for (const b of accounts) {
    const acct = accountOptions.find((a) => a.id === b.account_id);
    if (acct) ids.add(acct.provider_id);
  }
  return [...ids];
}

// ---- 账号行编辑器（权重 + 优先级共用） ----

/** 单条账号行的编辑组件，包含服务商选择、账号选择、权重/优先级输入 */
function AccountRowEditor({
  accountEntry,
  index,
  rowKey,
  selectedProvider,
  providerAccounts,
  accountsLoading,
  usedAccountIds,
  providers,
  onProviderChange,
  onAccountChange,
  onFieldChange,
  onRemove,
  isWeighted,
  dragHandle,
}: {
  accountEntry: AccountEntry;
  index: number;
  rowKey: string;
  selectedProvider: string | null;
  providerAccounts: AccountOption[];
  accountsLoading: boolean;
  usedAccountIds: Set<string>;
  providers: ProviderOption[];
  onProviderChange: (index: number, providerId: string) => void;
  onAccountChange: (index: number, accountId: string) => void;
  onFieldChange: (index: number, field: 'weight' | 'priority', value: number | null) => void;
  onRemove: (rowKey: string) => void;
  isWeighted: boolean;
  dragHandle?: ReactNode;
}): ReactNode {
  const availAccounts = providerAccounts.filter(
    (a) => !usedAccountIds.has(a.id) || a.id === accountEntry.account_id,
  );

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '8px 0',
        borderBottom: '1px solid var(--semi-color-border)',
      }}
    >
      {dragHandle && (
        <span
          style={{
            color: 'var(--semi-color-text-2)',
            fontSize: 14,
            width: 24,
            textAlign: 'center',
            flexShrink: 0,
          }}
        >
          {dragHandle}
        </span>
      )}
      <Select
        placeholder="选择服务商"
        value={selectedProvider}
        onChange={(v) => onProviderChange(index, v as string)}
        size="small"
        style={{ flex: 1, minWidth: 0 }}
      >
        {providers.map((p) => (
          <Select.Option key={p.id} value={p.id}>
            {p.name}
          </Select.Option>
        ))}
      </Select>
      <Select
        placeholder="选择账号"
        value={accountEntry.account_id || undefined}
        onChange={(v) => onAccountChange(index, v as string)}
        loading={accountsLoading}
        disabled={!selectedProvider}
        size="small"
        style={{ flex: 1.5, minWidth: 0 }}
      >
        {availAccounts.map((a) => (
          <Select.Option key={a.id} value={a.id}>
            {a.status === 'enabled' ? (
              <Tag color="green" size="small" style={{ marginRight: 4 }}>
                已启用
              </Tag>
            ) : (
              <Tag color="grey" size="small" style={{ marginRight: 4 }}>
                已禁用{a.disabled_reason ? `（${disabledReasonLabel(a.disabled_reason)}）` : ''}
              </Tag>
            )}
            {a.name} (******{a.api_key_suffix})
          </Select.Option>
        ))}
      </Select>
      {isWeighted ? (
        <InputNumber
          value={accountEntry.weight ?? 1}
          min={0}
          max={100}
          size="small"
          style={{ width: 80, flexShrink: 0 }}
          onChange={(v) => onFieldChange(index, 'weight', v as number | null)}
        />
      ) : (
        <span style={{ width: 80, flexShrink: 0 }} />
      )}
      <Button
        size="small"
        type="danger"
        icon={null}
        style={{ flexShrink: 0 }}
        onClick={() => onRemove(rowKey)}
      >
        −
      </Button>
    </div>
  );
}

// ---- 拖拽排序列表（优先级模式） ----

/** 可拖拽排序的账号列表（优先级模式下使用），支持通过拖拽调整顺序 */
function DraggableAccountList({
  accounts,
  rowKeys,
  rowSelectedProviders,
  accountsCache,
  loadingProviders,
  usedAccountIds,
  providers,
  onProviderChange,
  onAccountChange,
  onFieldChange,
  onRemove,
  onReorder,
}: {
  accounts: AccountEntry[];
  rowKeys: string[];
  rowSelectedProviders: (string | null)[];
  accountsCache: Record<string, AccountOption[]>;
  loadingProviders: Set<string>;
  usedAccountIds: Set<string>;
  providers: ProviderOption[];
  onProviderChange: (index: number, providerId: string) => void;
  onAccountChange: (index: number, accountId: string) => void;
  onFieldChange: (index: number, field: 'weight' | 'priority', value: number | null) => void;
  onRemove: (rowKey: string) => void;
  onReorder: (accounts: AccountEntry[]) => void;
}): ReactNode {
  const [draggingIndex, setDraggingIndex] = useState<number | null>(null);

  const handleDragStart = (e: React.DragEvent, index: number) => {
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', String(index));
    setDraggingIndex(index);
  };
  const handleDragEnd = () => {
    setDraggingIndex(null);
  };
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  };
  const handleDrop = (e: React.DragEvent, dropIndex: number) => {
    e.preventDefault();
    setDraggingIndex(null);
    const dragIndex = Number(e.dataTransfer.getData('text/plain'));
    if (dragIndex === dropIndex || isNaN(dragIndex)) return;
    const newAccounts = [...accounts];
    const [moved] = newAccounts.splice(dragIndex, 1);
    newAccounts.splice(dropIndex, 0, moved);
    onReorder(newAccounts);
  };

  return (
    <div>
      {/* 表头 */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '8px 0',
          borderBottom: '2px solid var(--semi-color-border)',
          fontWeight: 500,
          fontSize: 13,
          color: 'var(--semi-color-text-0)',
        }}
      >
        <span style={{ width: 24, flexShrink: 0, textAlign: 'center' }}>⠿</span>
        <span style={{ flex: 1, minWidth: 0 }}>服务商</span>
        <span style={{ flex: 1.5, minWidth: 0 }}>账号</span>
        <span style={{ width: 80, flexShrink: 0 }} />
        <span style={{ width: 48, flexShrink: 0 }} />
      </div>

      {accounts.length === 0 && (
        <div
          style={{
            color: 'var(--semi-color-text-2)',
            padding: 16,
            textAlign: 'center',
          }}
        >
          暂无账号，请点击"添加账号"
        </div>
      )}

      {accounts.map((accountEntry, index) => (
        <div
          key={rowKeys[index]}
          draggable
          onDragStart={(e) => handleDragStart(e, index)}
          onDragEnd={handleDragEnd}
          onDragOver={handleDragOver}
          onDrop={(e) => handleDrop(e, index)}
          style={{ cursor: draggingIndex === index ? 'grabbing' : 'grab' }}
        >
          <AccountRowEditor
            accountEntry={accountEntry}
            index={index}
            rowKey={rowKeys[index]}
            selectedProvider={rowSelectedProviders[index]}
            providerAccounts={
              rowSelectedProviders[index] ? (accountsCache[rowSelectedProviders[index]!] ?? []) : []
            }
            accountsLoading={
              rowSelectedProviders[index]
                ? loadingProviders.has(rowSelectedProviders[index]!)
                : false
            }
            usedAccountIds={usedAccountIds}
            providers={providers}
            onProviderChange={onProviderChange}
            onAccountChange={onAccountChange}
            onFieldChange={onFieldChange}
            onRemove={onRemove}
            isWeighted={false}
            dragHandle={'⠿'}
          />
        </div>
      ))}
    </div>
  );
}

// ---- 主组件 ----

/**
 * AccessPointDrawer - 接入点编辑抽屉组件
 *
 * 提供接入点的创建和编辑功能，包括基本信息、账户池选择、模型路由配置。
 * 支持权重负载均衡和优先级排序两种路由策略。
 */
export default function AccessPointDrawer({
  visible,
  editingAccessPoint,
  saving,
  formData,
  providers,
  accounts,
  accountsLoading: _accountsLoading,
  onClose,
  onFormChange,
  onSave,
}: AccessPointDrawerProps): ReactNode {
  // 账号缓存: provider_id → AccountOption[]
  const [accountsCache, setAccountsCache] = useState<Record<string, AccountOption[]>>({});
  const [loadingProviders, setLoadingProviders] = useState<Set<string>>(new Set());

  // 每行选中的 provider
  const [rowSelectedProviders, setRowSelectedProviders] = useState<(string | null)[]>([]);

  // 行键：有 account_id 则用它，否则用索引兜底
  const rowKeys = useMemo(() => {
    return formData.accounts.map((b, i) => b.account_id || `__row_${i}`);
  }, [formData.accounts]);

  // 合并全局 accounts 和本地缓存
  const allKnownAccounts = useMemo(() => {
    const merged = [...accounts];
    for (const list of Object.values(accountsCache)) {
      for (const a of list) {
        if (!merged.find((m) => m.id === a.id)) merged.push(a);
      }
    }
    return merged;
  }, [accounts, accountsCache]);

  // 从已选账号反向推导 provider
  function resolveProviderIdForAccount(accountId: string): string | undefined {
    return allKnownAccounts.find((a) => a.id === accountId)?.provider_id;
  }

  // 同步 rowSelectedProviders 长度。仅处理新增行；删除由 handleRemove 同步。
  const prevAccountsLenRef = useRef(formData.accounts.length);
  useEffect(() => {
    const prevLen = prevAccountsLenRef.current;
    const curLen = formData.accounts.length;
    prevAccountsLenRef.current = curLen;
    if (curLen > prevLen) {
      // 新增了行，追加 null
      setRowSelectedProviders((prev) => [...prev, ...Array(curLen - prevLen).fill(null)]);
    }
    // curLen < prevLen 的情况由 handleRemove 处理
  }, [formData.accounts.length]);

  // 为已有 account_id 的行回填 provider
  useEffect(() => {
    const updated = [...rowSelectedProviders];
    let changed = false;
    formData.accounts.forEach((b, i) => {
      if (b.account_id && !updated[i]) {
        const pid = resolveProviderIdForAccount(b.account_id);
        if (pid) {
          updated[i] = pid;
          changed = true;
        }
      }
    });
    if (changed) setRowSelectedProviders(updated);
  }, [formData.accounts, allKnownAccounts]); // eslint-disable-line react-hooks/exhaustive-deps

  // 编辑时预加载所有服务商账号，用于回显已有账号的 provider
  const preloadedRef = useRef(false);
  useEffect(() => {
    if (!editingAccessPoint || providers.length === 0 || preloadedRef.current) return;
    preloadedRef.current = true;
    providers.forEach(async (p) => {
      if (accountsCache[p.id]) return;
      setLoadingProviders((prev) => new Set(prev).add(p.id));
      try {
        const data = await api.get<AccountOption[]>(`/api/providers/${p.id}/accounts`);
        setAccountsCache((prev) => ({ ...prev, [p.id]: data }));
      } catch {
        console.warn(`[AccessPointDrawer] 预加载服务商 ${p.id} 账号失败`);
        setAccountsCache((prev) => ({ ...prev, [p.id]: [] }));
      } finally {
        setLoadingProviders((prev) => {
          const next = new Set(prev);
          next.delete(p.id);
          return next;
        });
      }
    });
  }, [editingAccessPoint, providers.length]); // eslint-disable-line react-hooks/exhaustive-deps

  const loadProviderAccounts = useCallback(
    async (providerId: string) => {
      if (accountsCache[providerId] || loadingProviders.has(providerId)) return;
      setLoadingProviders((prev) => new Set(prev).add(providerId));
      try {
        const data = await api.get<AccountOption[]>(`/api/providers/${providerId}/accounts`);
        setAccountsCache((prev) => ({ ...prev, [providerId]: data }));
      } catch {
        console.warn(`[AccessPointDrawer] 加载服务商 ${providerId} 账号失败`);
        setAccountsCache((prev) => ({ ...prev, [providerId]: [] }));
      } finally {
        setLoadingProviders((prev) => {
          const next = new Set(prev);
          next.delete(providerId);
          return next;
        });
      }
    },
    [accountsCache, loadingProviders],
  );

  const handleProviderChange = useCallback(
    (index: number, providerId: string) => {
      const updated = [...rowSelectedProviders];
      updated[index] = providerId;
      setRowSelectedProviders(updated);
      // 清除该行的 account 选择
      const accounts = [...formData.accounts];
      accounts[index] = { ...accounts[index], account_id: '' };
      onFormChange({ ...formData, accounts });
      // 加载该 provider 的账号
      loadProviderAccounts(providerId);
    },
    [formData, rowSelectedProviders, loadProviderAccounts, onFormChange],
  );

  const handleAccountChange = useCallback(
    (index: number, accountId: string) => {
      const accounts = [...formData.accounts];
      accounts[index] = { ...accounts[index], account_id: accountId };
      onFormChange({ ...formData, accounts });
    },
    [formData, onFormChange],
  );

  const handleFieldChange = useCallback(
    (index: number, field: 'weight' | 'priority', value: number | null) => {
      const accounts = [...formData.accounts];
      accounts[index] = {
        ...accounts[index],
        [field]: value ?? (field === 'weight' ? 1 : index + 1),
      };
      // priority 模式下自动排序
      if (field === 'priority' && formData.routing_strategy === 'priority') {
        accounts.sort((a, b) => (a.priority ?? 1) - (b.priority ?? 1));
      }
      onFormChange({ ...formData, accounts });
    },
    [formData, onFormChange],
  );

  // ref 避免闭包陈旧导致删除错行
  const formDataRef = useRef(formData);
  formDataRef.current = formData;
  const rowKeysRef = useRef(rowKeys);
  rowKeysRef.current = rowKeys;

  const handleRemove = useCallback(
    (rowKey: string) => {
      const curData = formDataRef.current;
      const curKeys = rowKeysRef.current;
      // 通过 key 过滤 accounts 和 rowSelectedProviders，保持二者同步
      const accounts = curData.accounts.filter(
        (_b: AccountEntry, i: number) => curKeys[i] !== rowKey,
      );
      setRowSelectedProviders((prev) => prev.filter((_, i: number) => curKeys[i] !== rowKey));
      onFormChange({ ...curData, accounts });
    },
    [onFormChange],
  );

  const handleReorder = useCallback(
    (reordered: AccountEntry[]) => {
      const updated = reordered.map((b, i) => ({ ...b, priority: i + 1 }));
      onFormChange({ ...formData, accounts: updated });
      // 重建 rowSelectedProviders
      const newProviders = reordered.map((b) =>
        b.account_id ? (resolveProviderIdForAccount(b.account_id) ?? null) : null,
      );
      setRowSelectedProviders(newProviders);
    },
    [formData, accounts, accountsCache, onFormChange],
  );

  const handleAddAccount = useCallback(() => {
    const next: AccountEntry = {
      account_id: '',
      weight: 1,
      priority: formData.accounts.length + 1,
    };
    onFormChange({ ...formData, accounts: [...formData.accounts, next] });
  }, [formData, onFormChange]);

  // 已在池中的 account_id 集合
  const usedAccountIds = useMemo(() => {
    const s = new Set<string>();
    formData.accounts.forEach((b) => {
      if (b.account_id) s.add(b.account_id);
    });
    return s;
  }, [formData.accounts]);

  // ---- ModelRoutingGrid ----
  const gridProviderIds = useMemo(() => {
    const ids = providerIdsFromAccounts(formData.accounts, allKnownAccounts);
    const existingIds = formData.model_routing_grid.provider_ids;
    const merged = existingIds.filter((pid) => ids.includes(pid));
    for (const pid of ids) {
      if (!merged.includes(pid)) merged.push(pid);
    }
    return merged;
  }, [formData.accounts, allKnownAccounts, formData.model_routing_grid.provider_ids]);

  const gridRows = formData.model_routing_grid.rows;

  const handleTargetChange = (rowIndex: number, providerId: string, value: string | null) => {
    const rows = gridRows.map((row, ri) =>
      ri !== rowIndex ? row : { ...row, targets: { ...row.targets, [providerId]: value } },
    );
    onFormChange({ ...formData, model_routing_grid: { provider_ids: gridProviderIds, rows } });
  };

  const handleAddRow = () => {
    const targets: Record<string, string | null> = {};
    gridProviderIds.forEach((pid) => {
      targets[pid] = null;
    });
    onFormChange({
      ...formData,
      model_routing_grid: {
        provider_ids: gridProviderIds,
        rows: [...gridRows, { source_model: '', targets }],
      },
    });
  };

  const handleRemoveRow = (rowIndex: number) => {
    const rows = gridRows.filter((_, i) => i !== rowIndex);
    onFormChange({ ...formData, model_routing_grid: { provider_ids: gridProviderIds, rows } });
  };

  const handleRowSourceChange = (rowIndex: number, value: string) => {
    // 防重复
    const dup = gridRows.find((r, i) => i !== rowIndex && r.source_model === value);
    if (dup) {
      // Semi Table render 中无法直接调用 Toast，静默忽略
      return;
    }
    const rows = gridRows.map((row, ri) =>
      ri !== rowIndex ? row : { ...row, source_model: value },
    );
    onFormChange({ ...formData, model_routing_grid: { provider_ids: gridProviderIds, rows } });
  };

  // 确保 provider_ids 同步 + __unmatched__ 行始终存在
  useEffect(() => {
    if (gridProviderIds.length === 0) return;
    const hasUnmatched = gridRows.some((r) => r.source_model === UNMATCHED_MODEL);
    let rows = gridRows;
    if (!hasUnmatched) {
      const targets: Record<string, string | null> = {};
      gridProviderIds.forEach((pid) => {
        targets[pid] = null;
      });
      rows = [{ source_model: UNMATCHED_MODEL, targets }, ...rows];
    }
    const synced = { provider_ids: gridProviderIds, rows };
    if (JSON.stringify(formData.model_routing_grid) !== JSON.stringify(synced)) {
      onFormChange({ ...formData, model_routing_grid: synced });
    }
  }, [gridProviderIds, gridRows]); // eslint-disable-line react-hooks/exhaustive-deps

  // ---- 渲染 ----
  const isWeighted = formData.routing_strategy === 'weighted';

  const weightedDataSource = useMemo(
    () =>
      formData.accounts.map((b, i) => ({
        ...b,
        _key: rowKeys[i],
      })),
    [formData.accounts, rowKeys],
  );

  const accountPoolWeightedColumns = [
    {
      title: '服务商',
      key: 'provider',
      width: 150,
      render: (_: unknown, _record: AccountEntry, index: number) => (
        <Select
          placeholder="选择服务商"
          value={rowSelectedProviders[index]}
          onChange={(v) => handleProviderChange(index, v as string)}
          size="small"
          style={{ width: '100%' }}
        >
          {providers.map((p) => (
            <Select.Option key={p.id} value={p.id}>
              {p.name}
            </Select.Option>
          ))}
        </Select>
      ),
    },
    {
      title: '账号',
      key: 'account',
      width: 260,
      render: (_: unknown, record: AccountEntry, index: number) => {
        const sp = rowSelectedProviders[index];
        const provAccounts = sp ? (accountsCache[sp] ?? []) : [];
        const isLoading = sp ? loadingProviders.has(sp) : false;
        const avail = provAccounts.filter(
          (a) => !usedAccountIds.has(a.id) || a.id === record.account_id,
        );
        return (
          <Select
            placeholder="选择账号"
            value={record.account_id || undefined}
            onChange={(v) => handleAccountChange(index, v as string)}
            loading={isLoading}
            disabled={!sp}
            size="small"
            style={{ width: '100%' }}
          >
            {avail.map((a) => (
              <Select.Option key={a.id} value={a.id}>
                {a.status === 'enabled' ? (
                  <Tag color="green" size="small" style={{ marginRight: 4 }}>
                    已启用
                  </Tag>
                ) : (
                  <Tag color="grey" size="small" style={{ marginRight: 4 }}>
                    已禁用{a.disabled_reason ? `（${disabledReasonLabel(a.disabled_reason)}）` : ''}
                  </Tag>
                )}
                {a.name} (******{a.api_key_suffix})
              </Select.Option>
            ))}
          </Select>
        );
      },
    },
    {
      title: '权重',
      key: 'weight',
      width: 100,
      render: (_: unknown, record: AccountEntry, index: number) => (
        <InputNumber
          value={record.weight ?? 1}
          min={0}
          max={100}
          size="small"
          style={{ width: 80 }}
          onChange={(v) => handleFieldChange(index, 'weight', v as number | null)}
        />
      ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 60,
      render: (_: unknown, _record: AccountEntry, index: number) => (
        <Button size="small" type="danger" icon={null} onClick={() => handleRemove(rowKeys[index])}>
          −
        </Button>
      ),
    },
  ];

  // 模型族预设
  const MODEL_FAMILIES = useMemo(
    () => [
      { label: '未匹配', value: UNMATCHED_MODEL, matchType: 'prefix' as const },
      { label: 'Claude Opus', value: 'claude-opus-', matchType: 'prefix' as const },
      { label: 'Claude Sonnet', value: 'claude-sonnet-', matchType: 'prefix' as const },
      { label: 'Claude Haiku', value: 'claude-haiku-', matchType: 'prefix' as const },
    ],
    [],
  );

  const sourceModelOptions = useMemo(() => {
    const seen = new Set<string>();
    // 1. 模型族预设（固定在前）
    const families = MODEL_FAMILIES.map((f) => ({
      value: f.value,
      label: (
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
          <Tag color="purple" size="small">
            模式匹配
          </Tag>
          <span>{f.label}</span>
        </span>
      ),
    }));
    // 2. 已存在的自定义值（去重）
    const custom: { value: string; label: ReactNode }[] = [];
    for (const r of gridRows) {
      if (!r.source_model || r.source_model === UNMATCHED_MODEL) continue;
      if (MODEL_FAMILIES.some((f) => f.value === r.source_model)) continue;
      if (seen.has(r.source_model)) continue;
      seen.add(r.source_model);
      custom.push({
        value: r.source_model,
        label: (
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <Tag color="blue" size="small">
              精准匹配
            </Tag>
            <span>{r.source_model}</span>
          </span>
        ),
      });
    }
    return [...families, ...custom];
  }, [gridRows, MODEL_FAMILIES]);

  const gridColumns = [
    {
      title: '原始模型',
      key: 'source_model',
      width: 200,
      render: (_: unknown, record: ModelRoutingRow, rowIndex: number) => {
        const locked = record.source_model === UNMATCHED_MODEL;
        return locked ? (
          <span
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 6,
              height: 32,
              padding: '0 12px',
              border: '1px solid var(--semi-color-border)',
              borderRadius: 'var(--semi-border-radius-small)',
              background: 'var(--semi-color-fill-0)',
              width: '100%',
              boxSizing: 'border-box',
            }}
          >
            <Tag color="purple" size="small">
              模式匹配
            </Tag>
            <span>未匹配</span>
          </span>
        ) : (
          <Select
            value={record.source_model || undefined}
            onChange={(v) => handleRowSourceChange(rowIndex, v as string)}
            filter
            allowCreate
            optionList={sourceModelOptions}
            placeholder="选择或输入模型"
            style={{ width: '100%' }}
          />
        );
      },
    },
    ...gridProviderIds.map((pid) => {
      const prov = providers.find((p) => p.id === pid);
      return {
        title: prov?.name ?? pid,
        key: pid,
        width: 180,
        render: (_: unknown, record: ModelRoutingRow) => {
          const currentVal = record.targets[pid] ?? null;
          const modelOpts = (prov?.models ?? []).map((m) => ({ value: m, label: m }));
          return (
            <Select
              value={currentVal ?? undefined}
              placeholder="留空"
              filter
              showClear
              optionList={modelOpts}
              style={{ width: '100%' }}
              onChange={(v) =>
                handleTargetChange(gridRows.indexOf(record), pid, (v as string) ?? null)
              }
            />
          );
        },
      };
    }),
    {
      title: '操作',
      key: 'actions',
      width: 80,
      render: (_: unknown, record: ModelRoutingRow) =>
        record.source_model === UNMATCHED_MODEL ? (
          <Tooltip content="无法删除未匹配规则">
            <Button size="small" type="danger" disabled icon={null}>
              −
            </Button>
          </Tooltip>
        ) : (
          <Button
            size="small"
            type="danger"
            onClick={() => handleRemoveRow(gridRows.indexOf(record))}
          >
            −
          </Button>
        ),
    },
  ];

  const labelStyle = {
    marginBottom: 4,
    fontSize: 14,
    fontWeight: 500,
    color: 'var(--semi-color-text-0)',
  } as const;

  return (
    <SideSheet
      title={editingAccessPoint ? '编辑接入点' : '创建接入点'}
      visible={visible}
      onCancel={onClose}
      size="large"
      maskClosable
    >
      <div style={{ padding: '0 4px' }}>
        {/* 区域 1: 基本字段 */}
        <div style={labelStyle}>名称</div>
        <Input
          value={formData.name}
          onChange={(value: string) => onFormChange({ ...formData, name: value })}
          placeholder="接入点名称"
        />
        <div style={{ marginTop: 16, ...labelStyle }}>Short Code</div>
        <Input
          value={formData.short_code}
          onChange={(value: string) => onFormChange({ ...formData, short_code: value })}
          placeholder="留空则自动生成 16 位短码"
        />
        <div style={{ marginTop: 16, ...labelStyle }}>API 类型</div>
        <Select
          value={formData.api_type}
          onChange={(value) => onFormChange({ ...formData, api_type: value as string })}
          style={{ width: '100%' }}
        >
          <Select.Option value="anthropic">Anthropic</Select.Option>
          <Select.Option value="openai" disabled>
            OpenAI（尚未支持）
          </Select.Option>
        </Select>

        {/* 区域 2: 路由策略 */}
        <div style={{ marginTop: 28, ...labelStyle }}>路由策略</div>
        <Select
          value={formData.routing_strategy}
          onChange={(value) => onFormChange({ ...formData, routing_strategy: value as string })}
          style={{ width: '100%' }}
        >
          <Select.Option value="weighted">权重负载均衡</Select.Option>
          <Select.Option value="priority">优先级排序</Select.Option>
        </Select>

        {/* 区域 3: 账号池 */}
        <div style={{ marginTop: 28 }}>
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginBottom: 12,
            }}
          >
            <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--semi-color-text-0)' }}>
              账号池
            </span>
            <Button size="small" onClick={handleAddAccount}>
              添加账号
            </Button>
          </div>

          {isWeighted ? (
            <Table
              columns={accountPoolWeightedColumns}
              dataSource={weightedDataSource}
              rowKey="_key"
              size="small"
              pagination={false}
              empty={
                <div style={{ color: 'var(--semi-color-text-2)', padding: 16 }}>
                  暂无账号，请点击"添加账号"
                </div>
              }
            />
          ) : (
            <DraggableAccountList
              accounts={formData.accounts}
              rowKeys={rowKeys}
              rowSelectedProviders={rowSelectedProviders}
              accountsCache={accountsCache}
              loadingProviders={loadingProviders}
              usedAccountIds={usedAccountIds}
              providers={providers}
              onProviderChange={handleProviderChange}
              onAccountChange={handleAccountChange}
              onFieldChange={handleFieldChange}
              onRemove={handleRemove}
              onReorder={handleReorder}
            />
          )}
        </div>

        {/* 区域 4: 模型路由映射表 */}
        <div style={{ marginTop: 28 }}>
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginBottom: 12,
            }}
          >
            <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--semi-color-text-0)' }}>
              模型路由映射表
            </span>
            <Button size="small" onClick={handleAddRow}>
              添加行
            </Button>
          </div>
          <Table
            columns={gridColumns}
            dataSource={gridRows}
            rowKey={(record) => record?.source_model ?? ''}
            size="small"
            pagination={false}
            empty={
              <div style={{ color: 'var(--semi-color-text-2)', padding: 16 }}>
                暂无路由映射，请点击"添加行"添加映射规则
              </div>
            }
          />
        </div>

        <Button
          type="primary"
          onClick={onSave}
          loading={saving}
          disabled={saving}
          block
          style={{ marginTop: 24 }}
        >
          {editingAccessPoint ? '更新' : '创建'}
        </Button>
      </div>
    </SideSheet>
  );
}

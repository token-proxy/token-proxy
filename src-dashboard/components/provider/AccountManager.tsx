import { type ReactNode, useRef, useState } from 'react';
import {
  Button,
  Input,
  Popconfirm,
  SideSheet,
  Space,
  Table,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { IconEyeClosedSolid, IconEyeOpened } from '@douyinfe/semi-icons';
import api from '../../api.ts';

const { Title, Text } = Typography;

/** 账号信息 */
export interface Account {
  id: string;
  provider_id: string;
  name: string;
  /** API Key 末尾几位，用于区分不同账号 */
  api_key_suffix: string;
  /** 账号状态: enabled | disabled */
  status: string;
  disabled_reason?: string;
  available_at?: string;
  created_at: string;
  updated_at: string;
}

// ── 辅助 ──

/** 根据 disabled_reason 返回中文后缀 */
function disabledReasonSuffix(reason?: string): string {
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
      return '';
  }
}

/** AccountManager 组件 Props */
interface AccountManagerProps {
  providerId: string;
  accounts: Account[];
  loading: boolean;
  onAccountsChanged: () => void;
}

/**
 * AccountManager - 服务商账号管理组件
 *
 * 提供对服务商下账号的增删改查、启用/禁用、API Key 编辑等功能。
 * API Key 输入框使用 CSS 遮罩代替 type="password" 以避免浏览器密码管理器干扰。
 */
export default function AccountManager({
  providerId,
  accounts,
  loading,
  onAccountsChanged,
}: AccountManagerProps): ReactNode {
  const [accountFormVisible, setAccountFormVisible] = useState(false);
  const [editingAccount, setEditingAccount] = useState<Account | null>(null);
  const [accountForm, setAccountForm] = useState({ name: '', api_key: '' });
  const [accountSaving, setAccountSaving] = useState(false);
  const [apiKeyVisible, setApiKeyVisible] = useState(false);
  const [operatingIds, setOperatingIds] = useState<string[]>([]);
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
    if (!editingAccount && !accountForm.api_key.trim()) {
      Toast.error('请填写 API Key');
      return;
    }
    setAccountSaving(true);
    try {
      if (editingAccount) {
        const body: Record<string, string> = { name: accountForm.name };
        if (accountForm.api_key.trim()) body.api_key = accountForm.api_key.trim();
        await api.put(`/api/providers/${providerId}/accounts/${editingAccount.id}`, body);
        Toast.success('账号已更新');
      } else {
        await api.post(`/api/providers/${providerId}/accounts`, {
          name: accountForm.name.trim() || undefined,
          api_key: accountForm.api_key.trim(),
        });
        Toast.success('账号已创建');
      }
      setAccountFormVisible(false);
      onAccountsChanged();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '保存账号失败');
    } finally {
      setAccountSaving(false);
    }
  };

  const handleDeleteAccount = async (id: string) => {
    const operationKey = `account:${id}`;
    if (operatingIdsRef.current.has(operationKey)) return;
    setOperation(operationKey, true);
    try {
      await api.delete(`/api/providers/${providerId}/accounts/${id}`);
      Toast.success('账号已删除');
      onAccountsChanged();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '删除账号失败');
    } finally {
      setOperation(operationKey, false);
    }
  };

  const handleToggleAccountStatus = async (accountId: string, currentStatus: string) => {
    const operationKey = `status:${accountId}`;
    if (operatingIdsRef.current.has(operationKey)) return;
    setOperation(operationKey, true);
    const nextStatus = currentStatus === 'enabled' ? 'disabled' : 'enabled';
    try {
      await api.put(`/api/accounts/${accountId}/status`, { status: nextStatus });
      Toast.success(`账号已${nextStatus === 'enabled' ? '启用' : '禁用'}`);
      onAccountsChanged();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '操作失败');
    } finally {
      setOperation(operationKey, false);
    }
  };

  const accountColumns = [
    { title: '名称', dataIndex: 'name', key: 'name', width: 120 },
    {
      title: 'API Key',
      dataIndex: 'api_key_suffix',
      key: 'api_key_suffix',
      width: 140,
      render: (suffix: string) => (suffix ? `******${suffix}` : '-'),
    },
    {
      title: '状态',
      key: 'status',
      width: 200,
      render: (_: unknown, record: Account) => {
        if (record.status === 'enabled') {
          return (
            <Tag color="green" size="small">
              已启用
            </Tag>
          );
        }
        const suffix = disabledReasonSuffix(record.disabled_reason);
        const label = suffix ? `已禁用（${suffix}）` : '已禁用';
        const availableAt = record.available_at
          ? new Date(record.available_at).toLocaleString()
          : null;
        return (
          <div>
            <Tag color="grey" size="small">
              {label}
            </Tag>
            {availableAt && (
              <div style={{ marginTop: 2 }}>
                <Text type="tertiary" size="small">
                  预计 {availableAt} 恢复
                </Text>
              </div>
            )}
          </div>
        );
      },
    },
    {
      title: '操作',
      key: 'actions',
      width: 220,
      render: (_: unknown, record: Account) => (
        <Space>
          <Button size="small" onClick={() => handleOpenAccountForm(record)}>
            编辑
          </Button>
          <Button
            size="small"
            type="danger"
            loading={operatingIds.includes(`status:${record.id}`)}
            onClick={() => handleToggleAccountStatus(record.id, record.status)}
          >
            {record.status === 'enabled' ? '禁用' : '启用'}
          </Button>
          <Popconfirm
            title="确认删除此账号?"
            onConfirm={() => handleDeleteAccount(record.id)}
            position="bottomRight"
          >
            <Button
              size="small"
              type="danger"
              loading={operatingIds.includes(`account:${record.id}`)}
            >
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 12,
        }}
      >
        <Title heading={6}>账号管理</Title>
        <Button size="small" onClick={() => handleOpenAccountForm()}>
          添加账号
        </Button>
      </div>
      <Table
        columns={accountColumns}
        dataSource={accounts}
        loading={loading}
        rowKey="id"
        size="small"
        scroll={{ x: 'max-content' }}
        pagination={false}
      />

      <SideSheet
        title={editingAccount ? '编辑账号' : '添加账号'}
        visible={accountFormVisible}
        onCancel={() => setAccountFormVisible(false)}
        width={560}
        maskClosable
      >
        <div style={{ padding: '0 4px' }}>
          <div>
            <div style={{ marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14 }}>
              名称
            </div>
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
              style={
                !apiKeyVisible && accountForm.api_key
                  ? ({
                      WebkitTextSecurity: 'disc',
                      textSecurity: 'disc',
                      fontFamily: 'text-security-disc, monospace',
                    } as React.CSSProperties)
                  : undefined
              }
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
    </>
  );
}

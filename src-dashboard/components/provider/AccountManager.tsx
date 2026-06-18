import { type ReactNode, useRef, useState } from 'react';
import { Button, Input, Popconfirm, SideSheet, Space, Table, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { IconEyeClosedSolid, IconEyeOpened } from '@douyinfe/semi-icons';
import api from '../../api.ts';

const {Title} = Typography;

export interface Account {
  id: string;
  provider_id: string;
  name: string;
  api_key_suffix: string;
  status: string;
  created_at: string;
  updated_at: string;
}

interface AccountManagerProps {
  providerId: string;
  accounts: Account[];
  loading: boolean;
  onAccountsChanged: () => void;
}

export default function AccountManager({
  providerId,
  accounts,
  loading,
  onAccountsChanged,
}: AccountManagerProps): ReactNode {
  const [accountFormVisible, setAccountFormVisible] = useState(false);
  const [editingAccount, setEditingAccount] = useState<Account | null>(null);
  const [accountForm, setAccountForm] = useState({name: '', api_key: ''});
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
      setAccountForm({name: account.name, api_key: ''});
    } else {
      setEditingAccount(null);
      setAccountForm({name: '', api_key: ''});
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
        const body: Record<string, string> = {name: accountForm.name};
        if (accountForm.api_key.trim()) body.api_key = accountForm.api_key.trim();
        await api.put(
          `/api/providers/${providerId}/accounts/${editingAccount.id}`,
          body,
        );
        Toast.success('Account 已更新');
      } else {
        await api.post(
          `/api/providers/${providerId}/accounts`,
          {
            name: accountForm.name.trim() || undefined,
            api_key: accountForm.api_key.trim(),
          },
        );
        Toast.success('Account 已创建');
      }
      setAccountFormVisible(false);
      onAccountsChanged();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '保存 Account 失败');
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
      Toast.success('Account 已删除');
      onAccountsChanged();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '删除 Account 失败');
    } finally {
      setOperation(operationKey, false);
    }
  };

  const accountColumns = [
    {title: '名称', dataIndex: 'name', key: 'name'},
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
    <>
      <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12}}>
        <Title heading={6}>Account 管理</Title>
        <Button size="small" onClick={() => handleOpenAccountForm()}>添加 Account</Button>
      </div>
      <Table
        columns={accountColumns}
        dataSource={accounts}
        loading={loading}
        rowKey="id"
        size="small"
        scroll={{x: 'max-content'}}
        pagination={false}
      />

      <SideSheet
        title={editingAccount ? '编辑 Account' : '添加 Account'}
        visible={accountFormVisible}
        onCancel={() => setAccountFormVisible(false)}
        width={560}
        maskClosable
      >
        <div style={{padding: '0 4px'}}>
          <div>
            <div style={{marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14}}>名称</div>
            <Input
              value={accountForm.name}
              onChange={(v: string) => setAccountForm({...accountForm, name: v})}
              placeholder="留空将自动以 API Key 后缀生成"
              autoComplete="off"
            />
          </div>
          <div style={{marginTop: 16}}>
            <div style={{marginBottom: 4, color: 'var(--semi-color-text-2)', fontSize: 14}}>
              {editingAccount ? 'API Key (留空表示不修改)' : 'API Key'}
            </div>
            {/*
                故意不使用 type="password"，避免触发浏览器密码管理器的"保存账号密码"弹窗。
                改为普通 text input + CSS 视觉遮罩（webkit-text-security）+ 自定义眼睛按钮。
              */}
            <Input
              value={accountForm.api_key}
              onChange={(v: string) => setAccountForm({...accountForm, api_key: v})}
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
                  icon={apiKeyVisible ? <IconEyeClosedSolid/> : <IconEyeOpened/>}
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
            style={{marginTop: 16}}
          >
            {editingAccount ? '更新' : '添加'}
          </Button>
        </div>
      </SideSheet>
    </>
  );
}

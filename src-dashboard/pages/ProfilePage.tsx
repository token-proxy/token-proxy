import { useState, useEffect, useCallback, useRef, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Card, Tabs, TabPane, Form, Button, Input, Toast,
  Typography, Table, Tag, Popconfirm, Modal,
} from '@douyinfe/semi-ui';
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form';
import api from '../api.ts';

const { Title, Text } = Typography;

/* ---- Types ---- */

interface UserProfile {
  id: string;
  username: string;
  display_name: string;
  status: string;
  created_at: string;
  updated_at: string;
}

interface UserApiKey {
  id: string;
  key_prefix: string;
  description: string;
  status: string;
  last_used_at: string | null;
  created_at: string;
}

interface CreateApiKeyResponse {
  id: string;
  full_key: string;
  key_prefix: string;
  description: string;
  status: string;
  created_at: string;
}

/* ---- Helpers ---- */

function formatDate(dateStr: string | null): string {
  if (!dateStr) return '-';
  const date = new Date(dateStr);
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/* ---- Component ---- */

export default function ProfilePage(): ReactNode {
  const navigate = useNavigate();

  /* ---- Profile state ---- */
  const [profile, setProfile] = useState<UserProfile | null>(null);
  const [profileLoading, setProfileLoading] = useState(false);
  const [profileSaving, setProfileSaving] = useState(false);

  /* ---- Password state ---- */
  const [passwordSaving, setPasswordSaving] = useState(false);

  /* ---- API key state ---- */
  const [apiKeys, setApiKeys] = useState<UserApiKey[]>([]);
  const [apiKeysLoading, setApiKeysLoading] = useState(false);
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

  /* ---- Create API key state ---- */
  const [createModalVisible, setCreateModalVisible] = useState(false);
  const [createSaving, setCreateSaving] = useState(false);
  const [createdKey, setCreatedKey] = useState<CreateApiKeyResponse | null>(null);
  const createFormRef = useRef<FormApi | null>(null);

  /* ---- Load profile ---- */
  const loadProfile = useCallback(async () => {
    setProfileLoading(true);
    try {
      const data = await api.get<UserProfile>('/api/users/me');
      setProfile(data);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '获取个人资料失败');
    } finally {
      setProfileLoading(false);
    }
  }, []);

  /* ---- Load API keys ---- */
  const loadApiKeys = useCallback(async () => {
    setApiKeysLoading(true);
    try {
      const data = await api.get<UserApiKey[]>('/api/users/me/api-keys');
      setApiKeys(data);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '获取 API Key 列表失败');
    } finally {
      setApiKeysLoading(false);
    }
  }, []);

  useEffect(() => {
    loadProfile();
    loadApiKeys();
  }, [loadProfile, loadApiKeys]);

  /* ---- Update profile ---- */
  const handleSaveProfile = async (values: { display_name: string }) => {
    setProfileSaving(true);
    try {
      const data = await api.put<UserProfile>('/api/users/me/profile', values);
      setProfile(data);
      localStorage.setItem('display_name', data.display_name);
      window.dispatchEvent(new Event('storage'));
      Toast.success('个人资料已更新');
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '保存失败');
    } finally {
      setProfileSaving(false);
    }
  };

  /* ---- Change password ---- */
  const handleChangePassword = async (values: {
    old_password: string;
    new_password: string;
    confirm_password: string;
  }) => {
    if (values.new_password.length < 6) {
      Toast.error('新密码长度不能少于 6 位');
      return;
    }
    if (values.new_password !== values.confirm_password) {
      Toast.error('两次输入的新密码不一致');
      return;
    }
    setPasswordSaving(true);
    try {
      await api.put('/api/users/me/change-password', {
        old_password: values.old_password,
        new_password: values.new_password,
      });
      Toast.success('密码修改成功，请重新登录');
      localStorage.removeItem('access_token');
      localStorage.removeItem('refresh_token');
      localStorage.removeItem('username');
      localStorage.removeItem('display_name');
      navigate('/login');
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '密码修改失败');
    } finally {
      setPasswordSaving(false);
    }
  };

  /* ---- Create API key ---- */
  const handleOpenCreateModal = () => {
    setCreateModalVisible(true);
  };

  const handleCreateApiKey = async (values: { description: string }) => {
    if (!values.description.trim()) {
      Toast.error('备注不能为空');
      return;
    }
    setCreateSaving(true);
    try {
      const data = await api.post<CreateApiKeyResponse>(
        '/api/users/me/api-keys',
        { description: values.description.trim() },
      );
      setCreateModalVisible(false);
      setCreatedKey(data);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '创建失败');
    } finally {
      setCreateSaving(false);
    }
  };

  const handleCreatedKeyModalClose = () => {
    setCreatedKey(null);
    loadApiKeys();
  };

  /* ---- Revoke API key ---- */
  const handleRevokeApiKey = async (id: string) => {
    if (operatingIdsRef.current.has(id)) return;
    setOperation(id, true);
    try {
      await api.post(`/api/users/me/api-keys/${id}/revoke`, {});
      Toast.success('API Key 已吊销');
      loadApiKeys();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '吊销失败');
    } finally {
      setOperation(id, false);
    }
  };

  /* ---- API key columns ---- */
  const apiKeyColumns = [
    { title: '备注', dataIndex: 'description', key: 'description' },
    {
      title: 'API Key',
      dataIndex: 'key_prefix',
      key: 'key_prefix',
      render: (text: string) => `${text}****`,
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (text: string) => (
        <Tag color={text === 'enabled' ? 'green' : 'red'}>
          {text === 'enabled' ? '启用' : '已吊销'}
        </Tag>
      ),
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
      width: 180,
      render: (text: string) => formatDate(text),
    },
    {
      title: '最后使用时间',
      dataIndex: 'last_used_at',
      key: 'last_used_at',
      width: 180,
      render: (text: string | null) => formatDate(text),
    },
    {
      title: '操作',
      key: 'actions',
      width: 120,
      render: (_: unknown, record: UserApiKey) => {
        if (record.status !== 'active') {
          return <Text type="secondary">-</Text>;
        }
        return (
          <Popconfirm
            title="确认吊销此 API Key?"
            content="吊销后使用此 Key 的请求将无法通过认证"
            onConfirm={() => handleRevokeApiKey(record.id)}
            position="bottomRight"
          >
            <Button
              size="small"
              type="danger"
              disabled={operatingIds.includes(record.id)}
              loading={operatingIds.includes(record.id)}
            >
              吊销
            </Button>
          </Popconfirm>
        );
      },
    },
  ];

  return (
    <div>
      <Title heading={3} style={{ marginBottom: 24 }}>个人设置</Title>

      <Card>
        <Tabs type="line" defaultActiveKey="profile">
          <TabPane tab="个人资料" itemKey="profile">
            <div style={{ maxWidth: 480, marginTop: 16 }}>
              {profileLoading && !profile ? (
                <Text type="secondary">加载中...</Text>
              ) : profile ? (
                <Form
                  onSubmit={handleSaveProfile}
                  initValues={{ display_name: profile.display_name }}
                >
                  <Form.Input
                    field="username"
                    label="账号"
                    initValue={profile.username}
                    disabled
                    extraText="账号不可修改"
                  />
                  <Form.Input
                    field="display_name"
                    label="姓名"
                    placeholder="请输入显示名称"
                    rules={[{ required: true, message: '请输入显示名称' }]}
                  />
                  <Button
                    type="primary"
                    htmlType="submit"
                    loading={profileSaving}
                    style={{ marginTop: 16 }}
                  >
                    保存
                  </Button>
                </Form>
              ) : (
                <Text type="secondary">加载失败</Text>
              )}
            </div>
          </TabPane>

          <TabPane tab="修改密码" itemKey="password">
            <div style={{ maxWidth: 480, marginTop: 16 }}>
              <Form onSubmit={handleChangePassword}>
                <Form.Input
                  field="old_password"
                  label="旧密码"
                  mode="password"
                  placeholder="请输入旧密码"
                  rules={[{ required: true, message: '请输入旧密码' }]}
                />
                <Form.Input
                  field="new_password"
                  label="新密码"
                  mode="password"
                  placeholder="请输入新密码（不少于 6 位）"
                  rules={[{ required: true, message: '请输入新密码' }]}
                />
                <Form.Input
                  field="confirm_password"
                  label="确认新密码"
                  mode="password"
                  placeholder="请再次输入新密码"
                  rules={[{ required: true, message: '请再次输入新密码' }]}
                />
                <Button
                  type="primary"
                  htmlType="submit"
                  loading={passwordSaving}
                  style={{ marginTop: 16 }}
                >
                  修改密码
                </Button>
              </Form>
            </div>
          </TabPane>

          <TabPane tab="API Key 管理" itemKey="api-keys">
            <div style={{ marginTop: 16 }}>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'flex-end',
                  marginBottom: 16,
                }}
              >
                <Button type="primary" onClick={handleOpenCreateModal}>
                  创建 API Key
                </Button>
              </div>

              <Table
                columns={apiKeyColumns}
                dataSource={apiKeys}
                loading={apiKeysLoading}
                rowKey="id"
                scroll={{ x: 'max-content' }}
                pagination={{ pageSize: 20 }}
                empty={<Text type="secondary">暂无 API Key</Text>}
              />

              <Modal
                title="创建 API Key"
                visible={createModalVisible}
                onCancel={() => setCreateModalVisible(false)}
                footer={
                  <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
                    <Button disabled={createSaving} onClick={() => setCreateModalVisible(false)}>
                      取消
                    </Button>
                    <Button
                      type="primary"
                      loading={createSaving}
                      onClick={() => createFormRef.current?.submitForm()}
                    >
                      创建
                    </Button>
                  </div>
                }
                width={560}
                maskClosable
              >
                <Form
                  onSubmit={handleCreateApiKey}
                  getFormApi={(api) => { createFormRef.current = api; }}
                >
                  <Form.Input
                    field="description"
                    label="备注"
                    placeholder="请输入备注信息"
                    rules={[{ required: true, message: '请输入备注' }]}
                  />
                </Form>
              </Modal>

              <Modal
                title="API Key 创建成功"
                visible={!!createdKey}
                onCancel={handleCreatedKeyModalClose}
                width={640}
                footer={
                  <Button type="primary" onClick={handleCreatedKeyModalClose}>
                    已保存，关闭
                  </Button>
                }
                maskClosable
              >
                {createdKey && (
                  <div>
                    <Text
                      type="warning"
                      style={{ display: 'block', marginBottom: 12 }}
                    >
                      关闭此弹窗后将无法再次查看完整 Key，请立即复制并安全保存。
                    </Text>
                    <Input
                      value={createdKey.full_key}
                      readOnly
                      style={{ marginBottom: 8 }}
                    />
                    <Button
                      type="primary"
                      onClick={() => {
                        navigator.clipboard.writeText(createdKey.full_key).then(
                          () => Toast.success('已复制到剪贴板'),
                          () => Toast.error('复制失败，请手动复制'),
                        );
                      }}
                    >
                      复制 Key
                    </Button>
                  </div>
                )}
              </Modal>
            </div>
          </TabPane>
        </Tabs>
      </Card>
    </div>
  );
}
import { type ReactNode, useRef, useState } from 'react';
import { useFetch } from '../../hooks/useFetch.ts';
import {
  Button,
  Form,
  Input,
  Modal,
  Popconfirm,
  Space,
  Table,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form';
import api from '../../api.ts';

const { Text } = Typography;

/* ---- 类型定义 ---- */

/** 用户 API Key 信息 */
interface UserApiKey {
  id: string;
  key_prefix: string;
  description: string;
  status: string;
  last_used_at: string | null;
  created_at: string;
}

/** 创建 API Key 的响应 */
interface CreateApiKeyResponse {
  id: string;
  full_key: string;
  key_prefix: string;
  description: string;
  status: string;
  created_at: string;
}

/* ---- 辅助函数 ---- */

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

/* ---- 组件 ---- */

/**
 * ApiKeyManager - 用户 API Key 管理组件
 *
 * 提供个人 API Key 的创建、编辑备注、吊销等功能。
 * 创建成功后弹窗展示完整 Key，并提示用户立即保存。
 */
export default function ApiKeyManager(): ReactNode {
  /* ---- API key 状态 ---- */
  const {
    data: apiKeys,
    loading: apiKeysLoading,
    refetch: loadApiKeys,
  } = useFetch(() => api.get<UserApiKey[]>('/api/users/me/api-keys'), []);
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

  /* ---- 创建 API key 状态 ---- */
  const [createModalVisible, setCreateModalVisible] = useState(false);
  const [createSaving, setCreateSaving] = useState(false);
  const [createdKey, setCreatedKey] = useState<CreateApiKeyResponse | null>(null);
  const createFormRef = useRef<FormApi | null>(null);

  /* ---- 编辑 API key 备注状态 ---- */
  const [editModalVisible, setEditModalVisible] = useState(false);
  const [editingKey, setEditingKey] = useState<UserApiKey | null>(null);
  const [editSaving, setEditSaving] = useState(false);
  const editFormRef = useRef<FormApi | null>(null);

  /* ---- 创建 API key ---- */
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
      const data = await api.post<CreateApiKeyResponse>('/api/users/me/api-keys', {
        description: values.description.trim(),
      });
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

  /* ---- 吊销 API key ---- */
  const handleRevokeApiKey = async (id: string) => {
    if (operatingIdsRef.current.has(id)) return;
    setOperation(id, true);
    try {
      await api.delete(`/api/users/me/api-keys/${id}`);
      Toast.success('API Key 已吊销');
      loadApiKeys();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '吊销失败');
    } finally {
      setOperation(id, false);
    }
  };

  /* ---- 编辑 API key 备注 ---- */
  const handleOpenEditModal = (key: UserApiKey) => {
    setEditingKey(key);
    setEditModalVisible(true);
  };

  const handleEditApiKey = async (values: { description: string }) => {
    if (!editingKey) return;
    if (!values.description.trim()) {
      Toast.error('备注不能为空');
      return;
    }
    setEditSaving(true);
    try {
      await api.put(`/api/users/me/api-keys/${editingKey.id}`, {
        description: values.description.trim(),
      });
      Toast.success('备注已更新');
      setEditModalVisible(false);
      setEditingKey(null);
      loadApiKeys();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '更新失败');
    } finally {
      setEditSaving(false);
    }
  };

  /* ---- API key 列定义 ---- */
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
      width: 160,
      render: (_: unknown, record: UserApiKey) => {
        if (record.status !== 'enabled') {
          return <Text type="secondary">-</Text>;
        }
        return (
          <Space>
            <Button size="small" onClick={() => handleOpenEditModal(record)}>
              编辑
            </Button>
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
          </Space>
        );
      },
    },
  ];

  return (
    <div>
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
        dataSource={(apiKeys ?? []).filter((k) => k.status === 'enabled')}
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
          getFormApi={(api) => {
            createFormRef.current = api;
          }}
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
        title="修改备注"
        visible={editModalVisible}
        onCancel={() => {
          setEditModalVisible(false);
          setEditingKey(null);
        }}
        footer={
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
            <Button disabled={editSaving} onClick={() => setEditModalVisible(false)}>
              取消
            </Button>
            <Button
              type="primary"
              loading={editSaving}
              onClick={() => editFormRef.current?.submitForm()}
            >
              保存
            </Button>
          </div>
        }
        width={480}
        maskClosable
      >
        <Form
          onSubmit={handleEditApiKey}
          getFormApi={(api) => {
            editFormRef.current = api;
          }}
          initValues={editingKey ? { description: editingKey.description } : undefined}
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
            <Text type="warning" style={{ display: 'block', marginBottom: 12 }}>
              关闭此弹窗后将无法再次查看完整 Key，请立即复制并安全保存。
            </Text>
            <Input value={createdKey.full_key} readOnly style={{ marginBottom: 8 }} />
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
  );
}

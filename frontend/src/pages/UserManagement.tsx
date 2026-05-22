import { useState, useEffect, useCallback, type ReactNode } from 'react';
import {
  Table, Button, Tag, Space, Popconfirm, SideSheet, Form,
  Toast, Typography,
} from '@douyinfe/semi-ui';
import api from '../api.ts';

const { Title } = Typography;

interface UserResponse {
  id: string;
  username: string;
  display_name: string;
  status: 'enabled' | 'disabled';
  created_at: string;
  updated_at: string;
}

interface UserFormData {
  username: string;
  display_name: string;
  password?: string;
}

export default function UserManagement(): ReactNode {
  const [users, setUsers] = useState<UserResponse[]>([]);
  const [loading, setLoading] = useState(false);
  const [sideSheetVisible, setSideSheetVisible] = useState(false);
  const [editingUser, setEditingUser] = useState<UserResponse | null>(null);
  const [saving, setSaving] = useState(false);

  const currentUsername = localStorage.getItem('username') || '';

  const loadUsers = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.get<UserResponse[]>('/api/users');
      setUsers(data);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '获取用户列表失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadUsers();
  }, [loadUsers]);

  const openCreateDrawer = () => {
    setEditingUser(null);
    setSideSheetVisible(true);
  };

  const openEditDrawer = (user: UserResponse) => {
    setEditingUser(user);
    setSideSheetVisible(true);
  };

  const handleSave = async (values: UserFormData) => {
    setSaving(true);
    try {
      if (editingUser) {
        const body: Record<string, string> = {
          display_name: values.display_name,
        };
        if (values.password) {
          body.password = values.password;
        }
        await api.put(`/api/users/${editingUser.id}`, body);
        Toast.success('用户已更新');
      } else {
        await api.post('/api/users', values);
        Toast.success('用户已创建');
      }
      setSideSheetVisible(false);
      loadUsers();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const handleToggleStatus = async (user: UserResponse) => {
    const newStatus = user.status === 'enabled' ? 'disabled' : 'enabled';
    try {
      await api.put(`/api/users/${user.id}`, {
        display_name: user.display_name,
        status: newStatus,
      });
      Toast.success(`用户已${newStatus === 'enabled' ? '启用' : '禁用'}`);
      loadUsers();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '操作失败');
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await api.delete(`/api/users/${id}`);
      Toast.success('用户已删除');
      loadUsers();
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '删除失败');
    }
  };

  const formatDate = (dateStr: string): string => {
    const date = new Date(dateStr);
    return date.toLocaleDateString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const columns = [
    { title: '用户名', dataIndex: 'username', key: 'username' },
    { title: '姓名', dataIndex: 'display_name', key: 'display_name', width: 140 },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (_: string, record: UserResponse) => (
        <Popconfirm
          title={`确认${record.status === 'enabled' ? '禁用' : '启用'}此用户?`}
          onConfirm={() => handleToggleStatus(record)}
          position="bottomRight"
        >
          <Tag color={record.status === 'enabled' ? 'green' : 'red'} style={{ cursor: 'pointer' }}>
            {record.status === 'enabled' ? '启用' : '禁用'}
          </Tag>
        </Popconfirm>
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
      title: '操作',
      key: 'actions',
      width: 160,
      render: (_: unknown, record: UserResponse) => (
        <Space>
          <Button size="small" onClick={() => openEditDrawer(record)}>编辑</Button>
          <Popconfirm
            title="确认删除此用户?"
            onConfirm={() => handleDelete(record.id)}
            position="bottomRight"
          >
            <Button
              size="small"
              type="danger"
              disabled={record.username === currentUsername}
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
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <Title heading={3}>用户管理</Title>
        <Button type="primary" onClick={openCreateDrawer}>创建用户</Button>
      </div>

      <Table
        columns={columns}
        dataSource={users}
        loading={loading}
        rowKey="id"
        scroll={{ x: 'max-content' }}
        pagination={{ pageSize: 20 }}
        empty={<Typography.Text type="secondary">暂无用户数据</Typography.Text>}
      />

      <SideSheet
        title={editingUser ? '编辑用户' : '创建用户'}
        visible={sideSheetVisible}
        onCancel={() => setSideSheetVisible(false)}
        width={480}
        maskClosable={false}
      >
        <Form
          onSubmit={handleSave}
          initValues={editingUser ? { username: editingUser.username, display_name: editingUser.display_name } : undefined}
          style={{ padding: '0 4px' }}
        >
          <Form.Input
            field="username"
            label="账号"
            placeholder="登录账号"
            disabled={!!editingUser}
            rules={[{ required: true, message: '请输入账号' }]}
          />
          <Form.Input
            field="display_name"
            label="姓名"
            placeholder="显示名称"
            rules={[{ required: true, message: '请输入姓名' }]}
          />
          <Form.Input
            field="password"
            label="密码"
            mode="password"
            placeholder={editingUser ? '留空则不修改密码' : '请输入密码'}
            rules={editingUser ? [] : [{ required: true, message: '请输入密码' }]}
          />
          <Button type="primary" htmlType="submit" loading={saving} block style={{ marginTop: 16 }}>
            {editingUser ? '更新' : '创建'}
          </Button>
        </Form>
      </SideSheet>
    </div>
  );
}

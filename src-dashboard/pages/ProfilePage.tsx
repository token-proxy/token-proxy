import { type ReactNode, useState } from 'react';
import { useFetch } from '../hooks/useFetch.ts';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Button, Card, Form, TabPane, Tabs, Toast, Typography } from '@douyinfe/semi-ui';
import api from '../api.ts';
import ApiKeyManager from '../components/user/ApiKeyManager.tsx';

const { Title, Text } = Typography;

/* ---- 类型定义 ---- */

interface UserProfile {
  id: string;
  username: string;
  display_name: string;
  status: string;
  created_at: string;
  updated_at: string;
}

/* ---- 组件 ---- */

/**
 * ProfilePage - 个人设置页面
 *
 * 包含个人资料编辑、密码修改、API Key 管理三个标签页。
 * 密码修改成功后清除本地令牌并跳转到登录页。
 * 通过 URL 查询参数 `tab` 控制激活的标签页（profile | password | apikey）。
 */
export default function ProfilePage(): ReactNode {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  // 从 URL 查询参数读取当前标签页，默认为 "profile"
  const activeTab = searchParams.get('tab') || 'profile';

  const handleTabChange = (key: string) => {
    setSearchParams({ tab: key });
  };

  /* ---- Profile 状态 ---- */
  const {
    data: profile,
    loading: profileLoading,
    refetch: loadProfile,
  } = useFetch(() => api.get<UserProfile>('/api/users/me'), []);
  const [profileSaving, setProfileSaving] = useState(false);

  /* ---- 密码状态 ---- */
  const [passwordSaving, setPasswordSaving] = useState(false);

  /* ---- 更新 Profile ---- */
  const handleSaveProfile = async (values: { display_name: string }) => {
    setProfileSaving(true);
    try {
      const data = await api.put<UserProfile>('/api/users/me/profile', values);
      loadProfile();
      localStorage.setItem('display_name', data.display_name);
      window.dispatchEvent(new Event('storage'));
      Toast.success('个人资料已更新');
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '保存失败');
    } finally {
      setProfileSaving(false);
    }
  };

  /* ---- 修改密码 ---- */
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
      await api.put('/api/users/me/password', {
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

  return (
    <div>
      <Title heading={3} style={{ marginBottom: 24 }}>
        个人设置
      </Title>

      <Card>
        <Tabs type="line" activeKey={activeTab} onChange={handleTabChange}>
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

          <TabPane tab="API Key" itemKey="apikey">
            <div style={{ marginTop: 16 }}>
              <ApiKeyManager />
            </div>
          </TabPane>
        </Tabs>
      </Card>
    </div>
  );
}

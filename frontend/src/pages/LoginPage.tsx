import type { ReactNode } from 'react';
import { Navigate, useNavigate } from 'react-router-dom';
import { Card, Form, Button, Toast, Typography } from '@douyinfe/semi-ui';

const { Title } = Typography;

interface LoginFormValues {
  username: string;
  password: string;
}

export default function LoginPage(): ReactNode {
  const navigate = useNavigate();
  const token = localStorage.getItem('access_token');

  if (token) {
    return <Navigate to="/dashboard" replace />;
  }

  const handleSubmit = async (values: LoginFormValues) => {
    try {
      const res = await fetch('/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(values),
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        Toast.error(err.error || '登录失败');
        return;
      }

      const data = await res.json();
      localStorage.setItem('access_token', data.access_token);
      localStorage.setItem('refresh_token', data.refresh_token);
      localStorage.setItem('username', data.username ?? values.username);
      localStorage.setItem('display_name', data.display_name ?? values.username);

      Toast.success('登录成功');
      navigate('/dashboard', { replace: true });
    } catch {
      Toast.error('网络错误');
    }
  };

  return (
    <div style={{
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      minHeight: '100vh',
      background: 'var(--semi-color-fill-0)',
    }}>
      <Card style={{ width: 400, padding: 24 }}>
        <Title heading={3} style={{ textAlign: 'center', marginBottom: 32 }}>
          Token Proxy
        </Title>
        <Form onSubmit={handleSubmit}>
          <Form.Input
            field="username"
            label="账号"
            placeholder="请输入账号"
            rules={[{ required: true, message: '请输入账号' }]}
          />
          <Form.Input
            field="password"
            label="密码"
            type="password"
            placeholder="请输入密码"
            rules={[{ required: true, message: '请输入密码' }]}
          />
          <Button type="primary" htmlType="submit" block style={{ marginTop: 16 }}>
            登录
          </Button>
        </Form>
      </Card>
    </div>
  );
}
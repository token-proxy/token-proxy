import { useEffect, useState, type ReactNode } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { Layout, Nav, Dropdown } from '@douyinfe/semi-ui';
import { IconUserCircle } from '@douyinfe/semi-icons';
import ThemeToggle from '../components/ThemeToggle';

const { Header, Sider, Content } = Layout;

interface NavItem {
  itemKey: string;
  text: string;
}

const navItems: NavItem[] = [
  { itemKey: '/dashboard', text: 'Dashboard' },
  { itemKey: '/providers', text: 'Provider 管理' },
  { itemKey: '/access-points', text: '接入点管理' },
  { itemKey: '/sessions', text: '会话日志' },
  { itemKey: '/logs', text: '请求日志' },
  { itemKey: '/users', text: '用户管理' },
  { itemKey: '/settings', text: '系统设置' },
];

export default function AdminLayout(): ReactNode {
  const navigate = useNavigate();
  const location = useLocation();
  const [selectedKeys, setSelectedKeys] = useState([location.pathname.replace(/\/$/, '') || '/dashboard']);
  const [displayName, setDisplayName] = useState(
    localStorage.getItem('display_name') || localStorage.getItem('username') || '管理员',
  );

  useEffect(() => {
    const syncDisplayName = () => {
      setDisplayName(localStorage.getItem('display_name') || localStorage.getItem('username') || '管理员');
    };
    window.addEventListener('storage', syncDisplayName);
    return () => window.removeEventListener('storage', syncDisplayName);
  }, []);

  const handleLogout = () => {
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    localStorage.removeItem('username');
    localStorage.removeItem('display_name');
    navigate('/login');
  };

  return (
    <Layout style={{ height: '100vh' }}>
      <Sider style={{ backgroundColor: 'var(--semi-color-bg-1)' }}>
        <Nav
          style={{ maxWidth: 220, height: '100%' }}
          selectedKeys={selectedKeys}
          onSelect={({ itemKey }) => {
            setSelectedKeys([itemKey as string]);
            navigate(itemKey as string);
          }}
          items={navItems}
          header={{
            text: 'Token Proxy',
            style: { fontSize: 18, fontWeight: 'bold', padding: '12px 16px' },
          }}
          footer={{
            collapseButton: true,
          }}
        />
      </Sider>
      <Layout>
        <Header style={{
          height: 64,
          minHeight: 64,
          backgroundColor: 'var(--semi-color-bg-1)',
          borderBottom: '1px solid var(--semi-color-border)',
          padding: '0 24px',
          display: 'flex',
          justifyContent: 'flex-end',
          alignItems: 'center',
          gap: 12,
          boxSizing: 'border-box',
        }}>
          <ThemeToggle />
          <Dropdown
            clickToHide
            render={
              <Dropdown.Menu>
                <Dropdown.Item onClick={() => navigate('/settings/profile')}>
                  个人设置
                </Dropdown.Item>
                <Dropdown.Divider />
                <Dropdown.Item onClick={handleLogout}>
                  退出登录
                </Dropdown.Item>
              </Dropdown.Menu>
            }
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, cursor: 'pointer' }}>
              <IconUserCircle size="large" />
              <span>{displayName}</span>
            </div>
          </Dropdown>
        </Header>
        <Content style={{ padding: 24, overflow: 'auto' }}>
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  );
}
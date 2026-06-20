import { type ReactNode, useEffect, useState } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import { Dropdown, Layout, Nav } from '@douyinfe/semi-ui';
import { IconHomeStroked, IconServerStroked, IconRoute, IconCommentStroked, IconListView, IconUserGroup, IconSettingStroked, IconUserCircle } from '@douyinfe/semi-icons';
import ThemeToggle from '@components/common/ThemeToggle';

const {Header, Sider, Content} = Layout;

interface NavItem {
  itemKey: string;
  text: string;
  icon?: ReactNode;
}

const navItems: NavItem[] = [
  {itemKey: '/dashboard', text: 'Dashboard', icon: <IconHomeStroked />},
  {itemKey: '/providers', text: '服务商管理', icon: <IconServerStroked />},
  {itemKey: '/access-points', text: '接入点管理', icon: <IconRoute />},
  {itemKey: '/sessions', text: '会话日志', icon: <IconCommentStroked />},
  {itemKey: '/logs', text: '请求日志', icon: <IconListView />},
  {itemKey: '/users', text: '用户管理', icon: <IconUserGroup />},
  {itemKey: '/settings', text: '系统设置', icon: <IconSettingStroked />},
];

// 匹配详情页路径：logs/:id 或 sessions/:sessionId
const DETAIL_PAGE_PATTERN = /^\/(logs|sessions)\/[^/]+$/;

/**
 * AdminLayout - 管理后台布局组件
 *
 * 包含侧边栏导航（Semi Nav）和顶部 Header（主题切换 + 用户菜单）。
 * 详情页（logs/:id, sessions/:sessionId）自动收起侧边栏以便展示更多内容。
 */
export default function AdminLayout(): ReactNode {
  const navigate = useNavigate();
  const location = useLocation();
  const [selectedKeys, setSelectedKeys] = useState(() => {
    const path = location.pathname.replace(/\/$/, '') || '/dashboard';
    return DETAIL_PAGE_PATTERN.test(path) ? [] : [path];
  });
  const [isCollapsed, setIsCollapsed] = useState(
    DETAIL_PAGE_PATTERN.test(location.pathname.replace(/\/$/, '')),
  );
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

  useEffect(() => {
    const currentPath = location.pathname.replace(/\/$/, '') || '/dashboard';

    // 详情页（logs/:id / sessions/:sessionId）：自动收起侧边栏，不激活任何导航项
    if (DETAIL_PAGE_PATTERN.test(currentPath)) {
      setIsCollapsed(true);
      setSelectedKeys([]);
      return;
    }

    // 列表页或其他页面：匹配导航高亮，不干预折叠状态（保留用户偏好）
    const matchingItem = [...navItems]
      .sort((a, b) => b.itemKey.length - a.itemKey.length)
      .find(item => currentPath === item.itemKey || currentPath.startsWith(item.itemKey + '/'));
    if (matchingItem) {
      setSelectedKeys([matchingItem.itemKey]);
    }
  }, [location.pathname]);

  const handleLogout = () => {
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    localStorage.removeItem('username');
    localStorage.removeItem('display_name');
    navigate('/login');
  };

  return (
    <Layout style={{height: '100vh'}}>
      <Sider style={{backgroundColor: 'var(--semi-color-bg-1)'}}>
        <Nav
          style={{maxWidth: 220, height: '100%'}}
          isCollapsed={isCollapsed}
          selectedKeys={selectedKeys}
          onCollapseChange={setIsCollapsed}
          onSelect={({itemKey}) => {
            setSelectedKeys([itemKey as string]);
            navigate(itemKey as string);
          }}
          items={navItems}
          header={{
            text: 'Token Proxy',
            style: {fontSize: 18, fontWeight: 'bold', padding: '12px 16px'},
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
          <ThemeToggle/>
          <Dropdown
            clickToHide
            render={
              <Dropdown.Menu>
                <Dropdown.Item onClick={() => navigate('/settings/profile')}>
                  个人设置
                </Dropdown.Item>
                <Dropdown.Divider/>
                <Dropdown.Item onClick={handleLogout}>
                  退出登录
                </Dropdown.Item>
              </Dropdown.Menu>
            }
          >
            <div style={{display: 'flex', alignItems: 'center', gap: 8, cursor: 'pointer'}}>
              <IconUserCircle size="large"/>
              <span>{displayName}</span>
            </div>
          </Dropdown>
        </Header>
        <Content style={{padding: 24}}>
          <Outlet/>
        </Content>
      </Layout>
    </Layout>
  );
}
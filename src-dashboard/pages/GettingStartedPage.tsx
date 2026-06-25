/**
 * 开始使用页面（个人工作台）。
 *
 * 顶层编排组件：聚合三个独立封装卡片，仅负责 `refreshKey` 状态管理与组件装配。
 * 数据获取、错误处理、加载态均由各卡片组件自包含。
 */

import { type ReactNode, useCallback, useState } from 'react';
import { Button, Typography } from '@douyinfe/semi-ui';
import { IconKey } from '@douyinfe/semi-icons';
import { useNavigate } from 'react-router-dom';
import { UsageOverviewCard } from '@components/getting-started/UsageOverviewCard';
import { MetricsCard } from '@components/getting-started/MetricsCard';
import { MyAccessPointsSection } from '../components/access-point/MyAccessPointsSection';
import './GettingStartedPage.css';

/**
 * 开始使用主页面组件。
 *
 * 仅持有 `refreshKey` 状态用于跨卡片统一刷新：
 * - `UsageOverviewCard` 接收 refreshKey 驱动热力图重取
 * - `MetricsCard` 接收 refreshKey + onRefresh 回调驱动 KPI/质量重取
 * - `MyAccessPointsSection` 独立管理自身数据
 */
export default function GettingStartedPage(): ReactNode {
  const navigate = useNavigate();
  const [refreshKey, setRefreshKey] = useState(0);

  const handleRefresh = useCallback(() => {
    setRefreshKey((k) => k + 1);
  }, []);

  return (
    <div className="getting-started-container">
      {/* 顶部：标题 */}
      <div className="getting-started-header">
        <Typography.Title heading={3} style={{ margin: 0 }}>
          开始使用
        </Typography.Title>
      </div>

      {/* 第一行：用量总览 + 数据指标 50/50 */}
      <div className="gs-hero-grid">
        <UsageOverviewCard refreshKey={refreshKey} />
        <MetricsCard refreshKey={refreshKey} onRefresh={handleRefresh} />
      </div>

      {/* 第二行：我的接入点 */}
      <MyAccessPointsSection
        extraHeaderContent={
          <Button size="small" icon={<IconKey />} onClick={() => navigate('/profile')}>
            API Key
          </Button>
        }
      />
    </div>
  );
}

/**
 * 开始使用页面（个人工作台）。
 *
 * 顶层编排组件：聚合三个独立封装卡片，每个卡片拥有自己的刷新按钮和生命周期，
 * 互不影响。数据获取、错误处理、加载态均由各卡片组件自包含。
 */

import { type ReactNode } from 'react';
import { Typography } from '@douyinfe/semi-ui';
import { IconKey } from '@douyinfe/semi-icons';
import { useNavigate } from 'react-router-dom';
import { UsageOverviewCard } from '@components/getting-started/UsageOverviewCard';
import { MetricsCard } from '@components/getting-started/MetricsCard';
import { UsageTrendsCard } from '@components/getting-started/UsageTrendsCard';
import { MyAccessPointsSection } from '../components/access-point/MyAccessPointsSection';
import './GettingStartedPage.css';

/**
 * 开始使用主页面组件。
 *
 * 三个卡片各有独立的刷新按钮和数据生命周期，互不影响。
 */

export default function GettingStartedPage(): ReactNode {
  const navigate = useNavigate();

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
        <UsageOverviewCard />
        <MetricsCard />
      </div>

      {/* 第二行：用量趋势 */}
      <UsageTrendsCard />

      {/* 第三行：我的接入点 */}
      <MyAccessPointsSection
        extraHeaderContent={
          <span
            className="gs-api-key-link"
            onClick={() => navigate('/profile?tab=apikey')}
            role="link"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === 'Enter') navigate('/profile?tab=apikey');
            }}
          >
            <IconKey style={{ marginRight: 4 }} />
            配置 API Key
          </span>
        }
      />
    </div>
  );
}

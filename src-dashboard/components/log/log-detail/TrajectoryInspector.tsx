import { type ReactNode } from 'react';
import { Tag, Tabs, Typography } from '@douyinfe/semi-ui';
import { IconClose } from '@douyinfe/semi-icons';
import CodeHighlight from '@components/common/CodeHighlight';
import type { TrajectoryCell, TrajectoryTab } from '../../../utils/trajectory.ts';

const { Text } = Typography;
const { TabPane } = Tabs;

/** 选中记录在账本中的位置 */
export interface TrajectoryLocation {
  /** 轮次标题，如「第 2 轮」 */
  turnTitle: string;
  /** 分组标题，如「第 3 步」 */
  groupTitle: string;
}

/** TrajectoryInspector 组件 Props */
interface TrajectoryInspectorProps {
  /** 选中的记录 */
  cell: TrajectoryCell;
  /** 记录在账本中的位置 */
  location: TrajectoryLocation;
  /** 关闭检查器 */
  onClose: () => void;
}

/**
 * TrajectoryInspector - 记录检查器
 *
 * 与 DSH 轨迹的本地检查器同构：标题栏给出记录类型、位置（第 N 轮 · 第 k 步）与序号，
 * 页签由记录自身声明（概述 / 输出 / 思考 / 参数 / 结果 / 请求头 / 响应头 / 计时 / 原始内容），
 * 内容按页签形态渲染为键值列表、JSON 高亮或等宽原文。
 */
export default function TrajectoryInspector({
  cell,
  location,
  onClose,
}: TrajectoryInspectorProps): ReactNode {
  return (
    <aside className="trajectory-inspector">
      {/* ─── 标题栏 ─── */}
      <header className="trajectory-inspector__head">
        <Tag size="small" color="blue">
          {cell.kindLabel}
        </Tag>
        <span className="trajectory-inspector__title">{cell.text || '（无内容）'}</span>
        <span className="trajectory-inspector__spacer" />
        <button
          type="button"
          className="trajectory-inspector__close"
          onClick={onClose}
          title="关闭检查器"
        >
          <IconClose />
        </button>
      </header>

      {/* ─── 位置与状态 ─── */}
      <div className="trajectory-inspector__sub">
        <Text size="small" type="tertiary">
          {location.turnTitle} · {location.groupTitle}
        </Text>
        <span className="trajectory-inspector__spacer" />
        {cell.status === 'failed' && (
          <Tag size="small" color="red">
            失败
          </Tag>
        )}
        {cell.status === 'pending' && (
          <Tag size="small" color="amber">
            等待中
          </Tag>
        )}
        <Text size="small" type="tertiary">
          #{cell.index}
        </Text>
      </div>

      {/* ─── 页签（collapsible=false：页签多时横向滚动，不折叠成更多菜单） ─── */}
      <Tabs
        type="line"
        size="small"
        collapsible={false}
        className="trajectory-inspector__tabs"
        key={cell.index}
      >
        {cell.tabs.map((tab) => (
          <TabPane itemKey={tab.key} tab={tab.label} key={tab.key}>
            <TabContent tab={tab} />
          </TabPane>
        ))}
      </Tabs>
    </aside>
  );
}

/** 页签内容按形态分发 */
function TabContent({ tab }: { tab: TrajectoryTab }): ReactNode {
  if (tab.kind === 'meta') {
    const items = tab.items ?? [];
    if (items.length === 0) return <Text type="tertiary">无信息</Text>;

    return (
      <dl className="trajectory-meta">
        {items.map((item) => (
          <div className="trajectory-meta__row" key={item.label}>
            <dt className="trajectory-meta__label">{item.label}</dt>
            <dd className="trajectory-meta__value" data-mono={item.mono ? 'true' : 'false'}>
              {item.value}
            </dd>
          </div>
        ))}
      </dl>
    );
  }

  const value = tab.value ?? '';
  if (!value.trim()) return <Text type="tertiary">未捕获内容</Text>;

  if (tab.kind === 'json') {
    return <CodeHighlight content={value} />;
  }

  return <pre className="trajectory-pre">{value}</pre>;
}

import { type ReactNode, useCallback, useState } from 'react';
import { Button } from '@douyinfe/semi-ui';
import { IconChevronDown, IconChevronUp } from '@douyinfe/semi-icons';
import MarkdownRender from '@components/common/MarkdownRender';

/** 超过此长度（字符数）视为长内容，默认自动收起 */
const LONG_CONTENT_THRESHOLD = 2000;

interface ExpandableContentBlockProps {
  /** 要渲染的 Markdown 文本内容 */
  content: string;
  /**
   * 受控：展开状态。
   * 传入时组件为受控模式，展开/收起完全由父级决定。
   * 不传入时组件内部自管理状态。
   */
  expanded?: boolean;
  /**
   * 非受控模式下的默认展开状态。
   * 不传时根据内容长度自动判断：< 500 字默认展开，≥ 500 字默认收起。
   */
  defaultExpanded?: boolean;
  /** 展开状态变化回调（受控与非受控模式均触发） */
  onExpandedChange?: (expanded: boolean) => void;
  /** 收起时的最小可见高度（px），默认 200 */
  collapsedHeight?: number;
  /** 黏滞收起按钮文案，默认 "收起内容" */
  collapseLabel?: string;
}

/**
 * 可展开/收起的长文本内容区块
 *
 * - 接收 Markdown 文本字符串，内部使用 MarkdownRender 渲染
 * - 自动判断内容长度：< 500 字直接全部渲染，≥ 500 字可展开/收起
 * - 受控 / 非受控双模式：expanded 不传时内部自管理，传入时由父级控制
 * - 收起时：显示 collapsedHeight 高度的预览 + "展开" 按钮
 * - 展开时：完整内容 + 视口底部黏滞"收起"按钮
 */
export default function ExpandableContentBlock({
  content,
  expanded: controlledExpanded,
  defaultExpanded,
  onExpandedChange,
  collapsedHeight = 100,
  collapseLabel = '收起内容',
}: ExpandableContentBlockProps): ReactNode {
  const isLong = content.length >= LONG_CONTENT_THRESHOLD;

  // 自动推断默认展开状态：用户指定 > 自动按长度判断
  const resolvedDefault = defaultExpanded ?? !isLong;

  const [internalExpanded, setInternalExpanded] = useState(resolvedDefault);

  const isControlled = controlledExpanded !== undefined;
  const expanded = isControlled ? controlledExpanded : internalExpanded;

  const expand = useCallback(() => {
    if (!isControlled) setInternalExpanded(true);
    onExpandedChange?.(true);
  }, [isControlled, onExpandedChange]);

  const collapse = useCallback(() => {
    if (!isControlled) setInternalExpanded(false);
    onExpandedChange?.(false);
  }, [isControlled, onExpandedChange]);

  // ── 短内容：直接全部渲染 ──
  if (!isLong) {
    return <MarkdownRender content={content}/>;
  }

  // ── 长内容：CSS 切换收起/展开态 ──
  return (
    <div>
      <div style={{
        maxHeight: expanded ? undefined : collapsedHeight,
        overflow: expanded ? undefined : 'hidden',
      }}>
        <MarkdownRender content={content}/>
      </div>
      <div
        style={{
          position: expanded ? 'sticky' : undefined,
          // bottom: -20 让按钮推入 Card body 的 20px padding-bottom 区域，贴死屏幕下边缘
          bottom: 0,
          textAlign: 'center',
          paddingTop: 12,
          zIndex: 10,
          background: expanded
            ? 'linear-gradient(transparent, var(--semi-color-bg-0) 60%)'
            : undefined,
        }}
      >
        <Button
          type="tertiary"
          size="small"
          icon={expanded ? <IconChevronUp/> : <IconChevronDown/>}
          onClick={expanded ? collapse : expand}
        >
          {expanded ? collapseLabel : '展开'}
        </Button>
      </div>
    </div>
  );
}

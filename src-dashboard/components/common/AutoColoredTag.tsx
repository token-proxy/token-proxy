import { type ReactElement } from 'react';
import { Tag } from '@douyinfe/semi-ui';

// ─── 类型定义 ────────────────────────────────────────────

/**
 * Semi Design Tag 组件的颜色值。
 *
 * 与 `@douyinfe/semi-ui` Tag 的 `color` 属性保持一致。
 * 参考 Semi Design 文档: https://semi.design/zh-CN/show/tag
 */
type TagColor =
  | 'amber'
  | 'blue'
  | 'cyan'
  | 'green'
  | 'grey'
  | 'indigo'
  | 'light-blue'
  | 'light-green'
  | 'lime'
  | 'orange'
  | 'pink'
  | 'purple'
  | 'red'
  | 'teal'
  | 'violet'
  | 'yellow'
  | 'white';

// ─── 自动着色调色板 ───────────────────────────────────────

/**
 * 自动颜色映射使用的调色板。
 *
 * 从 Semi Design 17 种 TagColor 中选取 12 种视觉区分度高的颜色，
 * 排除 `white`、`grey`（过于中性，看起来像未着色）和相近的 `light-green`。
 * 颜色顺序经过刻意打乱，使哈希相邻的值映射到视觉差异大的颜色。
 */
const AUTO_COLOR_PALETTE: readonly TagColor[] = [
  'blue',
  'green',
  'purple',
  'orange',
  'teal',
  'pink',
  'indigo',
  'cyan',
  'amber',
  'violet',
  'lime',
  'light-blue',
] as const;

// ─── 哈希算法 ────────────────────────────────────────────

/**
 * 基于 DJB2 算法将任意字符串映射为调色板中的确定性颜色。
 *
 * DJB2 是一种简单高效的字符串哈希算法，具有均匀分布特性。
 * 保证相同输入永远返回相同颜色，确保跨组件的一致用户体验。
 *
 * @param value - 用于颜色计算的字符串
 * @returns 调色板中的一种 Semi Design TagColor
 */
function hashToColor(value: string): TagColor {
  let hash = 5381;
  for (let i = 0; i < value.length; i++) {
    // hash * 33 + charCode，使用 32 位整数运算
    hash = ((hash << 5) + hash + value.charCodeAt(i)) | 0;
  }
  const index = Math.abs(hash) % AUTO_COLOR_PALETTE.length;
  return AUTO_COLOR_PALETTE[index];
}

// ─── 组件接口 ────────────────────────────────────────────

/**
 * AutoColoredTag 组件属性。
 *
 * 继承 Semi Design `Tag` 组件的全部属性，在此基础上扩展自动着色能力。
 * 不破坏原组件的 API —— 所有 Tag 原生属性均可透传。
 */
export interface AutoColoredTagProps extends React.ComponentProps<typeof Tag> {
  /**
   * 用于颜色计算的键值。
   *
   * 当标签显示文本与颜色计算依据不同时使用。
   * 例如：显示 "已启用" 但希望颜色基于 "enabled" 这个英文标识。
   *
   * 未指定时自动回退为 `children` 的字符串值。
   */
  colorKey?: string;
}

// ─── 组件实现 ────────────────────────────────────────────

/**
 * 自动着色标签组件。
 *
 * 封装 Semi Design `Tag`，基于内容值自动计算确定性的颜色，
 * 同时保留手动覆盖 `color` 属性的能力。
 *
 * ## 颜色解析优先级
 *
 * 1. 显式指定 `color` 属性 → 直接使用（完全手动控制）
 * 2. 指定 `colorKey` 属性 → 对 `colorKey` 哈希取色
 * 3. `children` 为字符串 → 对 `children` 哈希取色
 * 4. 以上皆不满足 → 回退到 Semi Design 默认色（`grey`）
 *
 * ## 使用示例
 *
 * ```tsx
 * // 自动着色：基于 "用户" 计算颜色
 * <AutoColoredTag>用户</AutoColoredTag>
 *
 * // 颜色键分离：显示 "已启用"，颜色基于 "enabled"
 * <AutoColoredTag colorKey="enabled">已启用</AutoColoredTag>
 *
 * // 手动覆盖
 * <AutoColoredTag color="red">错误</AutoColoredTag>
 *
 * // 透传 Semi Tag 其他属性
 * <AutoColoredTag size="large" shape="circle" type="solid">重要</AutoColoredTag>
 * ```
 */
export function AutoColoredTag({
  color,
  colorKey,
  children,
  ...restProps
}: AutoColoredTagProps): ReactElement {
  // 解析最终颜色：手动指定 > colorKey 哈希 > children 字符串哈希 > undefined（Semi 默认）
  const resolvedColor: TagColor | undefined =
    color ??
    (() => {
      const key = colorKey ?? (typeof children === 'string' ? children : '');
      return key ? hashToColor(key) : undefined;
    })();

  return (
    <Tag color={resolvedColor} {...restProps}>
      {children}
    </Tag>
  );
}

export default AutoColoredTag;

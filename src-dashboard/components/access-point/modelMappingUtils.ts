import { UNMATCHED_MODEL } from '../../types/accessPoint.ts';

/** Anthropic 模型族预设列表 */
export const ANTHROPIC_FAMILIES = [
  { label: '未匹配', value: UNMATCHED_MODEL, matchType: 'prefix' },
  { label: 'Claude Opus', value: 'claude-opus-', matchType: 'prefix' },
  { label: 'Claude Sonnet', value: 'claude-sonnet-', matchType: 'prefix' },
  { label: 'Claude Haiku', value: 'claude-haiku-', matchType: 'prefix' },
];

/** 映射匹配类型：精确匹配或前缀模式匹配 */
export type MappingMatchType = 'exact' | 'prefix';

/** 根据来源模型值推断匹配类型 */
export function matchTypeForSource(value: string): MappingMatchType {
  return (
    (ANTHROPIC_FAMILIES.find((family) => family.value === value)?.matchType as
      | MappingMatchType
      | undefined) ?? 'exact'
  );
}

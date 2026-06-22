import { Button, Select, Tag } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import { type ModelMapping, UNMATCHED_MODEL } from '../../types/accessPoint.ts';
import {
  ANTHROPIC_FAMILIES,
  type MappingMatchType,
  matchTypeForSource,
} from './modelMappingUtils.ts';

/** ModelMappingEditor 组件 Props */
interface ModelMappingEditorProps {
  mappings: ModelMapping[];
  apiType?: string;
  modelOptions: string[];
  onAdd: () => void;
  onRemove: (index: number) => void;
  onChange: (index: number, field: keyof ModelMapping, value: string) => void;
}

const MATCH_TYPE_LABELS: Record<MappingMatchType, string> = {
  exact: '精准匹配',
  prefix: '模式匹配',
};

const labelForModel = (value: string) => {
  if (value === UNMATCHED_MODEL) return '未匹配';
  return ANTHROPIC_FAMILIES.find((family) => family.value === value)?.label ?? value;
};

const uniqueOptions = (values: string[]) => [...new Set(values.filter(Boolean))];

const optionLabel = (type: MappingMatchType, value: string) => (
  <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
    <Tag color={type === 'prefix' ? 'purple' : 'blue'} size="small">
      {MATCH_TYPE_LABELS[type]}
    </Tag>
    <span>{labelForModel(value)}</span>
  </span>
);

/**
 * ModelMappingEditor - 模型映射编辑器组件
 *
 * 提供源模型到目标模型的映射编辑，支持 Anthropic 模型族预设。
 */
export default function ModelMappingEditor({
  mappings,
  apiType,
  modelOptions,
  onAdd,
  onRemove,
  onChange,
}: ModelMappingEditorProps): ReactNode {
  const isAnthropicApi = apiType?.toLowerCase() === 'anthropic';
  const sourceValues = uniqueOptions([
    ...(isAnthropicApi ? ANTHROPIC_FAMILIES.map((family) => family.value) : []),
    ...modelOptions,
    ...mappings.map((mapping) => mapping.source_model),
  ]);
  const targetValues = uniqueOptions([...modelOptions]);

  const sourceOptionList = sourceValues.map((value) => {
    const matchType = matchTypeForSource(value);
    return {
      value,
      label: optionLabel(matchType, value),
    };
  });
  const targetOptionList = targetValues.map((value) => ({
    value,
    label: optionLabel('exact', value),
  }));

  return (
    <div style={{ marginTop: 24 }}>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 8,
        }}
      >
        <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--semi-color-text-0)' }}>
          模型映射
        </span>
        <Button size="small" onClick={onAdd}>
          添加映射
        </Button>
      </div>
      {mappings.length === 0 && (
        <div style={{ color: 'var(--semi-color-text-2)', fontSize: 13, padding: '8px 0' }}>
          暂无映射规则，点击 "添加映射" 新增
        </div>
      )}
      {mappings.map((mapping, index) => (
        <div key={index} style={{ display: 'flex', gap: 8, marginBottom: 8, alignItems: 'center' }}>
          <Select
            key={`source-${sourceValues.join('|')}`}
            value={mapping.source_model || undefined}
            placeholder="源模型"
            filter
            allowCreate
            optionList={sourceOptionList}
            style={{ flex: 1 }}
            onChange={(value) => onChange(index, 'source_model', value as string)}
          />
          <span style={{ color: 'var(--semi-color-text-2)' }}>→</span>
          <Select
            key={`target-${targetValues.join('|')}`}
            value={mapping.target_model || undefined}
            placeholder="目标模型"
            filter
            optionList={targetOptionList}
            style={{ flex: 1 }}
            onChange={(value) => onChange(index, 'target_model', value as string)}
          />
          <Button type="danger" icon={null} onClick={() => onRemove(index)} size="small">
            删除
          </Button>
        </div>
      ))}
    </div>
  );
}

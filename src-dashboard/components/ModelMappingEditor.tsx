import { Button, Input } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { ModelMapping } from '../types/accessPoint.ts';

interface ModelMappingEditorProps {
  mappings: ModelMapping[];
  onAdd: () => void;
  onRemove: (index: number) => void;
  onChange: (index: number, field: keyof ModelMapping, value: string) => void;
}

export default function ModelMappingEditor({
  mappings,
  onAdd,
  onRemove,
  onChange,
}: ModelMappingEditorProps): ReactNode {
  return (
    <div style={{ marginTop: 24 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
        <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--semi-color-text-0)' }}>模型映射</span>
        <Button size="small" onClick={onAdd}>添加映射</Button>
      </div>
      {mappings.length === 0 && (
        <div style={{ color: 'var(--semi-color-text-2)', fontSize: 13, padding: '8px 0' }}>
          暂无映射规则，点击 "添加映射" 新增
        </div>
      )}
      {mappings.map((mapping, index) => (
        <div key={index} style={{ display: 'flex', gap: 8, marginBottom: 8, alignItems: 'center' }}>
          <Input
            value={mapping.source_model}
            onChange={(value: string) => onChange(index, 'source_model', value)}
            placeholder="源模型"
          />
          <span style={{ color: 'var(--semi-color-text-2)' }}>→</span>
          <Input
            value={mapping.target_model}
            onChange={(value: string) => onChange(index, 'target_model', value)}
            placeholder="目标模型"
          />
          <Button type="danger" icon={null} onClick={() => onRemove(index)} size="small">删除</Button>
        </div>
      ))}
    </div>
  );
}

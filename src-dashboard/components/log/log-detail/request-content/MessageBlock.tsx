import { type ReactNode } from 'react';
import { Descriptions, Tag } from '@douyinfe/semi-ui';
import ExpandableContentBlock from '@components/common/ExpandableContentBlock';
import CodeHighlight from '@components/common/CodeHighlight';

interface MessageBlockProps {
  block: Record<string, unknown>;
  role: string;
}

/** 单个消息内容块的独立卡片，类似 SystemPromptBlock */
export default function MessageBlock({
  block,
  role,
}: MessageBlockProps): ReactNode {
  const blockType = String(block.type ?? '');
  const roleLabel = role === 'user' ? '用户' : role === 'assistant' ? '助手' : role;
  const roleColor =
    role === 'user' ? 'blue' :
      role === 'assistant' ? 'green' :
        role === 'system' ? 'grey' : undefined;

  const cardStyle: React.CSSProperties = {
    marginBottom: 12,
    padding: 12,
    border: '1px solid var(--semi-color-border)',
    borderRadius: 6,
  };

  const headerStyle: React.CSSProperties = {
    marginBottom: 8,
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    flexWrap: 'wrap',
  };

  // ── 文本块 ──
  if (blockType === 'text') {
    const text = String(block.text ?? '');
    if (!text) return null;
    return (
      <div style={cardStyle}>
        <div style={headerStyle}>
          <Tag size="small" color={roleColor}>{roleLabel}</Tag>
          <Tag size="small" type="light">文本</Tag>
        </div>
        <ExpandableContentBlock content={text} collapseLabel="收起文本"/>
      </div>
    );
  }

  // ── 思考块 ──
  if (blockType === 'thinking') {
    const thought = String(block.thinking ?? '');
    if (!thought) return null;
    return (
      <div style={cardStyle}>
        <div style={headerStyle}>
          <Tag size="small" color={roleColor}>{roleLabel}</Tag>
          <Tag size="small" type="light">思考</Tag>
        </div>
        <ExpandableContentBlock content={thought} collapseLabel="收起思考"/>
      </div>
    );
  }

  // ── 工具调用块 ──
  if (blockType === 'tool_use') {
    const toolName = String(block.name ?? '');
    const input = block.input as Record<string, unknown> | undefined;
    return (
      <div style={cardStyle}>
        <div style={headerStyle}>
          <Tag size="small" color={roleColor}>{roleLabel}</Tag>
          <Tag size="small" color="orange">工具调用: {toolName}</Tag>
        </div>
        {input && Object.keys(input).length > 0 && (
          <Descriptions
            row
            size="small"
            data={Object.entries(input).map(([k, v]) => ({
              key: k,
              value: typeof v === 'string' ? v : JSON.stringify(v),
            }))}
          />
        )}
      </div>
    );
  }

  // ── 工具结果块 ──
  if (blockType === 'tool_result') {
    const toolUseId = String(block.tool_use_id ?? '');
    const rawContent = block.content;

    // 提取纯文本：支持 string / 内容块数组 / object 三种形态
    let textContent = '';
    if (typeof rawContent === 'string') {
      textContent = rawContent;
    } else if (Array.isArray(rawContent)) {
      textContent = (rawContent as Array<Record<string, unknown>>)
        .map((c) =>
          c.type === 'text' && typeof c.text === 'string' ? c.text : '',
        )
        .filter(Boolean)
        .join('\n');
    } else if (rawContent && typeof rawContent === 'object') {
      textContent = JSON.stringify(rawContent, null, 2);
    }

    // 自动检测 JSON 字符串：解析后重新格式化，确保缩进一致
    let language: string | undefined;
    if (textContent) {
      try {
        const parsed = JSON.parse(textContent);
        if (typeof parsed === 'object' && parsed !== null) {
          language = 'json';
          textContent = JSON.stringify(parsed, null, 2);
        }
      } catch {
        // 非 JSON 文本，保持原样，不指定语言
      }
    }

    return (
      <div style={cardStyle}>
        <div style={headerStyle}>
          <Tag size="small" color={roleColor}>{roleLabel}</Tag>
          <Tag size="small" color="green">
            工具结果{toolUseId ? ` (${toolUseId})` : ''}
          </Tag>
        </div>
        {textContent ? (
          <div style={{maxHeight: 400, overflow: 'auto'}}>
            <CodeHighlight
              content={textContent}
              language={language}
            />
          </div>
        ) : (
          <div style={{color: 'var(--semi-color-text-2)', fontSize: 13}}>
            (空结果)
          </div>
        )}
      </div>
    );
  }

  return null;
}

import { type ReactNode } from 'react';
import { CodeHighlight as SemiCodeHighlight } from '@douyinfe/semi-ui';
import 'prismjs/components/prism-json.js';

/** CodeHighlight 组件 Props */
interface CodeHighlightProps {
  content: string;
  language?: string;
}

/**
 * CodeHighlight - 代码高亮展示组件
 *
 * 封装 Semi CodeHighlight，预置 JSON 语法支持。
 */
export default function CodeHighlight({
  content,
  language = 'json',
}: CodeHighlightProps): ReactNode {
  return (
    <div className="code-highlight-wrapper">
      <SemiCodeHighlight
        code={content}
        language={language}
        lineNumber
        style={{margin: 0}}
      />
    </div>
  );
}

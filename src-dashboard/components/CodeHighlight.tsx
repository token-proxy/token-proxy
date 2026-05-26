import { type ReactNode } from 'react';
import { CodeHighlight as SemiCodeHighlight } from '@douyinfe/semi-ui';
import 'prismjs/components/prism-json.js';

interface CodeHighlightProps {
  content: string;
  language?: string;
}

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
        style={{ margin: 0 }}
      />
    </div>
  );
}

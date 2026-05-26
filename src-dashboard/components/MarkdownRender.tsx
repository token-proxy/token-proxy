import { type ReactNode } from 'react';
import { MarkdownRender as SemiMarkdownRender } from '@douyinfe/semi-ui';
import { cleanXmlTags } from '../utils/parseLogs.ts';

interface MarkdownRenderProps {
  content: string;
}

export default function MarkdownRender({
  content,
}: MarkdownRenderProps): ReactNode {
  const cleaned = cleanXmlTags(content);

  return (
    <div className="markdown-render">
      <SemiMarkdownRender
        raw={cleaned}
        format="md"
        components={{
          pre: ({ children }) => (
            <pre className="markdown-code-block">{children}</pre>
          ),
        }}
      />
    </div>
  );
}

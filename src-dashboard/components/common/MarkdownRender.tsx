import { type ReactNode } from 'react';
import { MarkdownRender as SemiMarkdownRender } from '@douyinfe/semi-ui';
import { cleanXmlTags } from '../../utils/parseLogs.ts';

/** MarkdownRender 组件 Props */
interface MarkdownRenderProps {
  content: string;
}

/**
 * MarkdownRender - Markdown 渲染组件
 *
 * 封装 Semi MarkdownRender，预处理 XML 标签清洗。
 */
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
          pre: ({children}) => (
            <pre className="markdown-code-block">{children}</pre>
          ),
        }}
      />
    </div>
  );
}

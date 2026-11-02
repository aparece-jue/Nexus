import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeSanitize from 'rehype-sanitize';

export function MarkdownPreview({ text }: { text: string }) {
  return <Markdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSanitize]} skipHtml
    components={{
      img: () => <span>图片暂未授权加载</span>,
      a: ({ children }) => <span>{children}（链接仅展示）</span>,
    }}>{text}</Markdown>;
}

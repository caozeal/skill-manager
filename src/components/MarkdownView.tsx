import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";

export function MarkdownView({ md }: { md: string }) {
  return (
    <div className="md-body text-[13px] text-gray-800">
      <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeHighlight]}>
        {md || "（空）"}
      </ReactMarkdown>
    </div>
  );
}

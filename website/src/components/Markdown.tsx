import ReactMarkdown from "react-markdown";
import { Link } from "@tanstack/react-router";
import { common } from "lowlight";
import rehypeHighlight from "rehype-highlight";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import remarkSmartypants from "remark-smartypants";

const remarkPlugins = [remarkSmartypants, remarkGfm, remarkMath];
const rehypePlugins = [rehypeKatex, [rehypeHighlight, { languages: common }] as const];

export function Markdown({ content }: { content: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={remarkPlugins}
      rehypePlugins={rehypePlugins as any}
      components={{
        a: ({ children, href }) => {
          if (href?.startsWith("/")) {
            return <Link to={href}>{children}</Link>;
          }
          return (
            <a href={href} target="_blank" rel="noopener noreferrer">
              {children}
            </a>
          );
        },
      }}
    >
      {content}
    </ReactMarkdown>
  );
}

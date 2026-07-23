import type { ReactNode } from "react";

interface MarkdownPreviewProps {
  content: string;
}

function inlineMarkup(value: string): ReactNode[] {
  const pattern = /(\[\[[^\]]+\]\]|`[^`]+`|\*\*[^*]+\*\*)/g;
  return value
    .split(pattern)
    .filter(Boolean)
    .map((part, index) => {
      if (part.startsWith("[[")) {
        return (
          <span className="wiki-link" key={`${part}-${index}`}>
            {part.slice(2, -2)}
          </span>
        );
      }
      if (part.startsWith("`")) {
        return <code key={`${part}-${index}`}>{part.slice(1, -1)}</code>;
      }
      if (part.startsWith("**")) {
        return <strong key={`${part}-${index}`}>{part.slice(2, -2)}</strong>;
      }
      return part;
    });
}

export function MarkdownPreview({ content }: MarkdownPreviewProps) {
  const lines = content.split("\n");
  const nodes: ReactNode[] = [];
  let inCode = false;
  let codeLines: string[] = [];

  lines.forEach((line, index) => {
    if (line.startsWith("```")) {
      if (inCode) {
        nodes.push(
          <pre key={`code-${index}`}>
            <code>{codeLines.join("\n")}</code>
          </pre>,
        );
        codeLines = [];
      }
      inCode = !inCode;
      return;
    }
    if (inCode) {
      codeLines.push(line);
      return;
    }
    if (line.startsWith("### ")) {
      nodes.push(<h3 key={index}>{inlineMarkup(line.slice(4))}</h3>);
    } else if (line.startsWith("## ")) {
      nodes.push(<h2 key={index}>{inlineMarkup(line.slice(3))}</h2>);
    } else if (line.startsWith("# ")) {
      nodes.push(<h1 key={index}>{inlineMarkup(line.slice(2))}</h1>);
    } else if (/^- \[[ x]\] /.test(line)) {
      const checked = line.startsWith("- [x]");
      nodes.push(
        <label className="preview-task" key={index}>
          <input type="checkbox" checked={checked} readOnly />
          <span>{inlineMarkup(line.slice(6))}</span>
        </label>,
      );
    } else if (line.startsWith("- ")) {
      nodes.push(
        <div className="preview-bullet" key={index}>
          <span>•</span>
          <p>{inlineMarkup(line.slice(2))}</p>
        </div>,
      );
    } else if (line.startsWith("> ")) {
      nodes.push(
        <blockquote key={index}>{inlineMarkup(line.slice(2))}</blockquote>,
      );
    } else if (line.trim()) {
      nodes.push(<p key={index}>{inlineMarkup(line)}</p>);
    } else {
      nodes.push(<div className="preview-space" key={index} />);
    }
  });

  return (
    <article className="markdown-preview" aria-label="Markdown preview">
      {nodes}
    </article>
  );
}

import type { ToolMeta } from "../lib/registry";
import { Icon } from "./Icon";

interface ToolCardProps {
  tool: ToolMeta;
  onSelect: (tool: ToolMeta) => void;
  featured?: boolean;
}

export function ToolCard({ tool, onSelect, featured = false }: ToolCardProps) {
  return (
    <button
      className={`tool-card ${featured ? "tool-card-featured" : ""}`}
      type="button"
      onClick={() => onSelect(tool)}
    >
      <span className={`tool-icon category-${tool.category}`}>
        <Icon name={tool.icon} size={featured ? 20 : 17} />
      </span>
      <span className="tool-card-copy">
        <strong>{tool.name}</strong>
        <span>{tool.summary}</span>
      </span>
      <Icon className="card-arrow" name="chevron-right" size={14} />
    </button>
  );
}

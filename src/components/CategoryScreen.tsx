import type { Category } from "../lib/registry";
import { useAppStore } from "../stores/app";
import { ToolCard } from "./ToolCard";

interface CategoryScreenProps {
  category: Category;
}

const categoryCopy: Partial<
  Record<Category, { title: string; description: string }>
> = {
  notes: {
    title: "Notes",
    description:
      "Capture context, connect it, and turn it into something useful.",
  },
  time: {
    title: "Time",
    description: "Track the work without letting tracking become the work.",
  },
  pdf: {
    title: "PDF",
    description: "Private document jobs, processed entirely on this Mac.",
  },
  images: {
    title: "Images",
    description: "Small, focused image tasks with safe defaults.",
  },
  workflow: {
    title: "Workflows",
    description:
      "Named local pipelines that connect notes, time, capture, media, and documents.",
  },
};

export function CategoryScreen({ category }: CategoryScreenProps) {
  const catalog = useAppStore((state) => state.catalog);
  const selectTool = useAppStore((state) => state.selectTool);
  const tools = catalog.filter((tool) => tool.category === category);
  const copy = categoryCopy[category] ?? {
    title: category[0].toUpperCase() + category.slice(1),
    description: "Focused local tools, one action at a time.",
  };

  return (
    <main className="workspace category-screen">
      <header className="category-header">
        <p className="eyebrow">Tool library</p>
        <h1>{copy.title}</h1>
        <p>{copy.description}</p>
      </header>
      <div className="category-grid">
        {tools.map((tool) => (
          <ToolCard key={tool.id} tool={tool} onSelect={selectTool} />
        ))}
      </div>
    </main>
  );
}

import type { Category } from "../lib/registry";
import { orderedCategories } from "../lib/registry";
import { useAppStore } from "../stores/app";
import { BrandMark } from "./BrandMark";
import { Icon } from "./Icon";
import { Shortcut } from "./Shortcut";

const categoryPresentation: Partial<
  Record<Category, { label: string; icon: string }>
> = {
  notes: { label: "Notes", icon: "square-pen" },
  time: { label: "Time", icon: "clock" },
  pdf: { label: "PDF", icon: "files" },
  capture: { label: "Capture", icon: "camera" },
  images: { label: "Images", icon: "image" },
  media: { label: "Media", icon: "play" },
  files: { label: "Files", icon: "folder" },
  dev: { label: "Developer", icon: "code" },
  create: { label: "Create", icon: "file-output" },
  workflow: { label: "Workflows", icon: "workflow" },
};

export function Rail() {
  const catalog = useAppStore((state) => state.catalog);
  const category = useAppStore((state) => state.category);
  const collapsed = useAppStore((state) => state.railCollapsed);
  const setCategory = useAppStore((state) => state.setCategory);
  const toggleRail = useAppStore((state) => state.toggleRail);
  const persona = useAppStore((state) => state.persona);
  const categories = orderedCategories(catalog, persona);
  const pinnedCount = Math.min(categories.length, persona === "writer" ? 3 : 4);

  return (
    <aside className={`rail ${collapsed ? "rail-collapsed" : ""}`}>
      <div className="rail-brand">
        <BrandMark compact={collapsed} />
      </div>
      <nav aria-label="Tool categories" className="rail-nav">
        <button
          className={`rail-item ${category === "home" ? "active" : ""}`}
          type="button"
          onClick={() => setCategory("home")}
          aria-current={category === "home" ? "page" : undefined}
          title={collapsed ? "Home" : undefined}
        >
          <Icon name="home" size={16} />
          {!collapsed && <span>Home</span>}
        </button>
        <p className="rail-label" aria-hidden={collapsed}>
          {!collapsed && "Library"}
        </p>
        {categories.map((item, index) => {
          const presentation = categoryPresentation[item] ?? {
            label: item,
            icon: "sparkles",
          };
          return (
            <div className="rail-category-slot" key={item}>
              {index === pinnedCount && !collapsed && (
                <p className="rail-label rail-more-label">More</p>
              )}
              <button
                className={`rail-item ${category === item ? "active" : ""}`}
                type="button"
                onClick={() => setCategory(item)}
                aria-current={category === item ? "page" : undefined}
                title={collapsed ? presentation.label : undefined}
              >
                <Icon name={presentation.icon} size={16} />
                {!collapsed && <span>{presentation.label}</span>}
                {!collapsed && index < 9 && (
                  <span className="rail-shortcut">⌘{index + 1}</span>
                )}
              </button>
            </div>
          );
        })}
      </nav>
      <div className="rail-footer">
        {!collapsed && (
          <div className="local-badge">
            <Icon name="shield" size={13} />
            <span>Local only</span>
          </div>
        )}
        <button
          className="rail-collapse"
          type="button"
          onClick={toggleRail}
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
        >
          <Icon name={collapsed ? "panel-open" : "panel-close"} size={15} />
          {!collapsed && <Shortcut>⌘\</Shortcut>}
        </button>
      </div>
    </aside>
  );
}

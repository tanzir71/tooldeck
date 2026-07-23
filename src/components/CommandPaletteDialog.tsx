import { useEffect, useMemo, useRef, useState } from "react";

import { searchCatalog } from "../lib/registry";
import { useAppStore } from "../stores/app";
import { Icon } from "./Icon";
import { Shortcut } from "./Shortcut";

export function CommandPaletteDialog() {
  const catalog = useAppStore((state) => state.catalog);
  const setOpen = useAppStore((state) => state.setPaletteOpen);
  const selectTool = useAppStore((state) => state.selectTool);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const results = useMemo(
    () => searchCatalog(catalog, query),
    [catalog, query],
  );
  const safeActiveIndex = Math.min(
    activeIndex,
    Math.max(results.length - 1, 0),
  );

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => inputRef.current?.focus());
    return () => window.cancelAnimationFrame(frame);
  }, []);

  return (
    <div
      className="palette-scrim"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) setOpen(false);
      }}
    >
      <section
        className="palette"
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
      >
        <div className="palette-search">
          <Icon name="search" size={18} />
          <input
            ref={inputRef}
            role="combobox"
            aria-autocomplete="list"
            aria-expanded="true"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setActiveIndex(0);
            }}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setActiveIndex((index) =>
                  Math.min(index + 1, results.length - 1),
                );
              } else if (event.key === "ArrowUp") {
                event.preventDefault();
                setActiveIndex((index) => Math.max(index - 1, 0));
              } else if (event.key === "Enter") {
                event.preventDefault();
                const tool = results[safeActiveIndex];
                if (tool) selectTool(tool);
              } else if (event.key === "Escape") {
                event.preventDefault();
                setOpen(false);
              }
            }}
            placeholder="What do you need?"
            aria-controls="palette-results"
            aria-activedescendant={results[safeActiveIndex]?.id}
          />
          <Shortcut>ESC</Shortcut>
        </div>

        <div className="palette-context">
          <span>
            <Icon name="sparkles" size={12} />
            Tools
          </span>
          <span>{results.length} results</span>
        </div>

        <div className="palette-results" id="palette-results" role="listbox">
          {results.length === 0 ? (
            <div className="palette-empty">
              <span className="empty-icon">
                <Icon name="search" size={17} />
              </span>
              <strong>No local tool matches “{query}”</strong>
              <span>Try a task like “resize image” or “track time”.</span>
            </div>
          ) : (
            results.map((tool, index) => (
              <button
                id={tool.id}
                type="button"
                role="option"
                aria-selected={index === safeActiveIndex}
                className={`palette-result ${
                  index === safeActiveIndex ? "active" : ""
                }`}
                key={tool.id}
                onMouseEnter={() => setActiveIndex(index)}
                onClick={() => selectTool(tool)}
              >
                <span className={`palette-icon category-${tool.category}`}>
                  <Icon name={tool.icon} size={17} />
                </span>
                <span className="palette-result-copy">
                  <strong>{tool.name}</strong>
                  <small>
                    {tool.category} · {tool.summary}
                  </small>
                </span>
                {index === safeActiveIndex && <Shortcut>↵</Shortcut>}
              </button>
            ))
          )}
        </div>

        <footer className="palette-footer">
          <span>
            <b>↑↓</b> Navigate
          </span>
          <span>
            <b>↵</b> Open
          </span>
          <span className="palette-private">
            <Icon name="shield" size={11} />
            Search stays local
          </span>
        </footer>
      </section>
    </div>
  );
}

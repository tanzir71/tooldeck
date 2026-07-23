import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { CategoryScreen } from "./components/CategoryScreen";
import { CommandPalette } from "./components/CommandPalette";
import { HomeScreen } from "./components/HomeScreen";
import { Rail } from "./components/Rail";
import { ToolWorkspace } from "./components/ToolWorkspace";
import { TopBar } from "./components/TopBar";
import { fetchCatalog } from "./lib/registry";
import { useAppStore } from "./stores/app";

export function App() {
  const catalog = useAppStore((state) => state.catalog);
  const category = useAppStore((state) => state.category);
  const selectedToolId = useAppStore((state) => state.selectedToolId);
  const setCatalog = useAppStore((state) => state.setCatalog);
  const setCatalogError = useAppStore((state) => state.setCatalogError);
  const setPaletteOpen = useAppStore((state) => state.setPaletteOpen);
  const toggleRail = useAppStore((state) => state.toggleRail);
  const setCategory = useAppStore((state) => state.setCategory);
  const theme = useAppStore((state) => state.theme);
  const cycleTheme = useAppStore((state) => state.cycleTheme);
  const selectedTool = catalog.find((tool) => tool.id === selectedToolId);

  useEffect(() => {
    let cancelled = false;
    void fetchCatalog()
      .then((result) => {
        if (!cancelled) setCatalog(result);
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setCatalogError(
            error instanceof Error
              ? error.message
              : "The local tool catalog could not be loaded.",
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, [setCatalog, setCatalogError]);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (
        (event.metaKey && event.key.toLowerCase() === "k") ||
        (event.altKey && event.code === "Space")
      ) {
        event.preventDefault();
        setPaletteOpen(true);
      } else if (event.metaKey && event.key === "\\") {
        event.preventDefault();
        toggleRail();
      } else if (event.metaKey && event.key === "Enter") {
        const runButton =
          document.querySelector<HTMLButtonElement>(".run-button");
        if (runButton) {
          event.preventDefault();
          runButton.click();
        }
      } else if (event.metaKey && /^[1-9]$/.test(event.key)) {
        const categories = Array.from(
          new Set(catalog.map((tool) => tool.category)),
        );
        const nextCategory = categories[Number(event.key) - 1];
        if (nextCategory) {
          event.preventDefault();
          setCategory(nextCategory);
        }
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [catalog, setCategory, setPaletteOpen, toggleRail]);

  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | undefined;
    void listen("tooldeck://open-palette", () => setPaletteOpen(true)).then(
      (dispose) => {
        unlisten = dispose;
      },
    );
    return () => unlisten?.();
  }, [setPaletteOpen]);

  return (
    <div className="app-shell">
      <Rail />
      <div className="app-main">
        <TopBar
          theme={theme}
          onOpenPalette={() => setPaletteOpen(true)}
          onCycleTheme={cycleTheme}
        />
        {selectedTool ? (
          <ToolWorkspace key={selectedTool.id} tool={selectedTool} />
        ) : category === "home" ? (
          <HomeScreen />
        ) : (
          <CategoryScreen category={category} />
        )}
      </div>
      <CommandPalette />
    </div>
  );
}

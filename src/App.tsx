import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { CategoryScreen } from "./components/CategoryScreen";
import { CommandPalette } from "./components/CommandPalette";
import { HomeScreen } from "./components/HomeScreen";
import { NotesWorkspace } from "./components/NotesWorkspace";
import { Onboarding } from "./components/Onboarding";
import { OutputCenter } from "./components/OutputCenter";
import { Rail } from "./components/Rail";
import { TimeWorkspace } from "./components/TimeWorkspace";
import { ToolWorkspace } from "./components/ToolWorkspace";
import { TopBar } from "./components/TopBar";
import { WorkflowWorkspace } from "./components/WorkflowWorkspace";
import { SettingsWorkspace } from "./components/SettingsWorkspace";
import { fetchCatalog, orderedCategories } from "./lib/registry";
import { sampleActivity } from "./lib/workflows";
import { useAppStore } from "./stores/app";

export function App() {
  const catalog = useAppStore((state) => state.catalog);
  const category = useAppStore((state) => state.category);
  const selectedToolId = useAppStore((state) => state.selectedToolId);
  const toolPrefill = useAppStore((state) => state.toolPrefill);
  const settingsOpen = useAppStore((state) => state.settingsOpen);
  const onboardingOpen = useAppStore((state) => state.onboardingOpen);
  const persona = useAppStore((state) => state.persona);
  const setCatalog = useAppStore((state) => state.setCatalog);
  const setCatalogError = useAppStore((state) => state.setCatalogError);
  const setPaletteOpen = useAppStore((state) => state.setPaletteOpen);
  const setOutputCenterOpen = useAppStore((state) => state.setOutputCenterOpen);
  const setSettingsOpen = useAppStore((state) => state.setSettingsOpen);
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
        const categories = orderedCategories(catalog, persona);
        const nextCategory = categories[Number(event.key) - 1];
        if (nextCategory) {
          event.preventDefault();
          setCategory(nextCategory);
        }
      } else if (event.key === "Escape") {
        if (settingsOpen) setSettingsOpen(false);
        else if (selectedToolId) setCategory(category);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    catalog,
    category,
    persona,
    selectedToolId,
    setCategory,
    setPaletteOpen,
    setSettingsOpen,
    settingsOpen,
    toggleRail,
  ]);

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

  useEffect(() => {
    if (!isTauri()) return;
    const sample = () => void sampleActivity().catch(() => undefined);
    const timer = window.setInterval(sample, 60_000);
    sample();
    return () => window.clearInterval(timer);
  }, []);

  return (
    <div className="app-shell">
      <Rail />
      <div className="app-main">
        <TopBar
          theme={theme}
          onOpenPalette={() => setPaletteOpen(true)}
          onOpenOutputCenter={() => setOutputCenterOpen(true)}
          onOpenSettings={() => setSettingsOpen(true)}
          onOpenTimer={() => {
            const timerTool = catalog.find((tool) => tool.id === "timer-start");
            if (timerTool) useAppStore.getState().selectTool(timerTool);
          }}
          onCycleTheme={cycleTheme}
        />
        {settingsOpen ? (
          <SettingsWorkspace />
        ) : selectedTool ? (
          selectedTool.id === "note-new" ? (
            <NotesWorkspace />
          ) : selectedTool.id === "timer-start" ? (
            <TimeWorkspace />
          ) : selectedTool.category === "workflow" ? (
            <WorkflowWorkspace key={selectedTool.id} tool={selectedTool} />
          ) : (
            <ToolWorkspace
              key={selectedTool.id}
              tool={selectedTool}
              prefill={toolPrefill}
            />
          )
        ) : category === "home" ? (
          <HomeScreen />
        ) : (
          <CategoryScreen category={category} />
        )}
      </div>
      <CommandPalette />
      <OutputCenter />
      {onboardingOpen && <Onboarding />}
    </div>
  );
}

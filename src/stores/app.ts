import { create } from "zustand";

import type { Category, ToolInput, ToolMeta } from "../lib/registry";

export type ThemePreference = "system" | "light" | "dark";

interface AppState {
  catalog: ToolMeta[];
  catalogLoading: boolean;
  catalogError: string | null;
  selectedToolId: string | null;
  toolPrefill: ToolInput | null;
  category: Category | "home";
  paletteOpen: boolean;
  outputCenterOpen: boolean;
  railCollapsed: boolean;
  theme: ThemePreference;
  recentToolIds: string[];
  setCatalog: (catalog: ToolMeta[]) => void;
  setCatalogError: (message: string) => void;
  selectTool: (tool: ToolMeta, prefill?: ToolInput) => void;
  setCategory: (category: Category | "home") => void;
  setPaletteOpen: (open: boolean) => void;
  setOutputCenterOpen: (open: boolean) => void;
  toggleRail: () => void;
  cycleTheme: () => void;
}

function savedTheme(): ThemePreference {
  let value: string | null = null;
  try {
    value = window.localStorage?.getItem("tooldeck-theme-preference") ?? null;
  } catch {
    value = null;
  }
  return value === "light" || value === "dark" || value === "system"
    ? value
    : "system";
}

function applyTheme(theme: ThemePreference) {
  const dark =
    theme === "dark" ||
    (theme === "system" &&
      typeof window.matchMedia === "function" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  try {
    window.localStorage?.setItem("tooldeck-theme-preference", theme);
  } catch {
    // Storage can be disabled; the selected theme still applies for this run.
  }
}

export const useAppStore = create<AppState>((set, get) => ({
  catalog: [],
  catalogLoading: true,
  catalogError: null,
  selectedToolId: null,
  toolPrefill: null,
  category: "home",
  paletteOpen: false,
  outputCenterOpen: false,
  railCollapsed: false,
  theme: savedTheme(),
  recentToolIds: [],
  setCatalog: (catalog) =>
    set({ catalog, catalogLoading: false, catalogError: null }),
  setCatalogError: (message) =>
    set({ catalogError: message, catalogLoading: false }),
  selectTool: (tool, prefill) =>
    set((state) => ({
      selectedToolId: tool.id,
      toolPrefill: prefill ?? null,
      category: tool.category,
      paletteOpen: false,
      recentToolIds: [
        tool.id,
        ...state.recentToolIds.filter((id) => id !== tool.id),
      ].slice(0, 6),
    })),
  setCategory: (category) =>
    set({
      category,
      selectedToolId: null,
      toolPrefill: null,
      paletteOpen: false,
    }),
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  setOutputCenterOpen: (outputCenterOpen) => set({ outputCenterOpen }),
  toggleRail: () => set((state) => ({ railCollapsed: !state.railCollapsed })),
  cycleTheme: () => {
    const order: ThemePreference[] = ["system", "light", "dark"];
    const theme = order[(order.indexOf(get().theme) + 1) % order.length];
    applyTheme(theme);
    set({ theme });
  },
}));

applyTheme(useAppStore.getState().theme);

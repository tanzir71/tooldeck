import { create } from "zustand";

import {
  onboardingComplete,
  saveDensity,
  saveOnboardingComplete,
  savePersona,
  savedDensity,
  savedPersona,
  type Density,
  type Persona,
} from "../lib/preferences";
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
  settingsOpen: boolean;
  onboardingOpen: boolean;
  railCollapsed: boolean;
  theme: ThemePreference;
  persona: Persona;
  density: Density;
  recentToolIds: string[];
  setCatalog: (catalog: ToolMeta[]) => void;
  setCatalogError: (message: string) => void;
  selectTool: (tool: ToolMeta, prefill?: ToolInput) => void;
  setCategory: (category: Category | "home") => void;
  setPaletteOpen: (open: boolean) => void;
  setOutputCenterOpen: (open: boolean) => void;
  setSettingsOpen: (open: boolean) => void;
  setOnboardingOpen: (open: boolean) => void;
  finishOnboarding: (persona: Persona) => void;
  setPersona: (persona: Persona) => void;
  setDensity: (density: Density) => void;
  toggleRail: () => void;
  cycleTheme: () => void;
  setTheme: (theme: ThemePreference) => void;
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

function applyDensity(density: Density) {
  document.documentElement.dataset.density = density;
  saveDensity(density);
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
  settingsOpen: false,
  onboardingOpen: !onboardingComplete(),
  railCollapsed: false,
  theme: savedTheme(),
  persona: savedPersona(),
  density: savedDensity(),
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
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
  setOnboardingOpen: (onboardingOpen) => set({ onboardingOpen }),
  finishOnboarding: (persona) => {
    savePersona(persona);
    saveOnboardingComplete(true);
    set({ persona, onboardingOpen: false });
  },
  setPersona: (persona) => {
    savePersona(persona);
    set({ persona });
  },
  setDensity: (density) => {
    applyDensity(density);
    set({ density });
  },
  toggleRail: () => set((state) => ({ railCollapsed: !state.railCollapsed })),
  cycleTheme: () => {
    const order: ThemePreference[] = ["system", "light", "dark"];
    const theme = order[(order.indexOf(get().theme) + 1) % order.length];
    applyTheme(theme);
    set({ theme });
  },
  setTheme: (theme) => {
    applyTheme(theme);
    set({ theme });
  },
}));

applyTheme(useAppStore.getState().theme);
applyDensity(useAppStore.getState().density);

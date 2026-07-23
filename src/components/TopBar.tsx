import type { ThemePreference } from "../stores/app";
import { Icon } from "./Icon";
import { Shortcut } from "./Shortcut";

interface TopBarProps {
  theme: ThemePreference;
  onOpenPalette: () => void;
  onCycleTheme: () => void;
}

const themeIcon: Record<ThemePreference, string> = {
  system: "sun",
  light: "sun",
  dark: "moon",
};

export function TopBar({ theme, onOpenPalette, onCycleTheme }: TopBarProps) {
  return (
    <header className="topbar" data-tauri-drag-region>
      <div className="traffic-space" data-tauri-drag-region />
      <button
        className="search-trigger"
        type="button"
        onClick={onOpenPalette}
        aria-label="Open command palette"
      >
        <Icon name="search" size={14} />
        <span>Search tools, notes, and actions</span>
        <Shortcut>⌘K</Shortcut>
      </button>
      <div className="topbar-actions">
        <button
          className="timer-pill"
          type="button"
          aria-label="No timer running. Start a timer."
        >
          <span className="timer-status" />
          <span>No timer</span>
          <Icon name="play" size={11} fill="currentColor" />
        </button>
        <button
          className="icon-button"
          type="button"
          onClick={onCycleTheme}
          aria-label={`Theme: ${theme}. Change theme.`}
          title={`Theme: ${theme}`}
        >
          <Icon name={themeIcon[theme]} size={15} />
        </button>
        <button
          className="icon-button"
          type="button"
          aria-label="Open settings"
          title="Settings"
        >
          <Icon name="settings" size={15} />
        </button>
      </div>
    </header>
  );
}

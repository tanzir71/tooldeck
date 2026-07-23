import { useEffect, useState } from "react";

import { formatDuration, getTimerStatus } from "../lib/time";
import type { TimeEntry } from "../lib/time";
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
  const [running, setRunning] = useState<TimeEntry | null>(null);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));

  useEffect(() => {
    let cancelled = false;
    const refresh = () => {
      void getTimerStatus().then((entry) => {
        if (!cancelled) setRunning(entry);
      });
    };
    refresh();
    const interval = window.setInterval(() => {
      setNow(Math.floor(Date.now() / 1000));
      refresh();
    }, 2_000);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, []);

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
          className={`timer-pill ${running ? "running" : ""}`}
          type="button"
          aria-label={
            running
              ? `${running.description}, ${formatDuration(
                  now - running.startedAt,
                )}`
              : "No timer running. Start a timer."
          }
        >
          <span className="timer-status" />
          <span>
            {running
              ? `${running.description} · ${formatDuration(
                  now - running.startedAt,
                )}`
              : "No timer"}
          </span>
          <Icon
            name={running ? "pause" : "play"}
            size={11}
            fill="currentColor"
          />
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

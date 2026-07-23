import { invoke, isTauri } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import {
  getAllWebviewWindows,
  getCurrentWebviewWindow,
} from "@tauri-apps/api/webviewWindow";
import { useCallback, useEffect, useState } from "react";

import {
  formatDuration,
  getTimerReport,
  getTimerStatus,
  startTimer,
  stopTimer,
} from "../lib/time";
import type { TimeEntry } from "../lib/time";
import { BrandMark } from "./BrandMark";
import { Icon } from "./Icon";

async function openMainWindow(openPalette = false) {
  if (!isTauri()) return;
  const windows = await getAllWebviewWindows();
  const main = windows.find((window) => window.label === "main");
  await main?.show();
  await main?.setFocus();
  if (openPalette) await emit("tooldeck://open-palette");
  await getCurrentWebviewWindow().hide();
}

export function MenuBarPanel() {
  const [running, setRunning] = useState<TimeEntry | null>(null);
  const [todaySeconds, setTodaySeconds] = useState(0);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));

  const refresh = useCallback(async () => {
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    const end = new Date(start);
    end.setDate(end.getDate() + 1);
    const [status, report] = await Promise.all([
      getTimerStatus(),
      getTimerReport(
        Math.floor(start.getTime() / 1000),
        Math.floor(end.getTime() / 1000),
      ),
    ]);
    setRunning(status);
    setTodaySeconds(report.totalSeconds);
  }, []);

  useEffect(() => {
    const initial = window.setTimeout(() => void refresh(), 0);
    const interval = window.setInterval(
      () => setNow(Math.floor(Date.now() / 1000)),
      1_000,
    );
    return () => {
      window.clearTimeout(initial);
      window.clearInterval(interval);
    };
  }, [refresh]);

  async function toggleTimer() {
    if (running) {
      await stopTimer();
    } else {
      await startTimer({
        projectId: 1,
        notePath: null,
        description: "Focus block",
        billable: false,
      });
    }
    await refresh();
  }

  async function showQuickNote() {
    if (!isTauri()) return;
    await invoke("show_floating_note");
    await getCurrentWebviewWindow().hide();
  }

  const liveSeconds = running ? now - running.startedAt : 0;

  return (
    <main className="menubar-panel">
      <header className="menubar-header">
        <BrandMark />
        <span className="local-dot">Local</span>
      </header>
      <section className="menubar-timer">
        <div className="menubar-timer-copy">
          <span className="large-timer">
            {formatDuration(liveSeconds).padStart(8, "0")}
          </span>
          <span>{running?.description ?? "Ready when you are"}</span>
        </div>
        <button
          className={`round-play ${running ? "is-running" : ""}`}
          type="button"
          aria-label={running ? "Stop timer" : "Start timer"}
          onClick={() => void toggleTimer()}
        >
          <Icon
            name={running ? "pause" : "play"}
            size={17}
            fill="currentColor"
          />
        </button>
      </section>
      <div className="today-row">
        <span>Today</span>
        <strong>{formatDuration(todaySeconds)}</strong>
      </div>
      <section className="quick-actions" aria-label="Quick actions">
        <button type="button" onClick={() => void showQuickNote()}>
          <Icon name="square-pen" size={15} />
          New note
          <span>⌥N</span>
        </button>
        <button type="button" onClick={() => void openMainWindow(true)}>
          <Icon name="sparkles" size={15} />
          Find a tool
          <span>⌥Space</span>
        </button>
        <button type="button">
          <Icon name="file-output" size={15} />
          Output Center
          <span>0</span>
        </button>
      </section>
      <footer className="menubar-footer">
        <button type="button" onClick={() => void openMainWindow()}>
          Open Tooldeck
        </button>
        <button type="button" aria-label="Open settings">
          <Icon name="settings" size={14} />
        </button>
      </footer>
    </main>
  );
}

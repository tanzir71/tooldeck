import {
  getAllWebviewWindows,
  getCurrentWebviewWindow,
} from "@tauri-apps/api/webviewWindow";
import { isTauri } from "@tauri-apps/api/core";

import { BrandMark } from "./BrandMark";
import { Icon } from "./Icon";

async function openMainWindow() {
  if (!isTauri()) return;
  const windows = await getAllWebviewWindows();
  const main = windows.find((window) => window.label === "main");
  await main?.show();
  await main?.setFocus();
  await getCurrentWebviewWindow().hide();
}

export function MenuBarPanel() {
  return (
    <main className="menubar-panel">
      <header className="menubar-header">
        <BrandMark />
        <span className="local-dot">Local</span>
      </header>
      <section className="menubar-timer">
        <div className="menubar-timer-copy">
          <span className="large-timer">00:00:00</span>
          <span>Ready when you are</span>
        </div>
        <button className="round-play" type="button" aria-label="Start timer">
          <Icon name="play" size={17} fill="currentColor" />
        </button>
      </section>
      <div className="today-row">
        <span>Today</span>
        <strong>0h 00m</strong>
      </div>
      <section className="quick-actions" aria-label="Quick actions">
        <button type="button">
          <Icon name="square-pen" size={15} />
          New note
          <span>⌥N</span>
        </button>
        <button type="button">
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

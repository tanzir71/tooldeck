import { useMemo } from "react";

import { useAppStore } from "../stores/app";
import { Icon } from "./Icon";
import { ToolCard } from "./ToolCard";

export function HomeScreen() {
  const catalog = useAppStore((state) => state.catalog);
  const loading = useAppStore((state) => state.catalogLoading);
  const error = useAppStore((state) => state.catalogError);
  const selectTool = useAppStore((state) => state.selectTool);
  const setPaletteOpen = useAppStore((state) => state.setPaletteOpen);
  const recentIds = useAppStore((state) => state.recentToolIds);

  const recents = useMemo(
    () =>
      recentIds
        .map((id) => catalog.find((tool) => tool.id === id))
        .filter((tool) => tool !== undefined),
    [catalog, recentIds],
  );
  const favorites = ["note-new", "timer-start", "pdf-merge", "image-resize"]
    .map((id) => catalog.find((tool) => tool.id === id))
    .filter((tool) => tool !== undefined);

  return (
    <main className="workspace home-screen">
      <section className="home-hero">
        <div>
          <p className="eyebrow">Thursday, July 23</p>
          <h1>Good morning.</h1>
          <p className="hero-copy">
            Your notes, time, and file tools are ready — and stay on this Mac.
          </p>
        </div>
        <button
          className="primary-button"
          type="button"
          onClick={() => setPaletteOpen(true)}
        >
          <Icon name="sparkles" size={15} />
          Find a tool
          <span className="button-shortcut">⌘K</span>
        </button>
      </section>

      <section className="continuity-card">
        <div className="continuity-top">
          <div>
            <p className="section-kicker">Jump back in</p>
            <h2>No timer running</h2>
            <p>Start a focus block and keep today’s work accounted for.</p>
          </div>
          <span className="time-total">
            0h 00m <small>today</small>
          </span>
        </div>
        <div className="continuity-actions">
          <button
            type="button"
            className="quiet-button"
            onClick={() => {
              const timer = catalog.find((tool) => tool.id === "timer-start");
              if (timer) selectTool(timer);
            }}
          >
            <Icon name="play" size={13} fill="currentColor" />
            Start timer
          </button>
          <span className="privacy-line">
            <Icon name="shield" size={12} />
            Activity never leaves your Mac
          </span>
        </div>
      </section>

      <section className="home-section">
        <div className="section-heading">
          <div>
            <p className="section-kicker">Pinned for you</p>
            <h2>Everyday tools</h2>
          </div>
          <button
            className="text-button"
            type="button"
            onClick={() => setPaletteOpen(true)}
          >
            View all
            <Icon name="chevron-right" size={13} />
          </button>
        </div>

        {loading && (
          <div className="state-panel" aria-live="polite">
            <Icon className="spin" name="loader" size={18} />
            <span>Loading your local tool catalog…</span>
          </div>
        )}
        {error && (
          <div className="state-panel state-error" role="alert">
            <Icon name="circle-alert" size={18} />
            <span>{error}</span>
          </div>
        )}
        {!loading && !error && (
          <div className="tool-grid">
            {favorites.map((tool, index) => (
              <ToolCard
                key={tool.id}
                tool={tool}
                onSelect={selectTool}
                featured={index < 2}
              />
            ))}
          </div>
        )}
      </section>

      <section className="home-section recents-section">
        <div className="section-heading compact-heading">
          <div>
            <p className="section-kicker">Recent</p>
            <h2>Your trail</h2>
          </div>
        </div>
        {recents.length === 0 ? (
          <div className="empty-row">
            <span className="empty-icon">
              <Icon name="clock" size={16} />
            </span>
            <p>
              Tools you open will appear here.
              <span>Nothing to clean up yet.</span>
            </p>
          </div>
        ) : (
          <div className="recent-list">
            {recents.map((tool) => (
              <ToolCard key={tool.id} tool={tool} onSelect={selectTool} />
            ))}
          </div>
        )}
      </section>
    </main>
  );
}

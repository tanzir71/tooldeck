import { useCallback, useEffect, useMemo, useState } from "react";

import {
  addManualEntry,
  formatDuration,
  getIdleSeconds,
  getTimerReport,
  getTimerStatus,
  listProjects,
  reconcileIdle,
  startTimer,
  stopTimer,
} from "../lib/time";
import type { Project, TimeEntry, TimerReport } from "../lib/time";
import { Icon } from "./Icon";

function todayBounds(): [number, number] {
  const start = new Date();
  start.setHours(0, 0, 0, 0);
  const end = new Date(start);
  end.setDate(end.getDate() + 1);
  return [Math.floor(start.getTime() / 1000), Math.floor(end.getTime() / 1000)];
}

export function TimeWorkspace() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [running, setRunning] = useState<TimeEntry | null>(null);
  const [report, setReport] = useState<TimerReport | null>(null);
  const [description, setDescription] = useState("");
  const [projectId, setProjectId] = useState<number | null>(1);
  const [billable, setBillable] = useState(false);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [idlePrompt, setIdlePrompt] = useState<number | null>(null);
  const [manualOpen, setManualOpen] = useState(false);
  const [manualDescription, setManualDescription] = useState("");
  const [manualStart, setManualStart] = useState("");
  const [manualEnd, setManualEnd] = useState("");

  const refresh = useCallback(async () => {
    const [dayStart, dayEnd] = todayBounds();
    const [projectItems, status, today] = await Promise.all([
      listProjects(),
      getTimerStatus(),
      getTimerReport(dayStart, dayEnd),
    ]);
    setProjects(projectItems);
    setRunning(status);
    setReport(today);
    if (status) {
      setDescription(status.description);
      setProjectId(status.projectId);
      setBillable(status.billable);
    } else if (projectItems[0]) {
      setProjectId((current) => current ?? projectItems[0].id);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    void refresh().catch((reason: unknown) => {
      if (!cancelled) {
        setError(reason instanceof Error ? reason.message : String(reason));
      }
    });
    return () => {
      cancelled = true;
    };
  }, [refresh]);

  useEffect(() => {
    if (!running) return;
    const interval = window.setInterval(
      () => setNow(Math.floor(Date.now() / 1000)),
      1_000,
    );
    return () => window.clearInterval(interval);
  }, [running]);

  useEffect(() => {
    if (!running) return;
    const checkIdle = () => {
      void getIdleSeconds().then((seconds) => {
        if (seconds >= 300) setIdlePrompt(Math.floor(seconds));
      });
    };
    const interval = window.setInterval(checkIdle, 15_000);
    return () => window.clearInterval(interval);
  }, [running]);

  const liveSeconds = running ? now - running.startedAt : 0;
  const selectedProject = useMemo(
    () => projects.find((project) => project.id === projectId),
    [projectId, projects],
  );

  async function toggleTimer() {
    setBusy(true);
    setError(null);
    try {
      if (running) {
        await stopTimer();
        setRunning(null);
        setDescription("");
      } else {
        const entry = await startTimer({
          projectId,
          notePath: null,
          description,
          billable,
        });
        setRunning(entry);
        setNow(Math.floor(Date.now() / 1000));
      }
      await refresh();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }

  async function resolveIdle(decision: "keep" | "discard" | "reassign") {
    if (!idlePrompt) return;
    try {
      await reconcileIdle(idlePrompt, decision, projectId);
      setIdlePrompt(null);
      await refresh();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  async function submitManual() {
    const startedAt = Math.floor(new Date(manualStart).getTime() / 1000);
    const stoppedAt = Math.floor(new Date(manualEnd).getTime() / 1000);
    try {
      await addManualEntry({
        projectId,
        description: manualDescription,
        startedAt,
        stoppedAt,
        billable,
      });
      setManualOpen(false);
      setManualDescription("");
      setManualStart("");
      setManualEnd("");
      await refresh();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  return (
    <main className="workspace time-workspace">
      <header className="time-header">
        <div>
          <p className="eyebrow">Focus</p>
          <h1>Time tracking</h1>
          <p>One timer, tied to the work that gives it meaning.</p>
        </div>
        <div className="today-total">
          <span>Today</span>
          <strong>{formatDuration(report?.totalSeconds ?? 0)}</strong>
        </div>
      </header>

      <section className={`timer-card ${running ? "running" : ""}`}>
        <div className="timer-card-top">
          <span
            className="timer-project-dot"
            style={{ background: selectedProject?.color }}
          />
          <span>{running ? "Tracking now" : "Ready to focus"}</span>
          {running?.billable && <i>Billable</i>}
        </div>
        <div className="timer-main">
          <div className="timer-clock">
            {formatDuration(running ? liveSeconds : 0)}
          </div>
          <button
            className={`timer-toggle ${running ? "stop" : ""}`}
            type="button"
            onClick={() => void toggleTimer()}
            disabled={busy}
            aria-label={running ? "Stop timer" : "Start timer"}
          >
            <Icon
              className={busy ? "spin" : ""}
              name={busy ? "loader" : running ? "pause" : "play"}
              size={18}
              fill={busy ? "none" : "currentColor"}
            />
          </button>
        </div>
        <div className="timer-inputs">
          <input
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            placeholder="What are you working on?"
            aria-label="Timer description"
            disabled={Boolean(running)}
          />
          <select
            value={projectId ?? ""}
            onChange={(event) => {
              const nextId = event.target.value
                ? Number(event.target.value)
                : null;
              setProjectId(nextId);
              const project = projects.find((item) => item.id === nextId);
              setBillable(project?.billable ?? false);
            }}
            aria-label="Timer project"
            disabled={Boolean(running)}
          >
            <option value="">No project</option>
            {projects.map((project) => (
              <option value={project.id} key={project.id}>
                {project.clientName} · {project.name}
              </option>
            ))}
          </select>
          <button
            type="button"
            className={`billable-toggle ${billable ? "active" : ""}`}
            onClick={() => setBillable((value) => !value)}
            disabled={Boolean(running)}
            aria-pressed={billable}
          >
            $
          </button>
        </div>
      </section>

      <section className="time-metrics">
        <div>
          <span>Total today</span>
          <strong>{formatDuration(report?.totalSeconds ?? 0)}</strong>
        </div>
        <div>
          <span>Billable</span>
          <strong>{formatDuration(report?.billableSeconds ?? 0)}</strong>
        </div>
        <div>
          <span>Amount</span>
          <strong>${(report?.amount ?? 0).toFixed(2)}</strong>
        </div>
      </section>

      <section className="entries-section">
        <div className="section-heading">
          <div>
            <p className="section-kicker">Timeline</p>
            <h2>Today’s entries</h2>
          </div>
          <button
            className="text-button"
            type="button"
            onClick={() => setManualOpen((open) => !open)}
          >
            <Icon name="plus" size={13} />
            Manual entry
          </button>
        </div>
        {manualOpen && (
          <div className="manual-entry-form">
            <input
              value={manualDescription}
              onChange={(event) => setManualDescription(event.target.value)}
              placeholder="What did you work on?"
              aria-label="Manual entry description"
            />
            <label>
              Start
              <input
                type="datetime-local"
                value={manualStart}
                onChange={(event) => setManualStart(event.target.value)}
              />
            </label>
            <label>
              End
              <input
                type="datetime-local"
                value={manualEnd}
                onChange={(event) => setManualEnd(event.target.value)}
              />
            </label>
            <button
              className="primary-button"
              type="button"
              disabled={!manualStart || !manualEnd}
              onClick={() => void submitManual()}
            >
              Add
            </button>
          </div>
        )}
        {!report || report.entries.length === 0 ? (
          <div className="time-empty">
            <span className="empty-icon">
              <Icon name="clock" size={16} />
            </span>
            <div>
              <strong>No time recorded today.</strong>
              <span>Your first focus block will appear here.</span>
            </div>
          </div>
        ) : (
          <div className="entry-list">
            {report.entries.map((entry) => (
              <div className="entry-row" key={entry.id}>
                <span
                  className="entry-color"
                  style={{ background: entry.projectColor ?? "#92928e" }}
                />
                <div>
                  <strong>{entry.description}</strong>
                  <span>{entry.projectName ?? "No project"}</span>
                </div>
                {entry.billable && <i>$</i>}
                <time>{formatDuration(entry.durationSeconds)}</time>
              </div>
            ))}
          </div>
        )}
      </section>

      {error && (
        <div className="notes-error" role="alert">
          <Icon name="circle-alert" size={14} />
          <span>{error}</span>
          <button type="button" onClick={() => setError(null)}>
            Dismiss
          </button>
        </div>
      )}
      {idlePrompt && (
        <div className="idle-prompt" role="alertdialog" aria-modal="true">
          <div>
            <Icon name="clock" size={16} />
            <span>
              You were away for about {Math.round(idlePrompt / 60)} minutes.
            </span>
          </div>
          <button type="button" onClick={() => void resolveIdle("keep")}>
            Keep
          </button>
          <button type="button" onClick={() => void resolveIdle("discard")}>
            Discard
          </button>
          <button
            className="primary-button"
            type="button"
            onClick={() => void resolveIdle("reassign")}
          >
            Reassign
          </button>
        </div>
      )}
    </main>
  );
}

import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

import type { JobRecord, SafetyRecord } from "../lib/jobs";
import {
  cancelJob,
  jobLabel,
  listJobs,
  listSafety,
  undoSafety,
} from "../lib/jobs";
import { useAppStore } from "../stores/app";
import { Icon } from "./Icon";

function outputName(path: string | null): string {
  if (!path) return "Preparing output…";
  return path.split("/").at(-1) ?? path;
}

function JobRow({
  job,
  onCancel,
}: {
  job: JobRecord;
  onCancel: (id: string) => void;
}) {
  const active = job.status === "queued" || job.status === "running";
  return (
    <article className="job-row">
      <div className={`job-glyph job-${job.status}`}>
        <Icon
          name={
            job.status === "completed"
              ? "circle-check"
              : job.status === "failed"
                ? "circle-alert"
                : job.status === "cancelled"
                  ? "x"
                  : "loader"
          }
          className={active ? "spin" : undefined}
          size={16}
        />
      </div>
      <div className="job-copy">
        <div className="job-title-line">
          <strong>{jobLabel(job.kind)}</strong>
          <span className={`job-status job-status-${job.status}`}>
            {job.status}
          </span>
        </div>
        <span className="job-output">{outputName(job.outputPath)}</span>
        {job.error && <p role="alert">{job.error}</p>}
        {active && (
          <div
            className="job-progress"
            role="progressbar"
            aria-label={`${jobLabel(job.kind)} progress`}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(job.progress * 100)}
          >
            <span style={{ width: `${Math.round(job.progress * 100)}%` }} />
          </div>
        )}
      </div>
      {active && (
        <button
          className="job-cancel"
          type="button"
          onClick={() => onCancel(job.id)}
          aria-label={`Cancel ${jobLabel(job.kind)}`}
        >
          Cancel
        </button>
      )}
    </article>
  );
}

export function OutputCenter() {
  const open = useAppStore((state) => state.outputCenterOpen);
  const setOpen = useAppStore((state) => state.setOutputCenterOpen);
  const [jobs, setJobs] = useState<JobRecord[]>([]);
  const [safety, setSafety] = useState<SafetyRecord[]>([]);
  const [tab, setTab] = useState<"outputs" | "safety">("outputs");
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [nextJobs, nextSafety] = await Promise.all([
        listJobs(),
        listSafety(),
      ]);
      setJobs(nextJobs);
      setSafety(nextSafety);
      setError(null);
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "The local job history could not be loaded.",
      );
    }
  }, []);

  useEffect(() => {
    if (!open) return;
    const refreshTimer = window.setTimeout(() => void refresh(), 0);
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.clearTimeout(refreshTimer);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [open, refresh, setOpen]);

  useEffect(() => {
    if (isTauri()) {
      let dispose: (() => void) | undefined;
      void listen("tooldeck://job-updated", () => void refresh()).then(
        (unlisten) => {
          dispose = unlisten;
        },
      );
      return () => dispose?.();
    }
    const update = () => void refresh();
    window.addEventListener("tooldeck:preview-job-updated", update);
    return () =>
      window.removeEventListener("tooldeck:preview-job-updated", update);
  }, [refresh]);

  if (!open) return null;

  async function handleCancel(id: string) {
    await cancelJob(id);
    await refresh();
  }

  async function handleUndo(id: number) {
    await undoSafety(id);
    await refresh();
  }

  return (
    <div className="output-scrim" onMouseDown={() => setOpen(false)}>
      <aside
        className="output-center"
        role="dialog"
        aria-modal="true"
        aria-labelledby="output-center-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="output-header">
          <div>
            <p className="eyebrow">Local queue</p>
            <h2 id="output-center-title">Output Center</h2>
          </div>
          <button
            className="icon-button"
            type="button"
            onClick={() => setOpen(false)}
            aria-label="Close Output Center"
          >
            <Icon name="x" size={16} />
          </button>
        </header>
        <div className="output-tabs" role="tablist" aria-label="Output Center">
          <button
            type="button"
            role="tab"
            aria-selected={tab === "outputs"}
            className={tab === "outputs" ? "active" : ""}
            onClick={() => setTab("outputs")}
          >
            Outputs
            <span>{jobs.length}</span>
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={tab === "safety"}
            className={tab === "safety" ? "active" : ""}
            onClick={() => setTab("safety")}
          >
            Safety
            <span>{safety.filter((record) => !record.undoneAt).length}</span>
          </button>
        </div>
        <div className="output-privacy">
          <Icon name="shield" size={14} />
          Processing stays on this Mac. Originals remain untouched.
        </div>
        <div className="job-list" role="tabpanel">
          {error ? (
            <div className="output-empty output-error" role="alert">
              <Icon name="circle-alert" size={20} />
              <strong>Couldn’t load jobs</strong>
              <span>{error}</span>
              <button type="button" onClick={() => void refresh()}>
                Try again
              </button>
            </div>
          ) : tab === "outputs" && jobs.length === 0 ? (
            <div className="output-empty">
              <Icon name="inbox" size={22} />
              <strong>No outputs yet</strong>
              <span>
                Run a PDF, image, or media tool and it will appear here.
              </span>
            </div>
          ) : tab === "outputs" ? (
            jobs.map((job) => (
              <JobRow key={job.id} job={job} onCancel={handleCancel} />
            ))
          ) : safety.length === 0 ? (
            <div className="output-empty">
              <Icon name="shield" size={22} />
              <strong>No file changes yet</strong>
              <span>Created outputs will be recorded here for safe undo.</span>
            </div>
          ) : (
            safety.map((record) => (
              <article className="safety-row" key={record.id}>
                <div>
                  <strong>{jobLabel(record.opKind)}</strong>
                  <span>{outputName(record.outputPath)}</span>
                </div>
                {record.undoneAt ? (
                  <span className="undone-label">Undone</span>
                ) : (
                  <button
                    type="button"
                    disabled={!record.reversible}
                    onClick={() => void handleUndo(record.id)}
                  >
                    Undo
                  </button>
                )}
              </article>
            ))
          )}
        </div>
      </aside>
    </div>
  );
}

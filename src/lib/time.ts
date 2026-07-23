import { invoke, isTauri } from "@tauri-apps/api/core";

export interface Project {
  id: number;
  clientName: string;
  name: string;
  color: string;
  billable: boolean;
  rate: number;
}

export interface TimeEntry {
  id: number;
  projectId: number | null;
  projectName: string | null;
  projectColor: string | null;
  notePath: string | null;
  description: string;
  startedAt: number;
  stoppedAt: number | null;
  source: "timer" | "manual" | "idle";
  billable: boolean;
  rateSnapshot: number;
  durationSeconds: number;
}

export interface TimerReport {
  totalSeconds: number;
  billableSeconds: number;
  amount: number;
  entries: TimeEntry[];
}

const previewProjects: Project[] = [
  {
    id: 1,
    clientName: "Personal",
    name: "General",
    color: "#4D5FDB",
    billable: false,
    rate: 0,
  },
  {
    id: 2,
    clientName: "Northstar",
    name: "Website refresh",
    color: "#3F8B68",
    billable: true,
    rate: 120,
  },
];

const previewEntries: TimeEntry[] = [];
let previewRunning: TimeEntry | null = null;
let previewId = 1;

export async function listProjects(): Promise<Project[]> {
  if (isTauri()) return invoke<Project[]>("projects_list");
  return structuredClone(previewProjects);
}

export async function createProject(
  name: string,
  color: string,
  billable: boolean,
  rate: number,
): Promise<Project> {
  if (isTauri()) {
    return invoke<Project>("project_create", { name, color, billable, rate });
  }
  const project: Project = {
    id: previewProjects.length + 1,
    clientName: "Personal",
    name,
    color,
    billable,
    rate,
  };
  previewProjects.push(project);
  return structuredClone(project);
}

export async function getTimerStatus(): Promise<TimeEntry | null> {
  if (isTauri()) return invoke<TimeEntry | null>("timer_status");
  return previewRunning ? withLiveDuration(previewRunning) : null;
}

export async function startTimer(input: {
  projectId: number | null;
  notePath: string | null;
  description: string;
  billable: boolean;
}): Promise<TimeEntry> {
  if (isTauri()) return invoke<TimeEntry>("timer_start", input);
  const now = Math.floor(Date.now() / 1000);
  if (previewRunning) {
    previewRunning.stoppedAt = now;
    previewRunning.durationSeconds = now - previewRunning.startedAt;
    previewEntries.unshift(previewRunning);
  }
  const project = previewProjects.find((item) => item.id === input.projectId);
  previewRunning = {
    id: previewId++,
    projectId: input.projectId,
    projectName: project?.name ?? null,
    projectColor: project?.color ?? null,
    notePath: input.notePath,
    description: input.description.trim() || "Focus block",
    startedAt: now,
    stoppedAt: null,
    source: "timer",
    billable: input.billable,
    rateSnapshot: project?.rate ?? 0,
    durationSeconds: 0,
  };
  return structuredClone(previewRunning);
}

export async function stopTimer(): Promise<TimeEntry> {
  if (isTauri()) return invoke<TimeEntry>("timer_stop");
  if (!previewRunning) throw new Error("No timer is running.");
  const now = Math.max(
    Math.floor(Date.now() / 1000),
    previewRunning.startedAt + 1,
  );
  previewRunning.stoppedAt = now;
  previewRunning.durationSeconds = now - previewRunning.startedAt;
  previewEntries.unshift(previewRunning);
  const stopped = structuredClone(previewRunning);
  previewRunning = null;
  return stopped;
}

export async function getTimerReport(
  dayStart: number,
  dayEnd: number,
): Promise<TimerReport> {
  if (isTauri()) {
    return invoke<TimerReport>("timer_report", { dayStart, dayEnd });
  }
  const entries = [
    ...(previewRunning ? [withLiveDuration(previewRunning)] : []),
    ...previewEntries,
  ].filter(
    (entry) =>
      entry.startedAt < dayEnd &&
      (entry.stoppedAt ?? Date.now() / 1000) > dayStart,
  );
  return {
    totalSeconds: entries.reduce(
      (total, entry) => total + entry.durationSeconds,
      0,
    ),
    billableSeconds: entries
      .filter((entry) => entry.billable)
      .reduce((total, entry) => total + entry.durationSeconds, 0),
    amount: entries
      .filter((entry) => entry.billable)
      .reduce(
        (total, entry) =>
          total + (entry.durationSeconds / 3600) * entry.rateSnapshot,
        0,
      ),
    entries,
  };
}

export async function addManualEntry(input: {
  projectId: number | null;
  description: string;
  startedAt: number;
  stoppedAt: number;
  billable: boolean;
}): Promise<TimeEntry> {
  if (isTauri()) return invoke<TimeEntry>("timer_add_manual", input);
  if (input.stoppedAt <= input.startedAt) {
    throw new Error("The end time must be after the start time.");
  }
  const project = previewProjects.find((item) => item.id === input.projectId);
  const entry: TimeEntry = {
    id: previewId++,
    projectId: input.projectId,
    projectName: project?.name ?? null,
    projectColor: project?.color ?? null,
    notePath: null,
    description: input.description.trim() || "Manual entry",
    startedAt: input.startedAt,
    stoppedAt: input.stoppedAt,
    source: "manual",
    billable: input.billable,
    rateSnapshot: project?.rate ?? 0,
    durationSeconds: input.stoppedAt - input.startedAt,
  };
  previewEntries.unshift(entry);
  return structuredClone(entry);
}

export async function getIdleSeconds(): Promise<number> {
  if (isTauri()) return invoke<number>("idle_seconds");
  return 0;
}

export async function reconcileIdle(
  idleSeconds: number,
  decision: "keep" | "discard" | "reassign",
  targetProjectId: number | null,
): Promise<TimeEntry | null> {
  if (isTauri()) {
    return invoke<TimeEntry | null>("timer_reconcile_idle", {
      idleSeconds,
      decision,
      targetProjectId,
    });
  }
  return previewRunning ? structuredClone(previewRunning) : null;
}

function withLiveDuration(entry: TimeEntry): TimeEntry {
  return {
    ...structuredClone(entry),
    durationSeconds:
      (entry.stoppedAt ?? Math.floor(Date.now() / 1000)) - entry.startedAt,
  };
}

export function formatDuration(totalSeconds: number): string {
  const safeSeconds = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(safeSeconds / 3600);
  const minutes = Math.floor((safeSeconds % 3600) / 60);
  const seconds = safeSeconds % 60;
  return hours > 0
    ? `${String(hours).padStart(2, "0")}:${String(minutes).padStart(
        2,
        "0",
      )}:${String(seconds).padStart(2, "0")}`
    : `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

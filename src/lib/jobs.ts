import { invoke, isTauri } from "@tauri-apps/api/core";

export type JobStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export interface JobRecord {
  id: string;
  kind: string;
  status: JobStatus;
  progress: number;
  outputPath: string | null;
  error: string | null;
  createdAt: string;
}

export interface SafetyRecord {
  id: number;
  opKind: string;
  outputPath: string;
  reversible: boolean;
  createdAt: string;
  undoneAt: string | null;
}

const previewJobs: JobRecord[] = [];
const previewSafety: SafetyRecord[] = [];

export async function runJob(
  kind: string,
  input: Record<string, unknown>,
): Promise<JobRecord> {
  if (isTauri()) {
    return invoke<JobRecord>("job_run", { kind, input });
  }

  const id = `preview-${Date.now()}`;
  const job: JobRecord = {
    id,
    kind,
    status: "running",
    progress: 0.28,
    outputPath: `/Users/local/Library/Application Support/Tooldeck/exports/${kind}-preview`,
    error: null,
    createdAt: new Date().toISOString(),
  };
  previewJobs.unshift(job);
  previewSafety.unshift({
    id: Date.now(),
    opKind: kind,
    outputPath: job.outputPath ?? "",
    reversible: true,
    createdAt: job.createdAt,
    undoneAt: null,
  });
  window.setTimeout(() => {
    const current = previewJobs.find((candidate) => candidate.id === id);
    if (!current || current.status === "cancelled") return;
    current.status = "completed";
    current.progress = 1;
    window.dispatchEvent(new CustomEvent("tooldeck:preview-job-updated"));
  }, 900);
  window.dispatchEvent(new CustomEvent("tooldeck:preview-job-updated"));
  return job;
}

export async function listSafety(): Promise<SafetyRecord[]> {
  if (isTauri()) return invoke<SafetyRecord[]>("safety_list");
  return previewSafety.map((record) => ({ ...record }));
}

export async function undoSafety(id: number): Promise<void> {
  if (isTauri()) {
    await invoke("safety_undo", { id });
    return;
  }
  const record = previewSafety.find((candidate) => candidate.id === id);
  if (record && !record.undoneAt) record.undoneAt = new Date().toISOString();
}

export async function listJobs(): Promise<JobRecord[]> {
  if (isTauri()) return invoke<JobRecord[]>("jobs_list");
  return previewJobs.map((job) => ({ ...job }));
}

export async function cancelJob(id: string): Promise<void> {
  if (isTauri()) {
    await invoke("job_cancel", { id });
    return;
  }
  const job = previewJobs.find((candidate) => candidate.id === id);
  if (job) {
    job.status = "cancelled";
    job.progress = 0;
    window.dispatchEvent(new CustomEvent("tooldeck:preview-job-updated"));
  }
}

export function jobLabel(kind: string): string {
  return kind
    .split("-")
    .map((part) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
    .join(" ");
}

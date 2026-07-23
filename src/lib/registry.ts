import { invoke, isTauri } from "@tauri-apps/api/core";

import { runJob } from "./jobs";

export type Category =
  | "notes"
  | "time"
  | "pdf"
  | "capture"
  | "images"
  | "media"
  | "files"
  | "dev"
  | "create"
  | "workflow";

export type Tier = "core" | "fast-follow" | "later";

export interface ToolMeta {
  id: string;
  name: string;
  summary: string;
  category: Category;
  tier: Tier;
  icon: string;
  keywords: string[];
  form: FormSchema | null;
  destructive: boolean;
  longRunning: boolean;
}

export interface FormSchema {
  fields: Field[];
}

export interface Field {
  key: string;
  label: string;
  type: FieldKind;
  advanced?: boolean;
  default?: unknown;
  help?: string;
}

export type FieldKind =
  | { control: "file-picker"; multiple: boolean; extensions: string[] }
  | { control: "folder-picker" }
  | { control: "text"; placeholder?: string }
  | { control: "textarea"; placeholder?: string }
  | { control: "number"; min?: number; max?: number; step?: number }
  | { control: "toggle" }
  | { control: "select"; options: SelectOption[] }
  | { control: "slider"; min: number; max: number; step: number };

export interface SelectOption {
  value: string;
  label: string;
}

export type ToolOutcome =
  | { kind: "done"; message: string; outputPath: string | null }
  | { kind: "job"; jobId: string };

export type ToolInput = Record<string, unknown>;

export interface DbStatus {
  path: string;
  schemaVersion: number;
}

export async function fetchCatalog(): Promise<ToolMeta[]> {
  if (isTauri()) {
    return invoke<ToolMeta[]>("tool_catalog");
  }
  const response = await fetch("/dev-catalog.json");
  if (!response.ok)
    throw new Error("The local preview catalog could not be loaded.");
  return response.json() as Promise<ToolMeta[]>;
}

export async function fetchDbStatus(): Promise<DbStatus | null> {
  if (!isTauri()) return null;
  return invoke<DbStatus>("db_status");
}

export async function runTool(
  id: string,
  input: ToolInput,
): Promise<ToolOutcome> {
  const jobKinds: Record<string, string> = {
    "pdf-merge": "pdf-merge",
    "pdf-extract-text": "pdf-extract-text",
    "pdf-split": "pdf-split",
    "pdf-reorder": "pdf-reorder",
    "pdf-delete-pages": "pdf-delete-pages",
    "pdf-rotate": "pdf-rotate",
    "pdf-compress": "pdf-compress",
    "pdf-to-images": "pdf-to-images",
    "images-to-pdf": "images-to-pdf",
    "pdf-watermark": "pdf-watermark",
    "pdf-page-numbers": "pdf-page-numbers",
    "pdf-redact": "pdf-redact",
    "ocr-image": "ocr-image",
    "ocr-pdf": "ocr-pdf",
    "export-document": "export-document",
    "image-resize": "image-resize",
    "video-trim": "video-trim",
  };
  const jobKind = jobKinds[id];
  if (jobKind) {
    const job = await runJob(jobKind, input);
    return { kind: "job", jobId: job.id };
  }
  if (isTauri()) {
    return invoke<ToolOutcome>("tool_run", { id, input });
  }
  await new Promise((resolve) => window.setTimeout(resolve, 360));
  return {
    kind: "done",
    message: `Preview completed for ${id.replaceAll("-", " ")}.`,
    outputPath: null,
  };
}

export function scoreTool(tool: ToolMeta, query: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 0;
  const name = tool.name.toLowerCase();
  const haystack = [tool.name, tool.category, ...tool.keywords]
    .join(" ")
    .toLowerCase();

  if (name === q) return 4;
  if (name.startsWith(q)) return 3 + q.length / name.length;
  if (haystack.includes(q)) return 2 + q.length / haystack.length;

  let queryIndex = 0;
  let gaps = 0;
  for (
    let index = 0;
    index < haystack.length && queryIndex < q.length;
    index++
  ) {
    if (haystack[index] === q[queryIndex]) queryIndex++;
    else if (queryIndex > 0) gaps++;
  }

  return queryIndex === q.length ? Math.max(0.1, 1 - gaps / 100) : -1;
}

export function searchCatalog(catalog: ToolMeta[], query: string): ToolMeta[] {
  const normalized = query.trim();
  if (!normalized) return catalog;
  return catalog
    .map((tool) => ({ tool, score: scoreTool(tool, normalized) }))
    .filter(({ score }) => score >= 0)
    .sort((left, right) => right.score - left.score)
    .map(({ tool }) => tool);
}

export function byCategory(
  catalog: ToolMeta[],
): Partial<Record<Category, ToolMeta[]>> {
  const groups: Partial<Record<Category, ToolMeta[]>> = {};
  for (const tool of catalog) {
    (groups[tool.category] ??= []).push(tool);
  }
  return groups;
}

export const PROFILE_PINS: Record<string, Category[]> = {
  freelancer: ["time", "create", "notes", "pdf"],
  designer: ["images", "capture", "media", "notes"],
  writer: ["notes", "create", "pdf"],
  developer: ["dev", "files", "notes", "capture"],
};

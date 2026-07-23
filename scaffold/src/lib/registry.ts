/**
 * Tooldeck tool registry — frontend types (handoff §15.1, §16).
 *
 * These mirror the Rust `ToolMeta` / `FormSchema` in src-tauri/src/registry.rs.
 * The frontend NEVER hardcodes the tool list: it calls the `tool_catalog` command
 * once, then generates the command palette, the Home grid, and the category rail
 * from the returned `ToolMeta[]`. Running a tool = render its `FormSchema`, collect
 * values, invoke `tool_run`.
 *
 * Keep this file in sync with the Rust side. (Optionally auto-generate it via
 * `tauri-specta` / `ts-rs` instead of hand-maintaining — recommended once stable.)
 */

import { invoke } from "@tauri-apps/api/core";

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
  icon: string; // Lucide icon name
  keywords: string[];
  form: FormSchema | null; // null => tool ships a custom React view keyed by id
  destructive: boolean;
  longRunning: boolean;
}

// ---- Declarative form (mirrors Rust FormSchema) ----------------------------

export interface FormSchema {
  fields: Field[];
}

export interface Field {
  key: string;
  label: string;
  kind: FieldKind;
  advanced?: boolean; // collapsed under "▸ Advanced" by default (§16.3)
  default?: unknown;
  help?: string;
}

export type FieldKind =
  | { control: "file-picker"; multiple: boolean; extensions: string[] }
  | { control: "folder-picker" }
  | { control: "text"; placeholder?: string }
  | { control: "number"; min?: number; max?: number; step?: number }
  | { control: "toggle" }
  | { control: "select"; options: SelectOption[] }
  | { control: "slider"; min: number; max: number; step: number };

export interface SelectOption {
  value: string;
  label: string;
}

// ---- Run outcomes (mirror Rust ToolOutcome) --------------------------------

export type ToolOutcome =
  | { kind: "done"; message: string; outputPath: string | null }
  | { kind: "job"; jobId: string };

export type ToolInput = Record<string, unknown>;

// ---- IPC wrappers ----------------------------------------------------------

export async function fetchCatalog(): Promise<ToolMeta[]> {
  return invoke<ToolMeta[]>("tool_catalog");
}

export async function runTool(id: string, input: ToolInput): Promise<ToolOutcome> {
  return invoke<ToolOutcome>("tool_run", { id, input });
}

// ---- Palette / grid helpers ------------------------------------------------

/** Lightweight fuzzy score: matches on name, category, and keywords (§16.1). */
export function scoreTool(tool: ToolMeta, query: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 0;
  const hay = [tool.name, tool.category, ...tool.keywords].join(" ").toLowerCase();
  if (hay.includes(q)) return q.length / hay.length + (tool.name.toLowerCase().startsWith(q) ? 1 : 0);
  // simple subsequence match so "rszim" still finds "Resize Image"
  let i = 0;
  for (const ch of hay) if (ch === q[i]) i++;
  return i === q.length ? 0.1 : -1;
}

export function searchCatalog(catalog: ToolMeta[], query: string): ToolMeta[] {
  return catalog
    .map((t) => ({ t, s: scoreTool(t, query) }))
    .filter((x) => x.s >= 0)
    .sort((a, b) => b.s - a.s)
    .map((x) => x.t);
}

/** Group by category for the rail / browse grids. */
export function byCategory(catalog: ToolMeta[]): Record<Category, ToolMeta[]> {
  const out = {} as Record<Category, ToolMeta[]>;
  for (const t of catalog) (out[t.category] ??= []).push(t);
  return out;
}

/**
 * Persona profiles decide which categories are pinned vs. collapsed on first run (§16.4).
 * Nothing is ever hidden from search — this only affects the default rail/Home layout.
 */
export const PROFILE_PINS: Record<string, Category[]> = {
  freelancer: ["time", "create", "notes", "pdf"],
  designer: ["images", "capture", "media", "notes"],
  writer: ["notes", "create", "pdf"],
  developer: ["dev", "files", "notes", "capture"],
};

import { invoke, isTauri } from "@tauri-apps/api/core";

import type { ToolInput } from "./registry";

export interface WorkflowRunResult {
  status: "job" | "nextTool" | "done" | "degraded";
  message: string;
  jobId: string | null;
  nextToolId: string | null;
  prefill: ToolInput | null;
  context: Record<string, unknown> | null;
}

export interface ActivityPermission {
  appNamesAvailable: boolean;
  windowTitlesAvailable: boolean;
  detail: string;
}

export interface ActivitySample {
  appName: string;
  windowTitle: string | null;
  permission: ActivityPermission;
  recorded: boolean;
  timeEntryId: number | null;
}

const previewPermission: ActivityPermission = {
  appNamesAvailable: true,
  windowTitlesAvailable: false,
  detail:
    "App names are available. Window titles stay off until Screen Recording access is enabled.",
};

export async function runWorkflow(
  id: string,
  input: ToolInput,
): Promise<WorkflowRunResult> {
  if (isTauri()) {
    return invoke<WorkflowRunResult>("workflow_run", { id, input });
  }
  await new Promise((resolve) => window.setTimeout(resolve, 360));
  const required: Partial<Record<string, string>> = {
    "workflow-research": "source_text",
    "workflow-context-switch": "project_id",
    "workflow-standup": "bullet",
    "workflow-capture-attach": "note_path",
    "workflow-media-inline": "video_path",
  };
  const missingKey = required[id];
  if (missingKey && !input[missingKey]) {
    return {
      status: "degraded",
      message: "Add the missing local input, then run this workflow again.",
      jobId: null,
      nextToolId: null,
      prefill: null,
      context: null,
    };
  }
  if (id === "workflow-capture-attach") {
    return {
      status: "nextTool",
      message: "Target note is ready. Choose a region in the capture tool.",
      jobId: null,
      nextToolId: "screenshot-capture",
      prefill: { note_path: input.note_path },
      context: null,
    };
  }
  const isJob = [
    "workflow-billable-document",
    "workflow-documentation",
    "workflow-media-inline",
  ].includes(id);
  return {
    status: isJob ? "job" : "done",
    message: isJob
      ? "The local workflow is now running in Output Center."
      : "The local workflow finished and linked its context.",
    jobId: isJob ? `preview-${Date.now()}` : null,
    nextToolId: null,
    prefill: null,
    context: null,
  };
}

export async function getActivityPermission(): Promise<ActivityPermission> {
  if (isTauri()) {
    return invoke<ActivityPermission>("activity_permission_status");
  }
  return previewPermission;
}

export async function requestActivityPermission(): Promise<ActivityPermission> {
  if (isTauri()) {
    return invoke<ActivityPermission>("activity_request_permission");
  }
  return previewPermission;
}

export async function sampleActivity(): Promise<ActivitySample | null> {
  if (isTauri()) return invoke<ActivitySample>("activity_sample");
  return null;
}

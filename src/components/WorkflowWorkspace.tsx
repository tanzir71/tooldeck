import { useEffect, useMemo, useState } from "react";

import { listNotes, type NoteSummary } from "../lib/notes";
import type { Field, ToolInput, ToolMeta } from "../lib/registry";
import { listProjects, type Project } from "../lib/time";
import { runWorkflow, type WorkflowRunResult } from "../lib/workflows";
import { useAppStore } from "../stores/app";
import { ActivityPermissionPanel } from "./ActivityPermissionPanel";
import { Icon } from "./Icon";
import { Shortcut } from "./Shortcut";
import { WorkflowFields } from "./WorkflowFields";

interface WorkflowWorkspaceProps {
  tool: ToolMeta;
}

function selectField(
  key: string,
  label: string,
  options: { value: string; label: string }[],
  help?: string,
): Field {
  return {
    key,
    label,
    type: { control: "select", options },
    help,
  };
}

function textField(
  key: string,
  label: string,
  placeholder: string,
  textarea = false,
): Field {
  return {
    key,
    label,
    type: textarea
      ? { control: "textarea", placeholder }
      : { control: "text", placeholder },
  };
}

function fieldsFor(
  id: string,
  projects: Project[],
  notes: NoteSummary[],
): Field[] {
  const noteOptions = [
    { value: "", label: "Choose a note…" },
    ...notes.map((note) => ({ value: note.path, label: note.title })),
  ];
  switch (id) {
    case "workflow-billable-document":
      return [
        selectField("template", "Deliverable", [
          { value: "invoice", label: "Invoice" },
          { value: "timesheet", label: "Timesheet / activity report" },
        ]),
        {
          key: "tax",
          label: "Tax",
          type: { control: "number", min: 0, max: 100, step: 1 },
          default: 0,
          help: "Percent · invoice only",
        },
      ];
    case "workflow-documentation":
      return [
        selectField(
          "note_path",
          "Project note",
          [
            { value: "", label: "Use most recent note" },
            ...noteOptions.slice(1),
          ],
          "Text, checklists, and attachment links stay together.",
        ),
      ];
    case "workflow-research":
      return [
        textField("source_name", "Source", "Paper.pdf · page 4"),
        textField(
          "source_text",
          "Extracted or OCR text",
          "Paste the local excerpt to capture…",
          true,
        ),
      ];
    case "workflow-context-switch":
      return [
        selectField(
          "project_id",
          "Active project",
          [
            { value: "", label: "Choose a project…" },
            ...projects.map((project) => ({
              value: String(project.id),
              label: `${project.clientName} · ${project.name}`,
            })),
          ],
          "Updates the shared context used by notes, timers, and capture.",
        ),
      ];
    case "workflow-standup":
      return [
        textField(
          "bullet",
          "Timestamped update",
          "Shipped the offline document pipeline…",
          true,
        ),
      ];
    case "workflow-capture-attach":
      return [selectField("note_path", "Target note", noteOptions)];
    case "workflow-media-inline":
      return [
        {
          key: "video_path",
          label: "Short video",
          type: {
            control: "file-picker",
            multiple: false,
            extensions: ["mp4", "mov", "m4v", "webm"],
          },
          help: "The source stays untouched.",
        },
        selectField(
          "note_path",
          "Attach to note",
          noteOptions,
          "Optional · the finished GIF is copied into attachments.",
        ),
        {
          key: "duration",
          label: "Duration",
          type: { control: "number", min: 0.1, max: 15, step: 0.1 },
          default: 8,
          help: "Seconds · capped at 15",
        },
      ];
    default:
      return [];
  }
}

function initialValues(fields: Field[]): ToolInput {
  return Object.fromEntries(
    fields.map((field) => [
      field.key,
      field.default ?? (field.type.control === "toggle" ? false : ""),
    ]),
  );
}

export function WorkflowWorkspace({ tool }: WorkflowWorkspaceProps) {
  const catalog = useAppStore((state) => state.catalog);
  const selectTool = useAppStore((state) => state.selectTool);
  const setCategory = useAppStore((state) => state.setCategory);
  const setOutputCenterOpen = useAppStore((state) => state.setOutputCenterOpen);
  const [projects, setProjects] = useState<Project[]>([]);
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [values, setValues] = useState<ToolInput>({});
  const [result, setResult] = useState<WorkflowRunResult | null>(null);
  const [running, setRunning] = useState(false);
  const fields = useMemo(
    () => fieldsFor(tool.id, projects, notes),
    [notes, projects, tool.id],
  );

  useEffect(() => {
    let cancelled = false;
    void Promise.all([listProjects(), listNotes()]).then(
      ([projectItems, noteItems]) => {
        if (!cancelled) {
          setProjects(projectItems);
          setNotes(noteItems);
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    setValues(initialValues(fields));
    setResult(null);
  }, [fields]);

  async function execute() {
    if (running) return;
    setRunning(true);
    setResult(null);
    try {
      const normalized = {
        ...values,
        project_id: values.project_id ? Number(values.project_id) : undefined,
      };
      const nextResult = await runWorkflow(tool.id, normalized);
      setResult(nextResult);
      if (nextResult.status === "job") setOutputCenterOpen(true);
    } catch (error) {
      setResult({
        status: "degraded",
        message:
          error instanceof Error
            ? error.message
            : "The workflow could not finish locally.",
        jobId: null,
        nextToolId: null,
        prefill: null,
        context: null,
      });
    } finally {
      setRunning(false);
    }
  }

  function continueToTool() {
    if (!result?.nextToolId) return;
    const nextTool = catalog.find((item) => item.id === result.nextToolId);
    if (nextTool) selectTool(nextTool, result.prefill ?? undefined);
  }

  return (
    <main className="workspace tool-workspace workflow-workspace">
      <button
        type="button"
        className="back-button"
        onClick={() => setCategory("workflow")}
      >
        <Icon name="chevron-right" size={13} className="back-chevron" />
        workflows
      </button>
      <header className="tool-header">
        <span className="tool-title-icon category-workflow">
          <Icon name={tool.icon} size={22} />
        </span>
        <div>
          <p className="eyebrow">Connected workflow</p>
          <h1>{tool.name}</h1>
          <p>{tool.summary}</p>
        </div>
      </header>

      <section className="workflow-steps" aria-label="Workflow stages">
        <span>1 · Gather local context</span>
        <Icon name="chevron-right" size={13} />
        <span>2 · Link or process</span>
        <Icon name="chevron-right" size={13} />
        <span>3 · Save locally</span>
      </section>

      {tool.id === "workflow-context-switch" && <ActivityPermissionPanel />}

      <section className="tool-form" aria-label={`${tool.name} inputs`}>
        <WorkflowFields
          fields={fields}
          values={values}
          onChange={(key, value) =>
            setValues((current) => ({ ...current, [key]: value }))
          }
        />
        {result && (
          <div
            className={`inline-result ${
              result.status === "degraded" ? "warning" : "success"
            }`}
            role={result.status === "degraded" ? "alert" : "status"}
          >
            <Icon
              name={
                result.status === "degraded" ? "circle-alert" : "circle-check"
              }
              size={17}
            />
            <div>
              <strong>
                {result.status === "degraded"
                  ? "Needs one more thing"
                  : result.status === "job"
                    ? "Running locally"
                    : "Context connected"}
              </strong>
              <span>{result.message}</span>
              {result.status === "nextTool" && (
                <button
                  type="button"
                  className="inline-link"
                  onClick={continueToTool}
                >
                  Continue to capture
                </button>
              )}
            </div>
          </div>
        )}
      </section>

      <footer className="tool-actionbar">
        <div className="safety-note">
          <Icon name="shield" size={13} />
          Explicit steps · local data only
        </div>
        <button
          className="primary-button run-button"
          type="button"
          disabled={running}
          onClick={() => void execute()}
        >
          <Icon
            className={running ? "spin" : undefined}
            name={running ? "loader" : "play"}
            size={14}
          />
          {running ? "Running…" : "Run workflow"}
          <Shortcut>⌘↵</Shortcut>
        </button>
      </footer>
    </main>
  );
}

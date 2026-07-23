import { useState } from "react";

import type { ToolInput, ToolMeta, ToolOutcome } from "../lib/registry";
import { runTool } from "../lib/registry";
import { useAppStore } from "../stores/app";
import { FieldControl } from "./FieldControl";
import { Icon } from "./Icon";
import { Shortcut } from "./Shortcut";

interface ToolWorkspaceProps {
  tool: ToolMeta;
}

type RunState =
  | { kind: "idle" }
  | { kind: "running" }
  | { kind: "success"; outcome: ToolOutcome }
  | { kind: "error"; message: string };

function initialValues(tool: ToolMeta): ToolInput {
  const values: ToolInput = {};
  for (const field of tool.form?.fields ?? []) {
    if (field.default !== undefined) values[field.key] = field.default;
    else if (field.type.control === "toggle") values[field.key] = false;
    else values[field.key] = "";
  }
  return values;
}

export function ToolWorkspace({ tool }: ToolWorkspaceProps) {
  const setCategory = useAppStore((state) => state.setCategory);
  const [values, setValues] = useState<ToolInput>(() => initialValues(tool));
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [runState, setRunState] = useState<RunState>({ kind: "idle" });
  const fields = tool.form?.fields ?? [];
  const defaultFields = fields.filter((field) => !field.advanced);
  const advancedFields = fields.filter((field) => field.advanced);

  const canRun = defaultFields
    .filter(
      (field) =>
        field.type.control === "file-picker" ||
        field.type.control === "folder-picker",
    )
    .every((field) => {
      const value = values[field.key];
      return Array.isArray(value) ? value.length > 0 : Boolean(value);
    });

  async function execute() {
    if (!canRun || runState.kind === "running") return;
    setRunState({ kind: "running" });
    try {
      const outcome = await runTool(tool.id, values);
      setRunState({ kind: "success", outcome });
    } catch (error) {
      setRunState({
        kind: "error",
        message:
          error instanceof Error
            ? error.message
            : "This tool could not finish.",
      });
    }
  }

  return (
    <main className="workspace tool-workspace">
      <button
        type="button"
        className="back-button"
        onClick={() => setCategory(tool.category)}
      >
        <Icon name="chevron-right" size={13} className="back-chevron" />
        {tool.category}
      </button>
      <header className="tool-header">
        <span className={`tool-title-icon category-${tool.category}`}>
          <Icon name={tool.icon} size={22} />
        </span>
        <div>
          <p className="eyebrow">{tool.category}</p>
          <h1>{tool.name}</h1>
          <p>{tool.summary}</p>
        </div>
      </header>

      <section className="tool-form" aria-label={`${tool.name} settings`}>
        {defaultFields.map((field) => (
          <div className="field" key={field.key}>
            <div className="field-label">
              <label>{field.label}</label>
              {field.help && <span>{field.help}</span>}
            </div>
            <FieldControl
              field={field}
              value={values[field.key]}
              onChange={(value) =>
                setValues((current) => ({ ...current, [field.key]: value }))
              }
            />
          </div>
        ))}

        {advancedFields.length > 0 && (
          <div className="advanced">
            <button
              type="button"
              className="advanced-trigger"
              onClick={() => setAdvancedOpen((openState) => !openState)}
              aria-expanded={advancedOpen}
            >
              <Icon
                name={advancedOpen ? "chevron-down" : "chevron-right"}
                size={14}
              />
              Advanced
              <span>{advancedFields.length} optional</span>
            </button>
            {advancedOpen && (
              <div className="advanced-fields">
                {advancedFields.map((field) => (
                  <div
                    className={`field ${
                      field.type.control === "toggle" ? "field-inline" : ""
                    }`}
                    key={field.key}
                  >
                    <div className="field-label">
                      <label>{field.label}</label>
                      {field.help && <span>{field.help}</span>}
                    </div>
                    <FieldControl
                      field={field}
                      value={values[field.key]}
                      onChange={(value) =>
                        setValues((current) => ({
                          ...current,
                          [field.key]: value,
                        }))
                      }
                    />
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {runState.kind === "success" && (
          <div className="inline-result success" role="status">
            <Icon name="circle-check" size={17} />
            <div>
              <strong>Finished</strong>
              <span>
                {runState.outcome.kind === "done"
                  ? runState.outcome.message
                  : "Added to Output Center."}
              </span>
            </div>
          </div>
        )}
        {runState.kind === "error" && (
          <div className="inline-result error" role="alert">
            <Icon name="circle-alert" size={17} />
            <div>
              <strong>Couldn’t finish</strong>
              <span>
                {runState.message} Try again without changing the original.
              </span>
            </div>
          </div>
        )}
      </section>

      <footer className="tool-actionbar">
        <div className="safety-note">
          <Icon name="shield" size={13} />
          Originals stay untouched
        </div>
        <button
          className="primary-button run-button"
          type="button"
          disabled={!canRun || runState.kind === "running"}
          onClick={() => void execute()}
        >
          {runState.kind === "running" ? (
            <Icon className="spin" name="loader" size={15} />
          ) : (
            <Icon name="play" size={14} fill="currentColor" />
          )}
          {runState.kind === "running" ? "Running…" : "Run"}
          <Shortcut>⌘↵</Shortcut>
        </button>
      </footer>
    </main>
  );
}

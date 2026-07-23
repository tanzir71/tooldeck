import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useRef } from "react";

import type { Field } from "../lib/registry";
import { Icon } from "./Icon";

interface FieldControlProps {
  field: Field;
  value: unknown;
  onChange: (value: unknown) => void;
}

export function FieldControl({ field, value, onChange }: FieldControlProps) {
  const browserFileInput = useRef<HTMLInputElement>(null);
  const kind = field.type;

  async function chooseFile() {
    if (!isTauri()) {
      browserFileInput.current?.click();
      return;
    }
    const selection = await open({
      multiple: kind.control === "file-picker" && kind.multiple,
      directory: kind.control === "folder-picker",
      filters:
        kind.control === "file-picker"
          ? [{ name: field.label, extensions: kind.extensions }]
          : undefined,
    });
    if (selection) onChange(selection);
  }

  if (kind.control === "file-picker" || kind.control === "folder-picker") {
    const selected = Array.isArray(value)
      ? value.map(String)
      : typeof value === "string" && value
        ? [value]
        : [];
    return (
      <div className="file-control">
        <button
          type="button"
          className={`drop-zone ${selected.length > 0 ? "has-file" : ""}`}
          onClick={() => void chooseFile()}
        >
          <span className="drop-icon">
            <Icon
              name={selected.length > 0 ? "circle-check" : "folder"}
              size={20}
            />
          </span>
          <span>
            <strong>
              {selected.length > 0
                ? selected.map((path) => path.split("/").at(-1)).join(", ")
                : kind.control === "folder-picker"
                  ? "Choose a folder"
                  : kind.multiple
                    ? "Choose files"
                    : "Choose a file"}
            </strong>
            <small>
              {selected.length > 0
                ? "Ready to run · choose again to replace"
                : kind.control === "file-picker"
                  ? kind.extensions.map((item) => item.toUpperCase()).join(", ")
                  : "Nothing is changed until you run"}
            </small>
          </span>
        </button>
        <input
          ref={browserFileInput}
          hidden
          type="file"
          multiple={kind.control === "file-picker" && kind.multiple}
          accept={
            kind.control === "file-picker"
              ? kind.extensions.map((extension) => `.${extension}`).join(",")
              : undefined
          }
          onChange={(event) => {
            const names = Array.from(event.target.files ?? []).map(
              (file) => file.name,
            );
            onChange(
              kind.control === "file-picker" && kind.multiple
                ? names
                : names[0],
            );
          }}
        />
      </div>
    );
  }

  if (kind.control === "toggle") {
    return (
      <button
        className={`toggle ${value ? "on" : ""}`}
        type="button"
        role="switch"
        aria-checked={Boolean(value)}
        onClick={() => onChange(!value)}
      >
        <span />
      </button>
    );
  }

  if (kind.control === "select") {
    return (
      <select
        value={String(value ?? "")}
        onChange={(event) => onChange(event.target.value)}
      >
        {kind.options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    );
  }

  if (kind.control === "slider") {
    return (
      <div className="slider-row">
        <input
          type="range"
          value={Number(value ?? kind.min)}
          min={kind.min}
          max={kind.max}
          step={kind.step}
          onChange={(event) => onChange(Number(event.target.value))}
        />
        <output>{String(value)}</output>
      </div>
    );
  }

  if (kind.control === "number") {
    return (
      <input
        type="number"
        value={Number(value ?? 0)}
        min={kind.min}
        max={kind.max}
        step={kind.step}
        onChange={(event) => onChange(event.target.valueAsNumber)}
      />
    );
  }

  if (kind.control === "textarea") {
    return (
      <textarea
        value={String(value ?? "")}
        placeholder={kind.placeholder}
        rows={6}
        onChange={(event) => onChange(event.target.value)}
      />
    );
  }

  return (
    <input
      type="text"
      value={String(value ?? "")}
      placeholder={kind.placeholder}
      onChange={(event) => onChange(event.target.value)}
    />
  );
}

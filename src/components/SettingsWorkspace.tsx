import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";

import { getNotesVault, setNotesVault } from "../lib/notes";
import type { Persona } from "../lib/preferences";
import { fetchDbStatus } from "../lib/registry";
import {
  getActivityPermission,
  requestActivityPermission,
  type ActivityPermission,
} from "../lib/workflows";
import { useAppStore } from "../stores/app";
import { Icon } from "./Icon";

const profileLabels: Record<Persona, string> = {
  freelancer: "Freelancer",
  designer: "Designer",
  writer: "Writer",
  developer: "Developer",
};

interface PermissionRowProps {
  capability: string;
  access: string;
  state: string;
  active?: boolean;
}

function PermissionRow({
  capability,
  access,
  state,
  active = true,
}: PermissionRowProps) {
  return (
    <div className="permission-row">
      <span className={`permission-dot ${active ? "active" : ""}`} />
      <div>
        <strong>{capability}</strong>
        <small>{access}</small>
      </div>
      <span>{state}</span>
    </div>
  );
}

export function SettingsWorkspace() {
  const setSettingsOpen = useAppStore((state) => state.setSettingsOpen);
  const setOnboardingOpen = useAppStore((state) => state.setOnboardingOpen);
  const persona = useAppStore((state) => state.persona);
  const setPersona = useAppStore((state) => state.setPersona);
  const density = useAppStore((state) => state.density);
  const setDensity = useAppStore((state) => state.setDensity);
  const theme = useAppStore((state) => state.theme);
  const setTheme = useAppStore((state) => state.setTheme);
  const [vault, setVault] = useState("Loading…");
  const [dataPath, setDataPath] = useState("Local application support");
  const [permission, setPermission] = useState<ActivityPermission | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      getNotesVault(),
      fetchDbStatus(),
      getActivityPermission(),
    ]).then(([vaultPath, database, permissionValue]) => {
      if (!cancelled) {
        setVault(vaultPath);
        setDataPath(database?.path ?? "Browser preview · no database");
        setPermission(permissionValue);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  async function chooseVault() {
    setMessage(null);
    try {
      const selected = isTauri()
        ? await open({ directory: true, multiple: false })
        : "/Users/you/Documents/Tooldeck Notes";
      if (typeof selected === "string") {
        setVault(await setNotesVault(selected));
        setMessage(
          "Notes vault switched and reindexed. The previous folder was untouched.",
        );
      }
    } catch (error) {
      setMessage(
        error instanceof Error ? error.message : "The vault could not change.",
      );
    }
  }

  async function enableTitles() {
    setPermission(await requestActivityPermission());
  }

  return (
    <main className="workspace settings-workspace">
      <header className="settings-header">
        <div>
          <p className="eyebrow">Tooldeck</p>
          <h1>Settings</h1>
          <p>Personalize the surface without changing where your data lives.</p>
        </div>
        <button
          className="secondary-button"
          type="button"
          onClick={() => setSettingsOpen(false)}
        >
          Done
        </button>
      </header>

      <section className="settings-section">
        <div className="settings-section-copy">
          <Icon name="sparkles" size={17} />
          <div>
            <h2>Appearance</h2>
            <p>System-aligned defaults with an optional compact view.</p>
          </div>
        </div>
        <div className="settings-controls">
          <label>
            <span>Profile</span>
            <select
              value={persona}
              onChange={(event) => setPersona(event.target.value as Persona)}
            >
              {Object.entries(profileLabels).map(([value, label]) => (
                <option value={value} key={value}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Theme</span>
            <select
              value={theme}
              onChange={(event) =>
                setTheme(event.target.value as "system" | "light" | "dark")
              }
            >
              <option value="system">Follow system</option>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
            </select>
          </label>
          <label>
            <span>Density</span>
            <select
              value={density}
              onChange={(event) =>
                setDensity(event.target.value as "comfortable" | "compact")
              }
            >
              <option value="comfortable">Comfortable</option>
              <option value="compact">Compact</option>
            </select>
          </label>
        </div>
      </section>

      <section className="settings-section">
        <div className="settings-section-copy">
          <Icon name="folder" size={17} />
          <div>
            <h2>Notes vault</h2>
            <p>Plain Markdown. Switching folders never removes the old one.</p>
          </div>
        </div>
        <div className="settings-path-card">
          <div>
            <strong>{vault.split("/").at(-1)}</strong>
            <span>{vault}</span>
          </div>
          <button
            type="button"
            className="secondary-button"
            onClick={() => void chooseVault()}
          >
            Choose folder
          </button>
        </div>
        {message && (
          <p className="settings-message" role="status">
            {message}
          </p>
        )}
      </section>

      <section className="settings-section">
        <div className="settings-section-copy">
          <Icon name="shield" size={17} />
          <div>
            <h2>Privacy & permissions</h2>
            <p>Optional macOS access is requested only from a button here.</p>
          </div>
        </div>
        <div className="permission-matrix">
          <PermissionRow
            capability="App names"
            access="Foreground context"
            state="Available"
          />
          <PermissionRow
            capability="Window titles"
            access="Screen Recording"
            state={
              permission?.windowTitlesAvailable ? "Enabled" : "App-name only"
            }
            active={permission?.windowTitlesAvailable}
          />
          <PermissionRow
            capability="Idle detection"
            access="No permission"
            state="Available"
          />
          <PermissionRow
            capability="Screenshot capture"
            access="Screen Recording"
            state={
              permission?.windowTitlesAvailable ? "Enabled" : "Asked on use"
            }
            active={permission?.windowTitlesAvailable}
          />
          <PermissionRow
            capability="Files & folders"
            access="macOS folder prompts"
            state="Asked on use"
          />
        </div>
        {!permission?.windowTitlesAvailable && (
          <button
            type="button"
            className="secondary-button settings-permission-button"
            onClick={() => void enableTitles()}
          >
            Enable window titles
          </button>
        )}
      </section>

      <section className="settings-section settings-data">
        <div className="settings-section-copy">
          <Icon name="files" size={17} />
          <div>
            <h2>Local data</h2>
            <p>{dataPath}</p>
          </div>
        </div>
        <button
          type="button"
          className="text-button"
          onClick={() => {
            setSettingsOpen(false);
            setOnboardingOpen(true);
          }}
        >
          Replay setup
        </button>
      </section>

      <section className="settings-section">
        <div className="settings-section-copy">
          <Icon name="files" size={17} />
          <div>
            <h2>Components & licenses</h2>
            <p>Release inputs are reviewed and supplied separately.</p>
          </div>
        </div>
        <div className="settings-license-list">
          <div>
            <strong>FFmpeg / FFprobe</strong>
            <span>
              Release gate requires a reviewed, self-contained, LGPL-compatible
              build by default.
            </span>
          </div>
          <div>
            <strong>PDF command tools</strong>
            <span>
              Poppler distribution terms and corresponding source require human
              approval before shipping.
            </span>
          </div>
          <div>
            <strong>Offline OCR</strong>
            <span>
              Uses Apple Vision on-device; no Tesseract language pack is
              bundled.
            </span>
          </div>
        </div>
      </section>
    </main>
  );
}

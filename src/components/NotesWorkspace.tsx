import { useEffect, useMemo, useState } from "react";

import {
  createNote,
  getBacklinks,
  getDailyNote,
  getNote,
  linkNoteProject,
  listNotes,
  saveNote,
  searchNotes,
} from "../lib/notes";
import type { NoteDocument, NoteSummary } from "../lib/notes";
import { listProjects } from "../lib/time";
import type { Project } from "../lib/time";
import { Icon } from "./Icon";
import { MarkdownPreview } from "./MarkdownPreview";

type SaveState = "idle" | "saving" | "saved" | "error";
type EditorMode = "edit" | "split" | "preview";

export function NotesWorkspace() {
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [active, setActive] = useState<NoteDocument | null>(null);
  const [draft, setDraft] = useState("");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [mode, setMode] = useState<EditorMode>("split");
  const [template, setTemplate] = useState("blank");
  const [backlinks, setBacklinks] = useState<NoteSummary[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([listNotes(), listProjects()])
      .then(([items, projectItems]) => {
        if (cancelled) return;
        setNotes(items);
        setProjects(projectItems);
        if (items[0]) {
          void getNote(items[0].path).then((note) => {
            if (!cancelled) {
              setActive(note);
              setDraft(note.content);
            }
          });
        }
      })
      .catch((reason: unknown) => {
        if (!cancelled) {
          setError(reason instanceof Error ? reason.message : String(reason));
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    void getBacklinks(active.path).then((items) => {
      if (!cancelled) setBacklinks(items);
    });
    return () => {
      cancelled = true;
    };
  }, [active]);

  useEffect(() => {
    if (!query.trim()) return;
    const timeout = window.setTimeout(() => {
      void searchNotes(query)
        .then(setNotes)
        .catch((reason: unknown) =>
          setError(reason instanceof Error ? reason.message : String(reason)),
        );
    }, 180);
    return () => window.clearTimeout(timeout);
  }, [query]);

  useEffect(() => {
    if (!active || draft === active.content) return;
    const timeout = window.setTimeout(() => {
      void saveNote({ ...active, content: draft })
        .then((saved) => {
          setActive(saved);
          setSaveState("saved");
          setNotes((current) => [
            {
              path: saved.path,
              title: saved.title,
              preview:
                saved.content
                  .split("\n")
                  .find((line) => line.trim() && !line.startsWith("#")) ??
                "Empty note",
              tags: saved.tags,
              projectId: saved.projectId,
              updatedAt: saved.updatedAt,
            },
            ...current.filter((note) => note.path !== saved.path),
          ]);
        })
        .catch((reason: unknown) => {
          setSaveState("error");
          setError(reason instanceof Error ? reason.message : String(reason));
        });
    }, 520);
    return () => window.clearTimeout(timeout);
  }, [active, draft]);

  const wordCount = useMemo(
    () => draft.trim().split(/\s+/).filter(Boolean).length,
    [draft],
  );

  async function selectNote(path: string) {
    try {
      const note = await getNote(path);
      setActive(note);
      setDraft(note.content);
      setError(null);
      setSaveState("idle");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  async function makeNote() {
    try {
      const note = await createNote("Untitled note", template);
      setActive(note);
      setDraft(note.content);
      setNotes((current) => [
        {
          path: note.path,
          title: note.title,
          preview: "Empty note",
          tags: note.tags,
          projectId: note.projectId,
          updatedAt: note.updatedAt,
        },
        ...current,
      ]);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  async function openDaily() {
    try {
      const note = await getDailyNote();
      const refreshed = await listNotes();
      setNotes(refreshed);
      setActive(note);
      setDraft(note.content);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  async function clearSearch() {
    setQuery("");
    setNotes(await listNotes());
  }

  return (
    <main className="workspace notes-workspace">
      <aside className="notes-list-pane">
        <header className="notes-pane-header">
          <div>
            <p className="eyebrow">Knowledge</p>
            <h1>Notes</h1>
          </div>
          <button
            className="small-icon-button"
            type="button"
            onClick={() => void makeNote()}
            aria-label="New note"
            title="New note"
          >
            <Icon name="plus" size={15} />
          </button>
        </header>
        <div className="notes-search">
          <Icon name="search" size={13} />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search every note"
            aria-label="Search notes"
          />
          {query && (
            <button
              type="button"
              onClick={() => void clearSearch()}
              aria-label="Clear search"
            >
              <Icon name="x" size={12} />
            </button>
          )}
        </div>
        <div className="notes-quick-row">
          <button type="button" onClick={() => void openDaily()}>
            <Icon name="clock" size={12} />
            Today
          </button>
          <select
            value={template}
            onChange={(event) => setTemplate(event.target.value)}
            aria-label="New note template"
          >
            <option value="blank">Blank</option>
            <option value="meeting">Meeting</option>
            <option value="project">Project</option>
            <option value="standup">Standup</option>
          </select>
        </div>
        <div className="note-list" aria-label="Notes">
          {loading && (
            <div className="notes-list-state">
              <Icon className="spin" name="loader" size={16} />
              Indexing local notes…
            </div>
          )}
          {!loading && notes.length === 0 && (
            <div className="notes-list-state">
              <Icon name="square-pen" size={17} />
              <strong>No notes yet</strong>
              <span>Create one or jump to today.</span>
            </div>
          )}
          {notes.map((note) => (
            <button
              className={`note-row ${
                active?.path === note.path ? "active" : ""
              }`}
              type="button"
              key={note.path}
              onClick={() => void selectNote(note.path)}
            >
              <strong>{note.title}</strong>
              <span>{note.preview}</span>
              <small>
                {note.tags.slice(0, 2).map((tag) => (
                  <i key={tag}>#{tag}</i>
                ))}
              </small>
            </button>
          ))}
        </div>
      </aside>

      <section className="note-editor-pane">
        {!active ? (
          <div className="note-empty">
            <span className="empty-icon">
              <Icon name="square-pen" size={18} />
            </span>
            <h2>Your notes stay readable everywhere.</h2>
            <p>
              Each note is a plain Markdown file in your local Tooldeck vault.
            </p>
            <button
              className="primary-button"
              type="button"
              onClick={() => void makeNote()}
            >
              <Icon name="plus" size={14} />
              Create first note
            </button>
          </div>
        ) : (
          <>
            <header className="note-toolbar">
              <div className="save-status" aria-live="polite">
                <span className={`save-dot ${saveState}`} />
                {saveState === "saving"
                  ? "Saving"
                  : saveState === "error"
                    ? "Save failed"
                    : "Saved locally"}
              </div>
              <div className="editor-modes" aria-label="Editor view">
                {(["edit", "split", "preview"] as const).map((item) => (
                  <button
                    type="button"
                    className={mode === item ? "active" : ""}
                    key={item}
                    onClick={() => setMode(item)}
                  >
                    {item}
                  </button>
                ))}
              </div>
              <select
                value={active.projectId ?? ""}
                onChange={(event) => {
                  const projectId = event.target.value
                    ? Number(event.target.value)
                    : null;
                  void linkNoteProject(active.path, projectId).then(() =>
                    setActive((current) =>
                      current ? { ...current, projectId } : current,
                    ),
                  );
                }}
                aria-label="Linked project"
              >
                <option value="">No project</option>
                {projects.map((project) => (
                  <option value={project.id} key={project.id}>
                    {project.name}
                  </option>
                ))}
              </select>
            </header>
            <div className={`editor-surface mode-${mode}`}>
              {mode !== "preview" && (
                <textarea
                  className="markdown-editor"
                  value={draft}
                  onChange={(event) => {
                    setDraft(event.target.value);
                    setSaveState("saving");
                  }}
                  spellCheck
                  aria-label="Markdown editor"
                />
              )}
              {mode !== "edit" && <MarkdownPreview content={draft} />}
            </div>
            <footer className="note-statusbar">
              <span>{active.path}</span>
              <span>{wordCount} words</span>
              <span>{active.tags.length} tags</span>
              <span>
                {backlinks.length} backlink{backlinks.length === 1 ? "" : "s"}
              </span>
            </footer>
          </>
        )}
        {error && (
          <div className="notes-error" role="alert">
            <Icon name="circle-alert" size={14} />
            <span>{error}</span>
            <button type="button" onClick={() => setError(null)}>
              Dismiss
            </button>
          </div>
        )}
      </section>
    </main>
  );
}

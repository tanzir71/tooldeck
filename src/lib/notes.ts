import { invoke, isTauri } from "@tauri-apps/api/core";

export interface NoteSummary {
  path: string;
  title: string;
  preview: string;
  tags: string[];
  projectId: number | null;
  updatedAt: number;
}

export interface NoteDocument {
  path: string;
  title: string;
  content: string;
  tags: string[];
  projectId: number | null;
  updatedAt: number;
}

const previewNotes: NoteDocument[] = [
  {
    path: "welcome-to-tooldeck.md",
    title: "Welcome to Tooldeck",
    content:
      "# Welcome to Tooldeck\n\nYour notes are plain Markdown files that stay on this Mac.\n\n## Try the connected workflow\n\n- Add a #project tag\n- Link another note with [[Daily log]]\n- Start a timer from this context\n",
    tags: ["project"],
    projectId: 1,
    updatedAt: Date.now(),
  },
];

function summarize(note: NoteDocument): NoteSummary {
  const preview =
    note.content
      .split("\n")
      .find((line) => line.trim() && !line.startsWith("#"))
      ?.trim() ?? "Empty note";
  return {
    path: note.path,
    title: note.title,
    preview,
    tags: note.tags,
    projectId: note.projectId,
    updatedAt: note.updatedAt,
  };
}

export async function listNotes(): Promise<NoteSummary[]> {
  if (isTauri()) return invoke<NoteSummary[]>("notes_list");
  return previewNotes.map(summarize);
}

export async function getNote(path: string): Promise<NoteDocument> {
  if (isTauri()) return invoke<NoteDocument>("note_get", { path });
  const note = previewNotes.find((item) => item.path === path);
  if (!note) throw new Error("Note not found.");
  return structuredClone(note);
}

export async function createNote(
  title: string,
  template: string,
): Promise<NoteDocument> {
  if (isTauri()) {
    return invoke<NoteDocument>("note_create", { title, template });
  }
  const cleanTitle = title.trim() || "Untitled note";
  const templateBody =
    template === "meeting"
      ? "#meeting\n\n## Attendees\n\n- \n\n## Notes\n\n\n## Actions\n\n- [ ] "
      : template === "project"
        ? "#project\n\n## Context\n\n\n## Goals\n\n- \n\n## Next steps\n\n- [ ] "
        : "";
  const note: NoteDocument = {
    path: `${cleanTitle.toLowerCase().replace(/[^a-z0-9]+/g, "-") || "note"}.md`,
    title: cleanTitle,
    content: `# ${cleanTitle}\n\n${templateBody}`,
    tags: template === "blank" ? [] : [template],
    projectId: null,
    updatedAt: Date.now(),
  };
  previewNotes.unshift(note);
  return structuredClone(note);
}

export async function saveNote(note: NoteDocument): Promise<NoteDocument> {
  if (isTauri()) {
    return invoke<NoteDocument>("note_save", {
      path: note.path,
      content: note.content,
      expectedUpdatedAt: note.updatedAt,
    });
  }
  await new Promise((resolve) => window.setTimeout(resolve, 140));
  const title =
    note.content
      .split("\n")
      .find((line) => line.startsWith("# "))
      ?.slice(2)
      .trim() || note.title;
  const tags = Array.from(
    new Set(
      Array.from(note.content.matchAll(/(?:^|\s)#([\w-]+)/g)).map(
        (match) => match[1],
      ),
    ),
  );
  const updated = { ...note, title, tags, updatedAt: Date.now() };
  const index = previewNotes.findIndex((item) => item.path === note.path);
  if (index >= 0) previewNotes[index] = updated;
  return structuredClone(updated);
}

export async function getDailyNote(): Promise<NoteDocument> {
  if (isTauri()) return invoke<NoteDocument>("note_daily");
  const date = new Intl.DateTimeFormat("en-CA").format(new Date());
  const existing = previewNotes.find((note) => note.path === `${date}.md`);
  return existing ? structuredClone(existing) : createNote(date, "standup");
}

export async function searchNotes(query: string): Promise<NoteSummary[]> {
  if (isTauri()) return invoke<NoteSummary[]>("notes_search", { query });
  const normalized = query.toLowerCase();
  return previewNotes
    .filter(
      (note) =>
        note.title.toLowerCase().includes(normalized) ||
        note.content.toLowerCase().includes(normalized),
    )
    .map(summarize);
}

export async function getBacklinks(path: string): Promise<NoteSummary[]> {
  if (isTauri()) return invoke<NoteSummary[]>("note_backlinks", { path });
  const target = previewNotes.find((note) => note.path === path);
  if (!target) return [];
  return previewNotes
    .filter(
      (note) =>
        note.path !== path && note.content.includes(`[[${target.title}]]`),
    )
    .map(summarize);
}

export async function linkNoteProject(
  path: string,
  projectId: number | null,
): Promise<void> {
  if (isTauri()) {
    await invoke("note_link_project", { path, projectId });
    return;
  }
  const note = previewNotes.find((item) => item.path === path);
  if (note) note.projectId = projectId;
}

export async function getFloatingNote(): Promise<NoteDocument> {
  if (isTauri()) return invoke<NoteDocument>("floating_note_get");
  const existing = previewNotes.find(
    (note) => note.path === "Quick capture.md",
  );
  return existing ?? createNote("Quick capture", "blank");
}

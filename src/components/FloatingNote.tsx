import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

import { getFloatingNote, saveNote } from "../lib/notes";
import type { NoteDocument } from "../lib/notes";
import { BrandMark } from "./BrandMark";
import { Icon } from "./Icon";

export function FloatingNote() {
  const [note, setNote] = useState<NoteDocument | null>(null);
  const [draft, setDraft] = useState("");
  const [pinned, setPinned] = useState(true);
  const [opacity, setOpacity] = useState(96);
  const [saved, setSaved] = useState(true);

  useEffect(() => {
    let cancelled = false;
    void getFloatingNote().then((document) => {
      if (!cancelled) {
        setNote(document);
        setDraft(document.content);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!note || draft === note.content) return;
    const timeout = window.setTimeout(() => {
      void saveNote({ ...note, content: draft }).then((updated) => {
        setNote(updated);
        setSaved(true);
      });
    }, 420);
    return () => window.clearTimeout(timeout);
  }, [draft, note]);

  async function togglePin() {
    const nextPinned = !pinned;
    setPinned(nextPinned);
    if (isTauri()) await getCurrentWindow().setAlwaysOnTop(nextPinned);
  }

  async function close() {
    if (isTauri()) await getCurrentWindow().hide();
  }

  return (
    <main
      className="floating-note"
      style={{ "--note-opacity": opacity / 100 } as React.CSSProperties}
    >
      <header data-tauri-drag-region>
        <BrandMark />
        <span>{saved ? "Saved locally" : "Saving…"}</span>
        <button
          type="button"
          className={pinned ? "active" : ""}
          onClick={() => void togglePin()}
          aria-label={pinned ? "Unpin floating note" : "Pin floating note"}
          title={pinned ? "Unpin" : "Pin"}
        >
          <Icon name="zap" size={13} />
        </button>
        <button
          type="button"
          onClick={() => void close()}
          aria-label="Close floating note"
        >
          <Icon name="x" size={14} />
        </button>
      </header>
      <textarea
        value={draft}
        onChange={(event) => {
          setDraft(event.target.value);
          setSaved(false);
        }}
        placeholder="Capture something before it slips away…"
        aria-label="Quick capture note"
        autoFocus
      />
      <footer>
        <span>Quick capture.md</span>
        <label>
          <Icon name="sun" size={11} />
          <input
            type="range"
            min="72"
            max="100"
            value={opacity}
            onChange={(event) => setOpacity(Number(event.target.value))}
            aria-label="Floating note opacity"
          />
        </label>
        <span>⌥N</span>
      </footer>
    </main>
  );
}

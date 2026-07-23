# Changelog

## 0.4.0 — M5 offline OCR and document export

- Added direct Rust FFI to the macOS Vision framework for fast offline image
  OCR without Swift, an account, or a bundled network service.
- Added image-to-Markdown OCR with source attribution.
- Added scanned-PDF OCR that preserves the rendered page and adds an invisible
  searchable text layer.
- Added a template-driven export engine with Invoice, Timesheet / Activity
  Report, Project Document, Standup Log, and Research Writeup outputs.
- Added a shared declarative export workspace with multiline content, optional
  tracked hours, rate, and tax inputs.
- Added native fixtures for OCR recognition, searchable PDF extraction, all
  five template PDFs, invoice totals, and multilingual Unicode export.

## 0.3.0 — M4 local PDF and Output Center

- Added a persistent, cancellable local job runner with progress and
  human-readable completion/failure states.
- Added the Output Center with output history, live progress, cancellation,
  dark mode, empty/error states, and a Safety tab.
- Added scoped undo: generated exports are moved into a recoverable internal
  trash and user-selected originals are never mutated.
- Added PDF merge, split, reorder, page removal, rotation, compression,
  PDF-to-images, images-to-PDF, text extraction, watermarking, page numbering,
  and flattened rectangular redaction.
- Added real local PDF fixtures covering merge, split, source preservation, and
  removal of redacted text from the output text layer.
- Added migration 3 for undo status, workflow definitions, and app-activity
  sampling groundwork.

## 0.2.0 — M1–M3 connected context

- Added a plain-Markdown notes vault with atomic writes and external-edit
  reconciliation.
- Added SQLite FTS5 search, tag indexing, templates, daily notes, wiki-links,
  backlinks, and note-to-project links.
- Added a split Markdown editor/preview with debounced autosave and visible
  local-save state.
- Added durable timer and manual entries, local projects, UTC timestamp math,
  rate snapshots, daily totals, and recovery of a running timer after restart.
- Added entry edit/split/merge service operations and native macOS idle-time
  detection with keep/discard/reassign reconciliation.
- Connected the top bar and menu-bar panel to live timer state.
- Added the frameless, pinnable, always-on-top quick-capture note with ⌥N.
- Expanded native coverage to 17 tests, including external edits, FTS,
  backlinks, timer math, idle gaps, entry surgery, and crash recovery.

## 0.1.0 — M0 foundation

- Added the Tauri v2 / React / TypeScript / Tailwind application skeleton.
- Added a Rust-owned tool registry and typed catalog/run IPC contracts.
- Added registry-driven Home, category rail, command palette, and declarative
  tool workspace.
- Added four cross-category M0 tools: New note, Start timer, Merge PDFs, and
  Resize image.
- Added the native tray surface, ⌥Space global palette shortcut, and compact
  menu-bar window.
- Added SQLite bootstrap at Application Support/Tooldeck with an idempotent
  initial migration.
- Added light/dark design tokens, keyboard navigation, reduced-motion support,
  visible focus states, empty/error/success states, and local-only privacy
  affordances.
- Added the reproducible vector app icon source and generated macOS icon assets.
- Added the single-command verification gate with Rust, TypeScript, lint,
  formatting, unit, shell smoke, and production-build checks.

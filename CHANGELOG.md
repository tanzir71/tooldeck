# Changelog

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

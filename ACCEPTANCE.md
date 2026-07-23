# Tooldeck acceptance

The implementation follows the milestone loop in `TOOLDECK_CODEX_HANDOFF.md`.
A milestone advances only after `make verify` is green and its rubric is at
least 4.

## M0 — Skeleton + registry + palette

- [x] Tauri v2 + React + Tailwind launches on macOS 14+ / Apple Silicon
- [x] Tray icon and compact menu-bar surface are available
- [x] Rust registry exposes at least three tools through typed IPC
- [x] ⌘K opens the palette, fuzzy search works, and Enter opens a tool
- [x] Rail, Home, and single-tool workspace are generated from the catalog
- [x] A declarative stub tool runs through ⌘↵ and shows a result state
- [x] Design tokens, light/dark themes, focus states, and reduced motion work
- [x] SQLite opens below Application Support and migration 1 applies
- [x] `make verify` exits successfully
- [x] Rubric is at least 4

Evidence:

- Native `tauri dev` launch completed without runtime errors.
- Database migration 1 was read back from
  `~/Library/Application Support/Tooldeck/tooldeck.sqlite3`.
- Browser smoke covered Home → ⌘K → fuzzy query → Enter → workspace → ⌘↵.
- Screenshots: `artifacts/m0-home.png`, `artifacts/m0-palette-light.png`,
  `artifacts/m0-palette-dark.png`, and `artifacts/m0-menubar-dark.png`.
- Headless gate: 7 Rust tests, 4 frontend tests (3 unit + 1 shell smoke),
  Clippy with warnings denied, ESLint, TypeScript, formatting, and production
  build.

## Later milestones

- [x] M1 — Notes core
  - [x] Plain Markdown vault with atomic autosave and external-edit reindex
  - [x] Split editor/preview, FTS5 search, tags, daily log, and templates
  - [x] Empty/loading/error/saved states and keyboard-accessible controls
- [x] M2 — Time tracking core
  - [x] One durable running timer, UTC epochs, rate snapshots, and reports
  - [x] Projects, manual entries, edit/split/merge service operations
  - [x] Native idle query with keep/discard/reassign reconciliation
  - [x] Crash recovery verified by closing and reopening the SQLite database
- [x] M3 — Links and floating note
  - [x] Wiki-link index, backlinks, and note-to-project links
  - [x] Frameless always-on-top quick note with pin, opacity, and ⌥N
- [ ] M4 — PDF tools, jobs, and safety
- [ ] M5 — OCR and export engine
- [ ] M6 — Media micro-tools
- [ ] M7 — Connected workflows
- [ ] M8 — Polish pass
- [ ] M9 — Signed and notarized package

## Self-critique rubric

| Milestone | Functionality | Edge cases | States | Keyboard + a11y | Visual polish | No data loss | Tests | Performance | Pass |
| --------- | ------------: | ---------: | -----: | --------------: | ------------: | -----------: | ----: | ----------: | ---- |
| M0        |             4 |          4 |      4 |               4 |             4 |            4 |     4 |           5 | Pass |
| M1        |             4 |          4 |      4 |               4 |             4 |            4 |     5 |           4 | Pass |
| M2        |             4 |          4 |      4 |               4 |             4 |            5 |     5 |           4 | Pass |
| M3        |             4 |          4 |      4 |               4 |             4 |            4 |     4 |           4 | Pass |

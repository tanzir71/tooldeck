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

- [ ] M1 — Notes core
- [ ] M2 — Time tracking core
- [ ] M3 — Links and floating note
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

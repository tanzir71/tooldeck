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
- [x] M4 — PDF tools, jobs, and safety
  - [x] Merge, split, reorder, remove, rotate, compress, PDF↔images, and
        selectable-text extraction produce new exports
  - [x] Flattened watermark, page numbering, and rectangular redaction run
        locally; the fixture test confirms redacted text is absent from the
        resulting text layer
  - [x] Persistent jobs expose progress, cancellation, failure, and completed
        states without blocking the UI
  - [x] Output Center has output and Safety tabs, including scoped undo into a
        recoverable internal trash
  - [x] Originals are never passed as an output target
- [x] M5 — OCR and export engine
  - [x] Offline image OCR uses direct Rust→Apple Vision FFI (the handoff’s
        no-Swift fallback) and writes Markdown with source attribution
  - [x] Scanned PDFs are rendered page by page, recognized locally, and rebuilt
        with the original scan plus an invisible searchable text layer
  - [x] Invoice, timesheet/activity report, project document, standup log, and
        research writeup templates render valid PDFs with supplied data
  - [x] Unicode document export produces a valid non-empty PDF
  - [x] OCR and export work through the shared cancellable job, Output Center,
        and Safety/undo contracts
- [x] M6 — Media micro-tools
  - [x] Video trim, crop, MP4/MOV/WebM conversion, GIF export, and PNG frame
        extraction run through FFmpeg/FFprobe jobs
  - [x] GIF output is capped at 15 seconds, 15 fps, and a 960px longest edge
  - [x] Image crop, max-edge resize, PNG/JPEG/HEIC conversion, and JPEG
        compression preserve the selected original
  - [x] Interactive region capture and bitmap markup cover arrow, box, text,
        pixelation, and redaction
  - [x] Generated captures can be appended to a selected Markdown note and
        reindexed
- [x] M7 — Connected workflows
  - [x] Seven named pipelines connect billable time, project notes, OCR
        excerpts, project context, daily logs, capture, media, and exports
  - [x] Workflow definitions are installed idempotently and routed through a
        typed planner that returns job, next-tool, done, or degraded outcomes
  - [x] Capture handoff carries the selected note into the capture tool; media
        output can attach to a note after its background job completes
  - [x] Low-frequency foreground-app sampling attaches to a running time entry
        and stores window titles only with Screen Recording permission
  - [x] App-linked tracking has an explicit user-triggered permission control
        and degrades to app-name-only detail
  - [x] Every workflow has a tested missing-input recovery path
- [ ] M8 — Polish pass
- [ ] M9 — Signed and notarized package

## Self-critique rubric

| Milestone | Functionality | Edge cases | States | Keyboard + a11y | Visual polish | No data loss | Tests | Performance | Pass |
| --------- | ------------: | ---------: | -----: | --------------: | ------------: | -----------: | ----: | ----------: | ---- |
| M0        |             4 |          4 |      4 |               4 |             4 |            4 |     4 |           5 | Pass |
| M1        |             4 |          4 |      4 |               4 |             4 |            4 |     5 |           4 | Pass |
| M2        |             4 |          4 |      4 |               4 |             4 |            5 |     5 |           4 | Pass |
| M3        |             4 |          4 |      4 |               4 |             4 |            4 |     4 |           4 | Pass |
| M4        |             4 |          4 |      4 |               4 |             4 |            5 |     4 |           4 | Pass |
| M5        |             4 |          4 |      4 |               4 |             4 |            5 |     5 |           4 | Pass |
| M6        |             4 |          4 |      4 |               4 |             4 |            5 |     5 |           4 | Pass |
| M7        |             4 |          5 |      4 |               4 |             4 |            5 |     5 |           4 | Pass |

M4 evidence:

- Native Tauri launch applied migration 3 to the existing local database.
- The fixture gate creates real PDFs locally, merges and splits them, and
  verifies the source bytes remain unchanged.
- The redaction gate rasterizes and flattens a source containing
  `SECRET-CODE-7419`, then confirms `pdftotext` cannot recover that string.
- Safety tests reject paths outside Tooldeck exports and verify undo moves only
  a generated output.
- Screenshots: `artifacts/m4-output-center-light.png` and
  `artifacts/m4-pdf-redact-dark.png`.
- `make verify` passes with 25 Rust tests, 4 frontend tests, Clippy with
  warnings denied, ESLint, TypeScript, formatting, and the production web
  build.

M5 evidence:

- The unrestricted macOS Vision fixture test recognizes
  `SECRET-CODE-7419`; restricted CI sandboxes detect the unavailable native
  framework service and keep the rest of the deterministic gate runnable.
- The searchable-PDF fixture retains its JPEG page and exposes the expected OCR
  string through `pdftotext`.
- Every shipped template renders to PDF and is re-read to confirm the title,
  client/project, and body; invoice math is separately asserted.
- The Unicode fixture includes `Résumé · 東京 · প্রকল্প` and produces a valid
  PDF.
- Screenshots: `artifacts/m5-export-light.png`,
  `artifacts/m5-export-output-light.png`, and `artifacts/m5-ocr-dark.png`.
- `make verify` passes with 32 Rust tests and the full frontend/lint/type/build
  gate.

M6 evidence:

- The real FFmpeg fixture verifies a one-second trim within 0.08 seconds,
  WebM conversion, a 160px/8fps/two-second capped GIF, a 160×120 crop, and a
  PNG frame grab.
- Real ImageIO fixtures verify crop dimensions, JPEG conversion/compression,
  BMP-backed screenshot redaction, PNG re-rendering, and unchanged source
  bytes.
- The notes fixture copies a generated capture below `attachments/`, appends
  the Markdown image link, and rebuilds the derived index.
- Screenshots: `artifacts/m6-video-gif-light.png`,
  `artifacts/m6-media-output-light.png`, and
  `artifacts/m6-markup-dark.png`.
- `make verify` passes with 36 Rust tests and the full frontend/lint/type/build
  gate.

M7 evidence:

- Planner fixtures cover all seven recoverable missing-input states, plus
  populated job, note, context-switch, standup, capture-handoff, and media
  outcomes.
- The native app launches with the CoreGraphics permission probe and
  foreground sampler active; the sampler records only while a timer is
  running.
- Browser acceptance verifies seven discoverable workflow cards, successful
  and degraded Context Switcher results, full dark-mode parity, and exact
  preservation of `welcome-to-tooldeck.md` through the capture prefill handoff.
- Screenshots: `artifacts/m7-workflows-light.png`,
  `artifacts/m7-context-dark.png`, and
  `artifacts/m7-capture-handoff-dark.png`.
- `make verify` passes with 39 Rust tests and the full frontend/lint/type/build
  gate.

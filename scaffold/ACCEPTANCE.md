# Tooldeck — ACCEPTANCE (living checklist)

> The build loop (handoff §11) updates this file **every milestone**. A milestone advances only when all its criteria are `[x]` and its rubric row is ≥4. Keep the whole harness green — never regress a passing milestone.

## How to use
- Check items as they pass their automated test or manual verification.
- Record the self-critique rubric score (1–5) per milestone in the table at the bottom.
- Log known gaps as `- [ ] GAP: …` under the milestone; fix P0/P1 before advancing.
- Run `make verify` before checking anything off — it must exit 0.

---

## M0 — Skeleton + registry + palette
- [ ] Tauri v2 + React + Tailwind app launches on macOS 14+ (Apple Silicon)
- [ ] Tray icon present; menu-bar dropdown opens with timer placeholder + quick actions
- [ ] Tool **registry** exists (Rust `Tool` trait + TS `ToolMeta`); ≥3 stub tools registered
- [ ] Command palette (⌥Space / ⌘K) opens, fuzzy-finds stub tools, Enter opens one
- [ ] Main window renders rail → Home → single-tool workspace; a stub tool runs (Run / ⌘↵)
- [ ] Design tokens wired (spacing/type/color); light + dark both render, no flash
- [ ] SQLite connects; first migration applies; DB path under App Support/Tooldeck
- [ ] `make verify` runs and exits 0 (fmt, clippy -D warnings, tsc, eslint, cargo test, e2e smoke)
- [ ] Rubric ≥ 4

## M1 — Notes core
- [ ] Markdown editor + live preview; auto-save to `.md`; folder index built
- [ ] FTS5 search returns hits across the vault
- [ ] Tags, daily log (jump-to-today), templates work
- [ ] External-edit reconciliation test passes (edit file outside app → index updates, no data loss)
- [ ] Rubric ≥ 4

## M2 — Time tracking core
- [ ] Menu-bar timer start/stop/switch; projects/clients/tasks CRUD with rates/billable
- [ ] Manual + timer entries; edit/split/merge; reports (day/week/project/client/billable)
- [ ] Timestamps stored UTC; durations from stored start/stop
- [ ] Crash-recovery test: running timer survives quit/relaunch (resume or close at last activity)
- [ ] Idle detection prompts keep/discard/reassign
- [ ] Rubric ≥ 4

## M3 — Links + floating note
- [ ] Wiki-links/backlinks; note↔project↔entry linking model + tests
- [ ] Always-on-top floating note: stays on top, does not steal focus, opacity + pin work
- [ ] Rubric ≥ 4

## M4 — PDF file tools
- [ ] Merge/split/reorder/rotate/delete; compress; PDF↔images; extract text
- [ ] Watermark / page numbers
- [ ] **Redaction test: redacted text is absent from the re-extracted text layer**
- [ ] Job runner + Output Center + Safety/undo: every destructive op previews + reverses
- [ ] Rubric ≥ 4

## M5 — OCR + export/document engine
- [ ] Tesseract sidecar OCR: fixture image → expected substring
- [ ] Scanned PDF → searchable PDF (text layer added)
- [ ] Each template renders a valid PDF with correct data: Invoice, Timesheet, Project doc, Standup log, Research writeup
- [ ] Unicode/CJK export test passes (bundled Unicode font)
- [ ] Rubric ≥ 4

## M6 — Media micro-tools
- [ ] ffmpeg/ffprobe sidecars run; video trim (±1 frame), crop, →GIF (dims/fps capped), convert, frame-grab
- [ ] Image crop/resize/convert/compress/rotate/strip-metadata
- [ ] Screenshot capture + markup; attach to note
- [ ] No timeline/recording features leaked in (scope §1.3, §8)
- [ ] Rubric ≥ 4

## M7 — Connected workflows
- [ ] Each §7 pipeline runs end-to-end: Billable→Document, Documentation companion, Research companion, Context switcher, Standup log, Capture→annotate→attach, Media micro-task
- [ ] Every workflow degrades gracefully on missing input (clear empty state)
- [ ] App-linked tagging + permission flow (§9.1) works or degrades to app-name-only
- [ ] Rubric ≥ 4

## M8 — Polish pass
- [ ] Full §10 Definition of Done sweep across all surfaces
- [ ] Every state designed (loading/empty/error/success/permission-denied)
- [ ] a11y (§16.7), dark mode, onboarding, reduced-motion, performance (no jank)
- [ ] UI acceptance gate (§16.8) passes for every tool/screen
- [ ] Rubric ≥ 4 on **every** dimension for **every** surface

## M9 — Package & notarize
- [ ] App + every sidecar signed (hardened runtime); notarized; stapled; DMG built
- [ ] `codesign --verify --deep --strict` passes; `spctl -a -t exec -vv` accepts
- [ ] Clean-machine first-run opens without Gatekeeper warnings; sidecars run post-install
- [ ] FFmpeg license documented in-app + repo (flagged to human)
- [ ] Rubric ≥ 4

---

## Self-critique rubric log (§11.3) — score 1–5, must be ≥4 to pass
| Milestone | Functionality | Edge cases | States | Keyboard+a11y | Visual polish | No data-loss | Test coverage | Performance | Pass? |
|---|---|---|---|---|---|---|---|---|---|
| M0 |  |  |  |  |  |  |  |  |  |
| M1 |  |  |  |  |  |  |  |  |  |
| M2 |  |  |  |  |  |  |  |  |  |
| M3 |  |  |  |  |  |  |  |  |  |
| M4 |  |  |  |  |  |  |  |  |  |
| M5 |  |  |  |  |  |  |  |  |  |
| M6 |  |  |  |  |  |  |  |  |  |
| M7 |  |  |  |  |  |  |  |  |  |
| M8 |  |  |  |  |  |  |  |  |  |
| M9 |  |  |  |  |  |  |  |  |  |

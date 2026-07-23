# Tooldeck — Codex Build Handoff

> **What this is:** A complete, self-contained brief for an autonomous coding agent (Codex) to build a polished, local-first macOS productivity app **in an iterative build loop**. Everything Codex needs to make architecture, scope, and acceptance decisions is in this one file.
>
> **Hard constraint:** Codex **cannot write Swift**. This app is therefore built on **Tauri v2 (Rust backend + web frontend)** — no Xcode, no SwiftUI, no AppKit-in-Swift. All native macOS behavior is reached through Rust crates or FFI.
>
> **Codename:** `Tooldeck` (rename freely).
> **Target platform:** macOS 14 (Sonoma)+ on Apple Silicon. Intel is out of scope.
> **Distribution:** Signed + **notarized DMG**, direct download. Not sandboxed, not Mac App Store.

---

## 0. TL;DR for the agent

Build a lightweight, **100% offline** macOS menu-bar + window app that folds together three core toolkits — **contextual notes**, **frictionless time tracking**, and **local PDF/OCR utilities** — plus a small set of **on-device media micro-tools** (crop/trim video, image resize/convert). Explicitly **not** a screen recorder or a video editor.

The differentiator is not the tools individually — it's the **connected workflows** that chain notes → tracked time → exported documents, where the exported document may be an **invoice, a project doc, a client report, a research writeup, or a standup log**. Invoicing is *one* output, not the point.

Around those three pillars sits a **wide shelf of small, single-purpose utilities** (image, file, text, data/dev, capture, QR, media micro-tools) — Foldout-style breadth — kept sane by a **command-palette-first UI** (§16) where every tool is a keystroke away and nothing clutters the screen until you ask for it. The full catalog is in §15, tiered so v1 stays buildable.

Stack: **Tauri v2 + Rust core + SQLite + a TypeScript/React front end**. Data lives in local files and a local SQLite DB. No accounts, no cloud, no network calls except an explicit, opt-in "check for updates."

Build it **in a loop**: implement one milestone, run the self-verification harness for that milestone, fix until green, take a screenshot, self-critique against the polish checklist, then advance. Section 11 is the loop protocol — treat it as the operating manual.

---

## 1. Product vision & positioning

### 1.1 The thing we're replacing
The reference product is **Foldout** (getfoldout.com) — a native (Swift) macOS app that bundles 113+ single-purpose utilities: file tools, PDF tools, image/video converters, screen capture, cinematic screen recording, OCR, dev-data utilities, menu-bar helpers, and a natural-language command bar that routes you to the right tool. It processes everything locally, is signed/notarized, and sells at $5/mo.

**Tooldeck is a deliberately narrower, opinionated subset** of that idea, rebuilt on a non-Swift stack, and centered on *knowledge/time/document* work rather than being a kitchen-sink converter.

### 1.2 What we keep from that model
- **Local-first, offline, privacy-by-default.** Files never leave the Mac.
- **Menu-bar-resident** with a fast launcher / command palette.
- **Focused "workspace per tool"** UX — each tool opens with sensible defaults pre-filled.
- **A "review before it touches your files" safety layer** — no destructive op runs without a preview + undo.
- **Signed + notarized** so Gatekeeper opens it cleanly.

### 1.3 Breadth model — narrow *core*, broad *utility shelf*
Tooldeck has **three deep pillars** (notes, time tracking, PDF/OCR + document export) and a **wide, shallow shelf of single-purpose utilities** around them (image, file, text, data/dev, media micro-tools, capture, QR, etc.) — the Foldout-style "100 small tools, folded in." The catalog is **exhaustive but tiered** (§15) so breadth never blocks a buildable v1: the shelf grows tool-by-tool through a plugin/registry pattern (§15.1), each tool cheap to add and independently testable.

The thing that keeps this from becoming bloated is the **UI** (§16): a command-palette-first, Raycast-style model where every tool is one keystroke + a few characters away, and heavy tools stay out of sight until summoned. Many tools, minimal surface.

**Hard exclusions (permanent non-goals), because a dedicated app does them better:**
- ❌ **Screen recording / cinematic recorder** — out. (Screen *capture* + markup stays; *recording* does not.)
- ❌ **Full timeline video editor** (multi-clip timelines, animated titles, intros/outros) — out, overkill.
- ❌ **Audio production studio** (multi-track, effects) — out. (Trivial audio ops like extract-from-video / trim / normalize are Tier 2, optional.)
- ❌ Anything requiring an account, a server, or telemetry.

We *do* keep **light, high-frequency media micro-tasks** that otherwise force people into a heavyweight editor: **crop/trim a video, video→GIF, crop/resize/convert/compress an image, quick screenshot markup.** "I just need to shave 4 seconds off this clip," done inline.

### 1.4 Who it's for
A knowledge worker / freelancer / designer / consultant who keeps notes, tracks time against projects or clients, and regularly produces **documents** — invoices, project docs, client-facing reports, research writeups, meeting/standup logs — and is tired of gluing together Toggl + a notes app + a PDF site + an image resizer + a video trimmer.

---

## 2. The stack decision (locked)

| Decision | Choice | Why |
|---|---|---|
| App framework | **Tauri v2** | ~10 MB app, 30–50 MB idle RAM, system WebKit WebView, capability-based permission model (everything off by default). Fits privacy-first, lightweight goals. Codex is fluent in Rust + TS. |
| Backend language | **Rust** (Tauri core + commands) | Native OS integration, sidecar orchestration, file/DB work. No Swift needed. |
| Frontend | **TypeScript + React + Vite** | Codex-friendly, huge ecosystem, easy to make polished. (Svelte is an acceptable alternative if the agent prefers; React is the default assumption throughout this doc.) |
| Styling | **Tailwind CSS + Radix UI primitives** (or shadcn/ui) | Accessible primitives, fast to reach a polished, native-feeling look. Support light/dark. |
| Local DB | **SQLite** via `rusqlite` (bundled, `libsqlite3-sys` with the bundled feature) | Zero-config, fast, local, private. |
| Migrations | `refinery` or hand-rolled versioned SQL in Rust | Deterministic schema evolution. |
| State (frontend) | **Zustand** (or Redux Toolkit) | Simple, predictable. |
| Markdown editor | **CodeMirror 6** (source) + a render pane, or **Milkdown/TipTap** for WYSIWYG-ish | Instant formatting, code highlighting. |
| PDF read/render | **`pdfium-render`** (Rust binding to PDFium) for rendering/rasterizing; **`lopdf`** for structural ops (merge/split/rotate/delete pages, metadata) | All local, no service. |
| PDF create/write | Render via a headless HTML→PDF path (see §6.3) or `printpdf` for programmatic invoices/reports | Needed for document/invoice export. |
| OCR | **Tesseract** via `rusty-tesseract` or a bundled `tesseract` sidecar (default). *Optional advanced path:* call Apple Vision via `objc2` FFI from Rust — no Swift required — behind a feature flag. | On-device text recognition. |
| Media (video crop/trim, video→gif, image ops) | **FFmpeg + FFprobe as Tauri sidecars** (`externalBin`) | One dependency covers all media micro-tools. See §6.4 + §9.4 for the notarization caveat. |
| Image ops | `image` crate (resize/convert/crop/compress) for simple cases; FFmpeg for the rest | Local, fast. |
| Active-window detection | **`x-win`** (preferred) or **`active-win-pos-rs`** | For app-linked timers. **Caveat in §9.1.** |
| Idle detection | `user-idle` crate, or FFI to `CGEventSourceSecondsSinceLastEventType` | For idle-time prompts. |
| Global hotkeys | `tauri-plugin-global-shortcut` | Launcher, quick-track, floating note. |
| Tray / menu bar | Tauri `TrayIcon` API | Menu-bar timer + launcher. |
| Always-on-top overlay | Tauri window with `always_on_top`, `decorations:false`, transparent, non-activating panel where possible | Floating note. |
| Packaging | `tauri build` → DMG; sign + notarize + staple (§12) | Notarized direct download. |

> **Note on the "let the agent decide" alternative:** it was considered and rejected. This doc commits to Tauri v2 so Codex spends its loop iterations building, not re-deciding architecture.

---

## 3. High-level architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend (WebView: React + TS + Tailwind)                   │
│  • Command palette / launcher                                │
│  • Notes UI  • Timer UI  • PDF UI  • Media micro-tools       │
│  • Floating overlay note (separate Tauri window)             │
│  • Menu-bar dropdown (tray-anchored window)                  │
└───────────────▲───────────────────────────┬─────────────────┘
                │ Tauri IPC (typed commands)  │ events (timer tick, idle, job progress)
┌───────────────┴───────────────────────────▼─────────────────┐
│  Rust core (Tauri commands + services)                       │
│  • notes_service     • timer_service    • pdf_service        │
│  • ocr_service       • media_service    • activewin_service  │
│  • job_runner (async, cancellable, progress events)          │
│  • safety/undo ledger   • export_service (doc/invoice/report)│
└───────┬──────────────────┬─────────────────┬────────────────┘
        │                  │                 │
   ┌────▼────┐        ┌────▼─────┐      ┌─────▼──────┐
   │ SQLite  │        │ Sidecars │      │ FS (files) │
   │ (rusqlite)       │ ffmpeg   │      │ notes/,    │
   │ notes, entries,  │ ffprobe  │      │ exports/,  │
   │ projects, jobs   │ tesseract│      │ attachments│
   └─────────┘        └──────────┘      └────────────┘
```

**Principles**
- **Rust owns all side effects.** The frontend never touches the filesystem directly; it calls typed Tauri commands.
- **Long-running work is a `Job`.** OCR, PDF compression, media transcode, batch renames → all run in a cancellable async job with progress events, surfaced in an **Output Center** (like Foldout's Jobs view).
- **Every destructive op writes to an undo ledger first** (Safety Center). Originals are never mutated in place — write to a new file, then optionally replace after user confirmation.
- **Modules are self-contained.** Each tool = a Rust service + a frontend route. Adding a tool later shouldn't require touching others.

### 3.1 Repo layout
```
tooldeck/
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ commands/            # one module per tool surface
│  │  │  ├─ notes.rs  timer.rs  pdf.rs  ocr.rs  media.rs
│  │  │  ├─ activewin.rs  export.rs  jobs.rs  safety.rs
│  │  ├─ services/            # business logic, no Tauri types
│  │  ├─ db/                  # rusqlite pool, migrations, models
│  │  ├─ jobs/                # async job runner + registry
│  │  └─ platform/            # macOS FFI (idle, panel window, permissions)
│  ├─ binaries/               # ffmpeg, ffprobe, tesseract sidecars
│  ├─ migrations/             # NNN_name.sql
│  ├─ capabilities/           # Tauri v2 permission scopes
│  ├─ tauri.conf.json
│  └─ Cargo.toml
├─ src/                       # React frontend
│  ├─ routes/                 # notes/ timer/ pdf/ media/ launcher/
│  ├─ components/  lib/  stores/  styles/
│  └─ main.tsx
├─ tests/
│  ├─ rust/                   # cargo test + integration
│  ├─ e2e/                    # WebDriver (tauri-driver) smoke flows
│  └─ fixtures/               # sample.pdf, scan.png, clip.mp4, etc.
├─ scripts/                   # sign.sh, notarize.sh, seed-fixtures.sh
├─ ACCEPTANCE.md              # living checklist the loop updates
└─ README.md
```

---

## 4. Core module: Contextual Notes

**Goal:** a fast, local, markdown-native notes surface that doubles as the *substrate* documents are exported from.

### 4.1 Features
- **Markdown sandbox** — CodeMirror source editor + live preview (toggle or split). Instant formatting, code-block syntax highlighting, checkboxes, tables. **Auto-save** on debounce; never a "save" button as the only path.
- **Floating overlay note** — a separate, pinnable, semi-transparent always-on-top window (`decorations:false`, `always_on_top:true`, adjustable opacity). Quick-capture from a global hotkey. Stays visible over other apps for copy/reference.
- **Daily log** — auto-generated per-day note file (`YYYY-MM-DD.md`) for running journal/standup capture. "Jump to today" hotkey.
- **Backlinks & `[[wiki-links]]`** — link notes to each other; a backlink pane shows references. This is what makes notes a knowledge base, not just scratch.
- **Note ↔ entity links** — a note can be linked to a **project/client** and to **time entries** and to **attachments** (PDFs, images). This is the join that powers the workflows in §7.
- **Tags** — `#tag` inline + a tag index.
- **Full-text search** across all notes (SQLite FTS5).
- **Templates** — reusable note scaffolds (meeting note, project brief, invoice line-items, report skeleton).

### 4.2 Storage
- Notes are **plain `.md` files on disk** under `~/Library/Application Support/Tooldeck/notes/` (portable, user-owned, greppable), with a SQLite index for search, links, tags, and metadata. **Files are the source of truth; DB is a derived index** that can be rebuilt by re-scanning the folder.
- Attachments live under `attachments/`, referenced by relative path.

### 4.3 Edge cases
- External edits to `.md` files (user edits in another editor) → watch the folder, reconcile, rebuild index; last-writer-wins with a conflict copy if the app had unsaved buffer.
- Huge notes / huge vaults → lazy load, virtualized lists, incremental FTS indexing.
- Broken `[[links]]` → show as unresolved, offer to create.
- Non-UTF8 / binary dropped into notes folder → skip gracefully.

---

## 5. Core module: Frictionless Time Tracking

**Goal:** Toggl-grade ergonomics, fully local, tightly linked to notes and projects.

### 5.1 Features
- **Menu-bar timer** — one-click start/stop from the tray. Shows running duration + current project. Global hotkey to start/stop and to "start timer on the note I'm looking at."
- **Projects & clients** — hierarchy: Client → Project → (optional) Task. Color-coded. Billable flag + hourly rate per project/client (rate is *data*, used by exports; not limited to invoicing).
- **Manual + timer entries** — add/edit past entries, split, merge, adjust start/stop.
- **Idle detection** — when no input for N minutes, on return prompt: *keep / discard / reassign* the idle span. Configurable threshold.
- **App-linked auto-tagging** — sample the **active application + window title** on an interval and attach it to the running entry (so an entry accrues "which app/site was in front"). Powers automatic categorization and the Context Switcher workflow. **Permission caveat: §9.1.**
- **Automatic timeline mode (Tier 1, opt-in)** — inspired by Timing/Rize: instead of (or alongside) manual timers, passively build a background timeline of foreground app/document/site usage, then let the user **review the day and assign blocks to projects at end of day** with rules ("Figma on ClientB-* → ClientB"). Fully local; no AI-cloud. This makes tracking zero-touch for people who forget to hit start. Manual timers remain the default; this is a mode, not a replacement.
- **Pomodoro / focus option** — optional interval timer that still logs real elapsed time.
- **Reports** — by day/week/project/client/billable; totals, rounded/exact, hours + amount (rate × time). Export to the document layer (§6.3, §7).
- **Reminders** — "timer still running after X hours?" and "you opened a linked project note but no timer is running — start one?" (opt-in, gentle, dismissible).

### 5.2 Data model (SQLite, illustrative)
```sql
clients(id, name, default_rate, currency, archived, created_at)
projects(id, client_id, name, color, billable, rate, archived, created_at)
tasks(id, project_id, name, archived)
time_entries(
  id, project_id, task_id, note_id NULL,
  description, started_at, stopped_at NULL,
  source ENUM('timer','manual'), billable, rate_snapshot,
  created_at, updated_at
)
entry_app_samples(id, entry_id, sampled_at, app_name, window_title)
tags(id, name); entry_tags(entry_id, tag_id)
notes(id, path, title, project_id NULL, created_at, updated_at)  -- index of the md files
note_links(src_note_id, dst_note_id, kind)  -- wiki-links/backlinks
attachments(id, note_id NULL, project_id NULL, path, kind, created_at)
jobs(id, kind, status, progress, input_json, output_path NULL, error NULL, created_at)
undo_ledger(id, op_kind, before_json, after_json, reversible, created_at)
settings(key, value)
```

### 5.3 Edge cases
- **Clock changes / sleep / timezone / DST** — store timestamps as UTC epoch; compute durations from stored start/stop, not from a running counter that a sleep would corrupt. On wake, reconcile a timer that ran across sleep (treat sleep span as idle → prompt).
- **App crash with a running timer** — persist the running entry's `started_at` immediately on start; on relaunch, detect the orphan and offer to resume or close it at last-known activity time.
- **Overlapping entries** — decide policy: allow but warn, or auto-stop the previous. Default: only one running timer; starting a new one stops the old.
- **Editing an entry into a negative/zero duration** — validate.
- **Rate changes over time** — snapshot the rate onto the entry at creation (`rate_snapshot`) so historical reports/invoices don't retroactively change.
- **Window title privacy** — window titles can contain sensitive text; provide a per-app "don't record titles" denylist and a global toggle.

---

## 6. Core module: Local PDF & OCR utilities

**Goal:** the handful of PDF jobs people otherwise upload to a sketchy website — done locally, with review.

### 6.1 PDF file-manipulation tools (via `lopdf` / `pdfium-render`)
- **Compress / shrink** — downsample embedded images, re-encode, drop unused objects. Show before/after size; never overwrite original.
- **Merge** — combine multiple PDFs, drag-to-reorder.
- **Split** — one-per-page, by range, or every N pages.
- **Reorder / delete / rotate pages** — visual thumbnail grid.
- **Extract text** — pull selectable text out (for text-layer PDFs).
- **PDF → images** (PNG/JPG per page) and **images → PDF**.
- **Page numbers / Bates stamping**, **text watermark**, **basic redaction** (draw box → flatten so content is truly removed, not just covered).
- **Password add/remove**, **fill & sign (lite)** — place text + a signature image, flatten.

### 6.2 OCR (via Tesseract sidecar; optional Vision-FFI path)
- **Image/scan → text** and **scanned-PDF → searchable-PDF** (OCR text layer added under the image).
- Output as **markdown** so it drops straight into a note (this is the Research Companion glue).
- Language packs selectable; default English bundled.

### 6.3 Document / export engine (the part invoicing is only one case of)
This is the piece that turns notes + tracked time into a **deliverable PDF or file**. Build it as a **template + data** system, not a hardcoded invoice generator:

- **Data sources:** a note (markdown body), a set of time entries (filtered by project/date/billable), project/client metadata, attachments.
- **Templates** (user-editable, shipped defaults):
  - **Invoice** — line items from billable time entries, rate, totals, tax field, client block, invoice #.
  - **Timesheet / activity report** — entries grouped by day/task, hours, optional amounts, for client status or internal reporting.
  - **Project document / spec** — the note's markdown, styled, with a header/footer, TOC, page numbers. Documentation is a first-class output.
  - **Meeting / standup log** — daily-log note → tidy PDF.
  - **Research writeup** — note + embedded OCR excerpts + image attachments.
- **Render path:** markdown/template → HTML (with a print stylesheet) → PDF. Prefer a deterministic HTML→PDF route (headless print via the WebView's print-to-PDF, or a bundled renderer) over hand-drawing PDFs, so templates are easy to style. `printpdf` is the fallback for purely programmatic layouts.
- **Output** lands in `exports/`, opens in the Output Center, never overwrites, and is logged for undo.

### 6.4 Edge cases
- **Encrypted / permission-restricted PDFs** — detect, ask for password, fail gracefully if truly locked.
- **Corrupt / malformed PDFs** — never crash the app; surface a clear job error.
- **Scanned vs. text PDFs** — detect whether a text layer exists before "extract text"; route to OCR if not.
- **Huge PDFs (1000s of pages)** — stream, show progress, allow cancel.
- **Fonts / CJK / RTL in export** — bundle a Unicode font; test non-Latin export.
- **Color profiles / CMYK images** in compression — don't wreck colors.
- **Redaction correctness** — verify redacted text is *gone* from the extracted text layer, not just visually covered. Add an automated test for this.

---

## 7. Connected workflows — the "secret sauce" (generalized)

Individual tools are commodity. The value is chaining them. Build these as **explicit, named, savable pipelines** (a lightweight "Workflows" engine like Foldout's recipes), and expose the common ones as one-click actions. **Invoicing is one pipeline among several — do not special-case it.**

1. **Billable → Document pipeline (invoice *or* report).**
   Highlight billable time for a project/date range → choose a template (Invoice / Timesheet / Activity report) → export PDF. Same data, different deliverable. A freelancer bills; a consultant sends a status report; an internal PM sends a timesheet.

2. **Documentation companion.**
   A project note accumulates content across a week (text, checklists, pasted screenshots, OCR'd excerpts). One action styles it into a **project doc / spec / handoff PDF** with header, TOC, page numbers. This is documentation as a primary use case, spanning any app the user works in.

3. **Research companion.**
   While viewing a PDF in the PDF tool, a side pane opens for notes. Any text pulled via **OCR or extract-text** pastes into the note **with a backlink to the source PDF + page**. Screenshots of a region can be cropped/annotated and attached inline.

4. **Context switcher.**
   When the tracked project changes (start timer on Client B), the app swaps the **floating overlay note** and the open workspace to that project's context — its note, its recent entries, its attachments. Active-window sampling can *suggest* the switch ("you've been in Figma on 'ClientB-app' for 10 min, switch project?").

5. **Meeting/standup log.**
   Global hotkey → append timestamped bullet to today's daily log while a meeting timer runs → end of day, export the log or roll it into a standup update.

6. **Capture → annotate → attach.**
   Quick screenshot (region) → crop/mark up (arrow/box/blur/redact) → attach to the active note or drop into a doc. (Capture yes; *recording* no.)

7. **Media micro-task inline.**
   "This clip needs to be 8s and a GIF for the doc" → trim range → export GIF/MP4 → attach to note/doc. No separate editor, no timeline.

Each workflow must degrade gracefully if a step's input is missing (no billable entries, no OCR text, etc.) with a clear empty state.

---

## 8. Media micro-tools (deliberately minimal)

All via the FFmpeg/FFprobe sidecars + `image` crate. **UI is single-purpose and dead simple** — pick input, set the one or two params that matter, run, get output. No timelines.

- **Video: crop (region), trim (range), video→GIF, quick compress/convert, extract a frame as PNG.**
- **Image: crop, resize (max-dim or preset), convert (PNG/JPG/HEIC/WebP), compress, rotate/flip, strip metadata.**
- **Screenshot capture + markup** (region capture, arrow/text/box/blur/redact, copy or attach). Uses macOS screencapture facilities via Rust/shell, not a recorder.

**Non-negotiable exclusions:** no screen *recording*, no cinematic recorder, no multi-clip timeline, no title-card animation, no audio studio. If Codex is tempted to add these "since ffmpeg can" — don't. They violate scope §1.3.

**Edge cases:** unsupported codecs (surface ffprobe error clearly), gigantic files (progress + cancel), zero-length trim, aspect-ratio-preserving vs. exact crop, GIF size explosion (cap dimensions/fps with a warning), HEIC support presence.

---

## 9. macOS integration specifics & permissions (the hard 20%)

This is where a non-Swift stack has sharp edges. Handle each explicitly.

### 9.1 Active-window / window-title reading ⚠️
Both `x-win` and `active-win-pos-rs` can read the frontmost window's app + title on macOS, **but the window *title* returns empty unless the app has Screen Recording permission granted.** Design for this:
- App works fully without the permission; app-linked *titles* are simply unavailable until granted.
- Provide a first-run explainer + a "Enable app-linked tracking" flow that deep-links to System Settings → Privacy & Security → Screen Recording.
- Gracefully record app *name* (often available with Accessibility) even when title is blocked, and clearly show which level of detail is active.
- Requesting Screen Recording will require re-launch after grant on some macOS versions — handle that UX.

### 9.2 Idle detection
`user-idle` crate or FFI to `CGEventSourceSecondsSinceLastEventType(kCGAnyInputEventType)`. No special permission for global idle seconds. Poll on a low-frequency interval; don't busy-loop.

### 9.3 Global shortcuts, tray, floating panel
- `tauri-plugin-global-shortcut` for launcher / quick-track / quick-note. Handle conflicts with existing system/app shortcuts (let user rebind).
- Tray icon shows timer state; clicking opens a tray-anchored window.
- Floating overlay = a Tauri window: `always_on_top`, `transparent`, `decorations:false`, `skip_taskbar`/accessory activation policy so it doesn't steal focus. On macOS set the app **activation policy to Accessory** (menu-bar app, no Dock icon) if desired — reachable via `tauri` APIs / `objc2` without Swift.

### 9.4 Sidecars, signing & notarization ⚠️
- FFmpeg/FFprobe/Tesseract ship as `externalBin` sidecars (per-target-triple naming, e.g. `ffmpeg-aarch64-apple-darwin`).
- **Known Tauri issue:** notarization can fail when sidecars are present unless **each sidecar binary is itself signed** with hardened runtime and included correctly. Sign every sidecar, enable `hardenedRuntime`, staple. Budget loop iterations for getting a *clean notarization* — add it as an explicit milestone gate (§12), not an afterthought.
- Use static/trusted FFmpeg builds; mind FFmpeg's **LGPL vs GPL** licensing — prefer LGPL builds and document the license in-app + repo. (Legal note, not legal advice — flag for the human.)

### 9.5 File access & privacy
- Not sandboxed → full disk access subject to macOS TCC prompts for Desktop/Documents/Downloads. Trigger prompts lazily, explain why.
- All app data under `~/Library/Application Support/Tooldeck/`. Provide "reveal in Finder" and a clean uninstall note.

### 9.6 Permissions matrix (surface in a Settings → Privacy pane)
| Capability | macOS permission | Degrades to |
|---|---|---|
| App-linked window titles | Screen Recording | app-name-only / off |
| Global hotkeys | none (may need Accessibility for some) | in-app shortcuts only |
| Idle detection | none | off |
| Screenshot capture | Screen Recording | disabled |
| Desktop/Docs file ops | TCC folder prompts | choose files manually |

---

## 10. What "fully polished" means (the bar)

The loop must not mark the app "done" until these hold. Treat as the **Definition of Done**, checked every milestone.

- **Keyboard-first:** command palette (⌘K) reaches every tool; every primary action has a shortcut; full tab/focus order; ESC closes overlays.
- **Every state designed:** loading, empty, error, success, offline-N/A, permission-denied. No dead ends; every error is human-readable + recoverable.
- **Light + dark mode**, follows system, no flashes, respects reduced-motion.
- **Native feel:** correct traffic-light insets, vibrancy/transparency where apt, no web-app scrollbars, momentum scroll, no text selection on chrome, proper window min-sizes.
- **Instant + non-blocking:** no spinner on the main thread; long jobs go to the Output Center with progress + cancel; UI stays at 60fps.
- **Safe:** nothing destructive without preview + undo; originals preserved; Safety Center lists and reverses recent ops.
- **Accessible:** WCAG-ish — contrast, focus rings, ARIA on custom controls, VoiceOver labels on key flows, ≥ adequate hit targets.
- **Data-safe:** crash/quit never loses a note buffer or a running timer; DB migrations are reversible/tested; corrupt input never crashes the app.
- **Onboarding:** a first-run that explains local-first, sets the notes folder, and offers (not forces) the optional permissions.
- **Consistent:** shared design tokens, one component library, consistent spacing/typography, no orphan one-off styles.

---

## 11. The build-in-a-loop protocol (operating manual for Codex)

Build iteratively. **One milestone per loop.** Do not attempt the whole app in one pass.

### 11.1 The loop
```
FOR each milestone M in the roadmap:
  1. PLAN   — restate M's scope + acceptance criteria; list files to touch.
  2. BUILD  — implement Rust services + commands, then frontend, then wire IPC.
  3. TEST   — run the verification harness for M (11.2). Fix until green.
  4. RUN    — `tauri dev`, exercise the happy path + 2 edge cases manually/e2e.
  5. SHOW   — capture a screenshot of each new surface (light + dark).
  6. CRITIQUE — self-review against §10 Definition of Done + §11.3 rubric.
                Log gaps as follow-up tasks. Fix P0/P1 before advancing.
  7. RECORD — update ACCEPTANCE.md (check off, note known gaps) + CHANGELOG.
  8. COMMIT — one milestone = one reviewable commit/PR with a summary.
  ADVANCE only when M's acceptance criteria are all green.
```

### 11.2 Verification harness (must exist and run headless)
- **Rust unit + integration tests** (`cargo test`) for every service: timer math, idle reconciliation, PDF merge/split/redaction, OCR on a fixture image, media trim on a fixture clip, export renders a valid PDF, DB migrations up/down.
- **Golden/fixture tests:** `tests/fixtures/` holds `sample-text.pdf`, `sample-scan.png`, `clip.mp4`, `photo.heic`. Assertions: redaction removes text from the extracted layer; OCR returns expected substring; trim produces the expected duration (±1 frame); invoice/report export produces a PDF with expected line count/total.
- **E2E smoke** via `tauri-driver` + WebDriver: launch, ⌘K → open each tool, start/stop a timer, create+save a note, run one PDF job, export one document.
- **Lint/format gates:** `cargo clippy -D warnings`, `cargo fmt --check`, `eslint`, `tsc --noEmit`, `prettier --check`.
- **Notarization dry-run** (later milestones): `tauri build` + `codesign --verify` + `spctl -a` pass on the DMG.
- The harness must be runnable via a single command (e.g. `make verify`) and must **exit non-zero on any failure** so the loop can gate on it.

### 11.3 Self-critique rubric (score each milestone 1–5; must be ≥4 to pass)
Functionality complete · Edge cases handled · Error/empty/loading states present · Keyboard + a11y · Visual polish (light+dark) · No data-loss paths · Tests cover the new code · Performance (no jank). Write the scores into ACCEPTANCE.md so progress is auditable.

### 11.4 Guardrails while looping
- **Never regress a passing milestone** — keep the full harness green, not just the new tests.
- **Don't expand scope** — if tempted to add a recorder/timeline/dev-utility, stop (see §1.3, §8).
- **Prefer boring, testable code** over clever code the loop can't verify.
- **If a native feature is blocked** (e.g., window title without Screen Recording), implement the graceful-degradation path rather than faking it, and note the limitation.
- **Keep commits small** so a human can review each milestone.

---

## 12. Milestone roadmap (build order)

Each milestone is a full loop iteration with its own acceptance gate.

- **M0 — Skeleton + registry + palette.** Tauri v2 + React + Tailwind boots; tray icon; **tool registry (§15.1)** with the three-surface shell (§16.1) — command palette (⌥Space/⌘K) + main window (rail/Home/workspace) + menu-bar dropdown — all generated from the registry with 2–3 stub tools; design tokens (§16.6) wired; SQLite connects + first migration; `make verify` runs. *Gate:* app launches, palette opens and fuzzy-finds stubs, a stub tool renders in the workspace with a working Run, DB migration applies, harness green. **Getting the registry + palette right here is what makes every later tool cheap.**
- **M1 — Notes core.** Markdown editor + preview, auto-save to `.md`, folder index, FTS search, tags, daily log, templates. *Gate:* create/edit/search/round-trip notes; external-edit reconciliation test passes.
- **M2 — Time tracking core.** Menu-bar timer, projects/clients, manual+timer entries, reports view. Timestamps UTC; crash-recovery of running timer. *Gate:* timer math + crash-recovery + report-total tests pass.
- **M3 — Notes ⇄ time ⇄ projects links + floating overlay note.** Wiki-links/backlinks, note↔project↔entry linking, always-on-top overlay with hotkey. *Gate:* linking model tests; overlay stays on top without stealing focus.
- **M4 — PDF file tools.** Merge/split/reorder/rotate/delete, compress, PDF↔images, extract text, watermark/page-numbers, redaction (with the "text truly removed" test). Job runner + Output Center + Safety/undo. *Gate:* all PDF fixture tests incl. redaction; undo reverses ops.
- **M5 — OCR + export/document engine.** Tesseract sidecar; scan→searchable PDF; template-based export (Invoice, Timesheet, Project doc, Standup log, Research writeup) via HTML→PDF. *Gate:* OCR fixture substring; each template renders a valid PDF with correct data; Unicode export works.
- **M6 — Media micro-tools.** FFmpeg/FFprobe sidecars; video crop/trim/→GIF/convert/frame-grab; image crop/resize/convert/compress; screenshot capture+markup. *Gate:* trim duration + convert + gif-cap tests; capture attaches to note.
- **M7 — Connected workflows.** Named pipelines from §7 as one-click actions + a small Workflows engine; context switcher; active-window app-linked tagging with permission flow. *Gate:* each workflow runs end-to-end and degrades gracefully on missing input.
- **M8 — Polish pass.** Full §10 Definition of Done sweep: states, a11y, dark mode, onboarding, empty states, performance, reduced-motion. *Gate:* every §11.3 rubric dimension ≥4 across all surfaces.
- **M9 — Package & notarize.** Sign app + sidecars, hardened runtime, notarize, staple, DMG; `spctl` + `codesign --verify` pass; first-run on a clean machine opens without Gatekeeper warnings. *Gate:* notarized DMG opens clean; sidecars run post-install.

---

## 13. Open items for the human (flag, don't block)
- **FFmpeg license** (LGPL vs GPL build) and bundling terms — confirm before shipping. Not legal advice.
- **Tesseract language packs** to bundle vs. download-on-demand (download would violate strict-offline; default to bundling English).
- **Apple Developer ID** cert + notarization credentials (needed for M9; agent can't obtain these).
- **App name / bundle id / icon** — `Tooldeck` is a placeholder.
- **Optional Vision-via-objc2 OCR path** — build only if Tesseract quality proves insufficient; keep behind a feature flag so the loop stays Swift-free.

---

## 14. Summary of decisions (so the agent doesn't re-litigate)
- **Stack:** Tauri v2 + Rust + SQLite + React/TS. No Swift.
- **Data:** strictly local (files + SQLite), no accounts, no cloud, offline-only.
- **Distribution:** signed + notarized DMG, not sandboxed, Apple Silicon / macOS 14+.
- **Scope IN:** notes, time tracking, PDF/OCR, document/export engine, minimal media micro-tools, connected workflows.
- **Scope OUT:** screen recording, timeline video editing, audio studio, dev-utility zoo, cloud/Toggl sync.
- **Method:** build in a loop, one milestone at a time, gated by a headless verification harness + a self-critique rubric, with invoicing treated as *one* document output among reports, docs, logs, and research writeups.
- **Breadth:** narrow deep core + wide shallow utility shelf (§15), grown via a tool registry; kept minimal by a command-palette-first UI (§16).

---

## 15. Exhaustive tool catalog (tiered)

This is the full shelf, synthesized from **Foldout's 113-tool catalog** plus category leaders — Raycast (launcher UX), Timing/Rize/Toggl/Tyme (time), PDF Expert/PDFgear/Skim (PDF), DevUtils/DevToys/Boop (dev-data), Permute/Gifski/ImageOptim/CleanShot (media/image). It is intentionally exhaustive; the **tier tags keep it buildable**.

**Tier legend**
- **`C` = Core v1** — ships in the first release (the three pillars + the utilities the workflows depend on).
- **`1` = Fast-follow (v1.x)** — cheap to add on the same engines (ffmpeg / image crate / lopdf / text), high frequency.
- **`2` = Later** — nice-to-have, or needs a new dependency; add once core is polished.
- **`✕` = Excluded** — permanent non-goal (see §1.3), listed so Codex doesn't "helpfully" add it.

### 15.1 The tool-registry pattern (how breadth stays cheap)
Every tool is a **registry entry**, not bespoke plumbing. A tool = `{ id, name, category, icon, keywords, defaults, run(input)->Job|Result, ui: FormSchema }`. The command palette, the home grid, and the category rail are all **generated from the registry**. Adding a tool = add one Rust handler + one registry entry + (optionally) a custom form; it appears everywhere automatically and inherits the shared job/undo/output plumbing. Most simple tools need only a declarative **FormSchema** (fields → run), no custom React. This is what lets the shelf reach Foldout-scale without Foldout-scale UI code.

### 15.2 Notes & Knowledge
| Tier | Tool | Notes |
|---|---|---|
| C | Markdown editor + live preview | CodeMirror 6, auto-save |
| C | Floating overlay note | always-on-top, hotkey capture |
| C | Daily log | per-day `.md`, jump-to-today |
| C | Wiki-links + backlinks | knowledge graph glue |
| C | Full-text search (FTS5) | across the vault |
| C | Tags + tag index | `#tag` |
| C | Note templates | meeting/brief/report/invoice-lines |
| 1 | Note → project/client/entry linking | powers workflows §7 |
| 1 | Quick-capture inbox | dump now, file later |
| 2 | Kanban / task board view over checkboxes | lightweight |
| 2 | Snippet / boilerplate library | reusable text |

### 15.3 Time Tracking
| Tier | Tool | Notes |
|---|---|---|
| C | Menu-bar timer (start/stop/switch) | one-click + hotkey |
| C | Projects / clients / tasks | rates, billable, colors |
| C | Manual + timer entries (edit/split/merge) | |
| C | Reports (day/week/project/client/billable) | hours + amounts |
| C | Idle detection + keep/discard/reassign | |
| 1 | App-linked auto-tagging | active-window sampling (§9.1) |
| 1 | Automatic timeline mode | Timing/Rize-style, review+assign |
| 1 | Pomodoro / focus timer | still logs real time |
| 2 | Weekly capacity / goals | "35 billable hrs/wk" |
| 2 | Calendar import (local .ics) | turn events into entries |

### 15.4 PDF
| Tier | Tool | Notes |
|---|---|---|
| C | Merge / split / reorder | thumbnail grid |
| C | Rotate / delete pages | |
| C | Compress PDF | downsample images |
| C | Extract text | text-layer PDFs |
| C | PDF → images / images → PDF | |
| C | Redaction (flattened, true removal) | tested (§6.4) |
| 1 | Page numbers / Bates stamping | |
| 1 | Text watermark | |
| 1 | Password add/remove | |
| 1 | Fill & sign (lite) | text + signature image, flatten |
| 2 | PDF → Word/other office formats | needs converter dep |
| 2 | Compare two PDFs (diff) | |
| 2 | Bookmarks / TOC editor | |

### 15.5 OCR & Capture
| Tier | Tool | Notes |
|---|---|---|
| C | OCR image/scan → markdown | Tesseract sidecar |
| C | Scanned PDF → searchable PDF | OCR text layer |
| C | Screen capture (region/window/full) | **no recording** |
| C | Screenshot markup (arrow/text/box/blur/redact) | |
| 1 | Scrolling / full-page capture | |
| 1 | Screenshot beautifier (padding/backdrop/frame) | tasteful, minimal |
| 1 | QR generator (link/wifi/contact/text) | |
| 1 | QR / barcode reader (from image or screen) | |
| 2 | Color picker (screen eyedropper) | |
| 2 | Screen ruler / pixel measure | |
| ✕ | Screen recording / cinematic recorder | dedicated app |

### 15.6 Images & SVG
| Tier | Tool | Notes |
|---|---|---|
| C | Crop / resize / rotate / flip | `image` crate |
| C | Convert (PNG/JPG/HEIC/WebP/TIFF) | |
| C | Compress / optimize (strip metadata) | ImageOptim-style |
| 1 | Batch process (folder in → folder out) | leaves originals |
| 1 | Watermark image (text/diagonal) | |
| 1 | Social-media resizer (IG/X/YT presets) | |
| 1 | App icon / favicon generator | one image → set |
| 2 | Background remover | needs model; heavier |
| 2 | Face blur / pixelate (local detect) | |
| 2 | Raster → SVG trace / SVG optimize | |
| 2 | Color palette extract (hex/CSS) | |
| 2 | Image grid / contact sheet | |

### 15.7 Video & Audio (micro only)
| Tier | Tool | Notes |
|---|---|---|
| C | Video trim (range) | ffmpeg sidecar |
| C | Video crop (region) | |
| C | Video → GIF (capped dims/fps) | |
| 1 | Convert / compress / remux | |
| 1 | Extract frame → PNG | |
| 1 | Merge clips end-to-end | simple concat, no timeline |
| 2 | Video → audio extract, audio trim/normalize | trivial ffmpeg |
| 2 | Subtitle SRT⇄VTT / time-shift | text op |
| ✕ | Timeline editor / titles / intro-outro | overkill |
| ✕ | Audio production studio | dedicated app |

### 15.8 Text & Data / Developer
| Tier | Tool | Notes |
|---|---|---|
| 1 | Markdown → HTML export | reuses editor |
| 1 | Text diff | two-pane |
| 1 | Line tools (sort/dedupe/trim/case) | |
| 1 | JSON format/minify/tree | |
| 1 | Base64 / URL / HTML-entity encode-decode | |
| 1 | Hash / checksum (MD5/SHA) | |
| 2 | Regex tester | |
| 2 | CSV/TSV/JSON/YAML converter | |
| 2 | Timestamp / number-base / case converter | |
| 2 | UUID/ULID, JWT decode, cron explainer | dev-data zoo (opt-in) |
| 2 | Color converter (HEX/RGB/HSL) | |
| 2 | Lorem ipsum / password generator | |

> Dev-data tools are **Tier 2 and collapsible** — valuable but not central to Tooldeck's identity. Keep them behind a "Developer" category so they don't dominate the palette for non-dev users (see §16.4 profiles).

### 15.9 Files, Folders & Archives
| Tier | Tool | Notes |
|---|---|---|
| 1 | Bulk rename (preview + apply, undo) | Rename Studio-style |
| 1 | Organize files (by type/date, conflict preview) | |
| 1 | File inspector (metadata/hash/quick actions) | |
| 2 | Duplicate finder (exact + similar) | heavier |
| 2 | Disk-space treemap / folder inventory | |
| 2 | Metadata review/strip (EXIF/GPS) | privacy |
| 2 | Create / extract / inspect archive (zip/tar/gz) | |
| 2 | Split / join large file | |

### 15.10 Create & Export (documents)
| Tier | Tool | Notes |
|---|---|---|
| C | Document/export engine (template + data) | §6.3 |
| C | Invoice template | one output, not the point |
| C | Timesheet / activity report | |
| C | Project doc / spec (styled note → PDF) | documentation-first |
| C | Standup / meeting log export | |
| C | Research writeup (note + OCR + images) | |
| 2 | Custom template editor (user-authored) | |

### 15.11 Mac workflow / menu bar
| Tier | Tool | Notes |
|---|---|---|
| C | Command palette / launcher | the spine (§16) |
| C | Global shortcuts (rebindable) | |
| C | Tray timer + quick actions | |
| C | Output Center (jobs) | |
| C | Safety Center (undo ledger) | |
| 1 | Clipboard history | privacy-aware |
| 1 | Watch folders (input → tool → output) | automation |
| 1 | Saved workflows / recipes | multi-step (§7) |
| 2 | Window manager (snap halves/quarters) | |
| 2 | Quick Actions (select text anywhere → transform) | |

**Rule for the loop:** implement `C` end-to-end and polished before touching `1`; never start a `2` while any `C`/`1` fails its gate. Adding a `1`/`2` tool must not require touching another tool (registry pattern).

---

## 16. UI / UX design system — "many tools, minimal surface"

**North star:** the app should feel like **Raycast married to a focused workspace** — press a key, type what you want, act, get out. Information-dense where it counts, generous whitespace everywhere else, native and trustworthy. The measure of success: a first-time user finds any tool in **≤2 seconds and ≤5 keystrokes**, and a power user rarely touches the mouse.

### 16.1 Three surfaces, nothing more
1. **Command palette (the spine).** Global hotkey (default ⌥Space) → floating, centered, Spotlight-like panel. **Search OR describe** ("resize image", "trim video to 8s", "new invoice", "start timer on ClientB"). Fuzzy-matches tools, notes, projects, recent files, and *actions*. Results are dense rows: icon · name · category · shortcut hint. Enter runs / opens. This is how breadth stays invisible — 100 tools, one search box.
2. **Menu-bar dropdown.** Tray icon shows timer state. Click → compact panel: running timer + start/stop/switch, today's total, 3–4 pinned quick actions, "Open Tooldeck," "New note." Nothing else. Glanceable.
3. **Main window.** A single window with three zones (below). Opens to **Home**; every tool renders in the **workspace** zone. No modals-over-modals, no nested windows (except the floating note + palette, which are their own lightweight windows).

### 16.2 Main window layout
```
┌──────────────────────────────────────────────────────────────┐
│  ⧉  Tooldeck            ⌘K Search…                    �())  ⚙︎  │  ← slim top bar: traffic-lights inset, global search, timer pill, settings
├───────────┬──────────────────────────────────────────────────┤
│  RAIL     │   WORKSPACE                                        │
│           │                                                    │
│  ★ Home   │   ┌── tool header: name + 1-line purpose ──┐       │
│  ◷ Time   │   │  primary input (drop zone / editor)     │      │
│  ✎ Notes  │   │                                         │      │
│  ▤ PDF    │   │  [ the ONE or TWO settings that matter ]│      │
│  ⧉ Images │   │                                         │      │
│  ▶ Media  │   │  ▸ Advanced (collapsed by default)      │      │
│  ⛭ Files  │   │                                         │      │
│  ‹/›Dev   │   └── sticky action bar:  [ Run ]  ⌘↵ ──────┘       │
│           │                                                    │
│  ───────  │                                                    │
│  Recents  │                                                    │
│  Favorites│                                                    │
└───────────┴──────────────────────────────────────────────────┘
```
- **Left rail = categories**, collapsible to icons-only (⌘\). It's a filter, not a deep tree — one level, no accordions.
- **Home** = a calm grid of **Favorites + Recents + "Jump back in"** (last note, running/last timer, recent exports), *not* a wall of 100 icons. The wall lives in the palette and in per-category grids, reached on demand.
- **Workspace** = exactly one tool at a time, with **one clear primary action** (sticky "Run", ⌘↵). Everything non-essential is under a collapsed **▸ Advanced** disclosure.

### 16.3 Minimalism rules (enforce in the loop)
- **One primary action per screen.** If a tool needs two, one is secondary/ghost styled.
- **Progressive disclosure.** Defaults visible; advanced options collapsed. A user should be able to run any tool without opening Advanced.
- **Defaults pre-filled** (Foldout-style): formats, dimensions, output folder carry sensible values so most jobs are drop → Run.
- **No chrome noise.** Monochrome UI + a single accent color. Icons are line-weight, consistent set (Lucide). No gradients, no drop-shadow soup.
- **Whitespace over borders.** Group by spacing, not boxes. Dense only in results lists and reports.
- **Everything reachable by keyboard.** ⌘K palette, ⌘1–9 categories, ⌘↵ run, ⌘F search-in-tool, Esc backs out one level. Show shortcut hints inline.
- **Native texture.** Correct traffic-light insets, sidebar vibrancy, system font (SF via `-apple-system`), momentum scroll, no web scrollbars, respects reduced-motion + system light/dark.

### 16.4 Handling breadth without clutter
- **Profiles / "surfaces."** On first run, ask the persona (Freelancer / Designer / Writer / Developer). This just sets which categories are pinned and which stay collapsed (e.g., Dev tools hidden for a Writer). Everything is still searchable; nothing is deleted — the palette always finds it.
- **Favorites + Recents** float the ~10 tools a given user actually uses to Home and the top of the palette; the long tail stays one search away.
- **"Describe what you need" routing.** The palette maps natural phrases to tools (like Foldout's command bar) — but it only *routes and pre-fills*, it never generates content or acts without an explicit Run.
- **Category grids** are the browse path for discovery; the palette is the speed path. Both are generated from the registry (§15.1).

### 16.5 Other surfaces
- **Floating note:** frameless, translucent, resizable, opacity slider, pin toggle; ≤ one toolbar row. Summoned/dismissed by hotkey.
- **Output Center:** a slide-in panel (not a separate window) listing jobs with progress, cancel, reveal-in-Finder, and errors. Non-blocking — you keep working while jobs run.
- **Safety Center:** a list of recent destructive ops with one-click **Undo**; reachable from the top bar and the palette.
- **Onboarding:** 3 quiet steps — pick persona, choose notes folder, optionally enable permissions (§9). Skippable, never a wall.

### 16.6 Design tokens (starting point)
- **Spacing:** 4-pt base (4/8/12/16/24/32). **Radius:** 8 (cards), 6 (inputs), 10 (windows).
- **Type:** SF/`-apple-system`; 13px body, 11px meta, 15–17px headers; one weight step for hierarchy.
- **Color:** neutral gray scale + **one accent** (user-selectable, default a calm blue/indigo); semantic success/warn/danger only where needed. Full dark-mode token parity.
- **Motion:** 120–180ms ease for enters, none for reduced-motion; palette/overlays fade+scale subtly, never bounce.
- **Density:** comfortable default; a compact toggle for reports/lists.

### 16.7 Accessibility (part of Definition of Done, §10)
Full keyboard operability, visible focus rings, ARIA/roles on custom controls, VoiceOver labels on primary flows, adequate hit targets (≥ 28–32px), AA contrast in both themes, reduced-motion honored, no color-only signaling.

### 16.8 UI acceptance gate (add to the loop)
A tool/screen isn't "done" until: reachable from palette **and** its category grid; has a single obvious primary action; runs on defaults with Advanced collapsed; has designed empty/loading/error/success states; works fully by keyboard; passes light+dark; and matches the token set. Score against §11.3 every milestone.

# Changelog

## 1.0.0 — M9 release engineering

- Added a release-only Tauri overlay for eight target-triple sidecars while
  keeping development independent of a shell PATH.
- Added reviewed sidecar intake with arm64 Mach-O, executable, self-contained
  linkage, FFmpeg GPL-policy, and SHA-256 checks.
- Added per-sidecar hardened-runtime signing, app signing, strict nested
  signature verification, and installed-bundle execution smoke tests.
- Added deterministic DMG creation/signing plus `notarytool --wait`, stapling,
  validation, and Gatekeeper assessment.
- Added a credential-free release audit that reports each missing human input
  without leaking or inventing credentials.
- Added the complete release and clean-machine runbook plus in-app/repository
  FFmpeg, Poppler, and Apple Vision OCR notices.
- Promoted the app, Rust crate, and package metadata to version 1.0.0.

## 0.7.0 — M8 onboarding, privacy, and polish

- Added a skippable three-step first run for profile choice, writable Markdown
  vault selection, and optional macOS permissions.
- Added profile-aware category pins and Home favorites for Freelancer,
  Designer, Writer, and Developer surfaces while preserving global search.
- Added a complete Settings workspace with theme, density, notes vault,
  privacy matrix, local data location, and setup replay.
- Added runtime notes-vault switching with write validation, persisted restart
  behavior, source-folder preservation, and derived-index rebuilding.
- Made Markdown reconciliation incremental for unchanged files and stale-index
  cleanup after external deletion.
- Added associated form labels, combobox semantics, functional timer/settings
  controls, Escape navigation, compact density, and responsive onboarding.
- Verified reduced motion, keyboard focus, control hit targets, light/dark
  parity, onboarding, settings, and privacy degradation states.

## 0.6.0 — M7 connected workflows

- Added a typed workflow planner with explicit job, next-tool, completed, and
  recoverable-degradation outcomes.
- Added one-click Billable → Document, Documentation Companion, Research
  Companion, Context Switcher, Meeting / Standup Log, Capture → Annotate →
  Attach, and Media Micro-task Inline workflows.
- Added idempotently installed workflow definitions backed by local SQLite.
- Added note-prefilled capture handoff and post-job note attachment for media
  exports.
- Added low-frequency foreground-app sampling linked to the running time entry.
- Added permission-aware window-title capture, app-name-only degradation, and a
  user-triggered Screen Recording access flow.
- Added populated and missing-input workflow fixtures plus light/dark browser
  acceptance for the workflow category and handoff states.

## 0.5.0 — M6 focused media and capture tools

- Added precise video trim/crop, MP4/MOV/WebM conversion, capped GIF export,
  and PNG frame extraction through cancellable FFmpeg jobs.
- Added image crop, resize, PNG/JPEG/HEIC conversion, and JPEG quality
  compression through the same safe export path.
- Added interactive macOS region capture.
- Added offline bitmap screenshot markup with arrow, outline box, text,
  pixelation, and black-redaction modes.
- Added optional capture-to-note attachment with Markdown link insertion and
  vault reindexing.
- Added adjacent-binary/Homebrew/system resolution for local media and PDF
  engines in development and packaged layouts.
- Added real FFmpeg/FFprobe, ImageIO, bitmap markup, and note-attachment
  fixtures.

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
